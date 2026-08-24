//! Orchestrates a single scrcpy mirror session: push the vendored server
//! jar, `adb forward` a local port to the device's abstract socket, launch
//! the on-device server (video + control, no audio), connect both channels,
//! and read the resulting frame stream / write touch-and-key input.
//!
//! Launch command verified directly against scrcpy v4.1 source
//! (`app/src/server.c::execute_server`, `server/.../Options.java` for
//! defaults): `tunnel_forward` defaults to `false` (must be passed
//! explicitly for forward-mode); `video`/`audio`/`control` default to `true`
//! (only `audio`/`control` need to be turned off); `scid` defaults to `-1`
//! ("none"), which makes `DesktopConnection` use the bare `"scrcpy"` socket
//! name — fine here since v1 never runs more than one session at a time.

use std::{
    io,
    path::PathBuf,
    process::{Command as StdCommand, Stdio},
    time::Duration,
};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    process::{Child, Command},
    time::{sleep, timeout},
};

use super::{
    control_socket::{self, KeyAction, TouchAction},
    device,
    device::DeviceError,
    protocol, vendor,
};

const CONNECT_ATTEMPTS: u32 = 30;
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(100);
/// Attempt budget for `connect_and_handshake_with_retry` specifically — kept
/// small (see `HANDSHAKE_ATTEMPT_TIMEOUT` below for why a *retry* here is
/// actually costly, not just slow).
const HANDSHAKE_ATTEMPTS: u32 = 2;
/// Per-attempt cap on `connect_and_handshake_with_retry`'s inner `await`.
///
/// Generous on purpose, and increasing this number is the *safe* direction
/// if it's ever still too short: the on-device server writes the handshake
/// in two separate syscalls — one dummy byte right on accept
/// (`DesktopConnection.open`), then the real device-name+codec payload only
/// once `Server.scrcpy()` gets there, which can be well past encoder/display
/// setup. Confirmed live (`adb shell` stdout capture + a manual socket
/// probe) that abandoning a connection *after* the dummy byte but before
/// that second write — which is exactly what timing out and dropping the
/// `TcpStream` does — makes the server's later write hit `EPIPE`
/// (`DesktopConnection.sendDeviceMeta`), which is *uncaught* and kills the
/// whole `app_process`, not just that one connection. Every subsequent
/// retry then fails instantly with "early eof" (nothing is listening
/// anymore) no matter how many attempts are left — so a too-short timeout
/// here doesn't just fail this attempt, it dooms the entire session, and a
/// short retry loop makes that worse, not better. A too-long timeout only
/// costs wall-clock time on a rare truly-dead server.
const HANDSHAKE_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(25);

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error(transparent)]
    Device(#[from] DeviceError),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("failed to push scrcpy-server to the device: {0}")]
    PushFailed(String),
    #[error("failed to set up adb forward: {0}")]
    ForwardFailed(String),
    #[error("failed to launch scrcpy-server on the device")]
    ServerLaunchFailed,
    #[error("could not connect to the on-device server after {0} attempts")]
    ConnectTimeout(u32),
    #[error("unexpected codec id 0x{0:08x} (only h264 is supported)")]
    UnsupportedCodec(i32),
}

/// Encoder tuning passed straight through as scrcpy server args. `None` on
/// any field keeps scrcpy's own default (native resolution, 8 Mbps,
/// uncapped fps) — see `Options.java` defaults in the v4.1 source.
#[derive(Debug, Clone, Copy, Default)]
pub struct EncoderOptions {
    /// Longest side, in pixels (`max_size`). `None`/`Some(0)` = native.
    pub max_size: Option<u32>,
    /// Bitrate in bits/sec (`video_bit_rate`). Caller converts from a
    /// human-friendly kbps UI value.
    pub bit_rate: Option<u32>,
    /// Capped frame rate (`max_fps`). `None`/`Some(0)` = uncapped.
    pub max_fps: Option<u32>,
}

pub struct MirrorSession {
    stream: TcpStream,
    // A second connection to the same forwarded port, accepted by the
    // device as the *control* channel (see `open()` in
    // `DesktopConnection.java`: accept() calls happen in a fixed
    // video/audio/control order gated on which are enabled — with audio
    // off, this is simply the next TCP connection after the video one, no
    // handshake bytes of its own). Used to inject touch/key/text events;
    // never read from.
    control_stream: TcpStream,
    pub device_name: String,
    pub codec_id: i32,
    // Held only for its Drop impl (tears down the adb forward + kills the
    // spawned on-device server process). Never read otherwise.
    _cleanup: SessionCleanup,
}

impl MirrorSession {
    /// Read the next raw wire packet (12-byte header, plus its payload for a
    /// frame packet) — passed through byte-for-byte to the browser, see
    /// `protocol` module docs.
    pub async fn read_packet(&mut self) -> io::Result<Vec<u8>> {
        read_packet_raw(&mut self.stream).await
    }

    pub async fn send_touch(
        &mut self,
        action: TouchAction,
        x: i32,
        y: i32,
        screen_width: u16,
        screen_height: u16,
    ) -> io::Result<()> {
        let msg = control_socket::encode_touch_event(action, x, y, screen_width, screen_height);
        self.control_stream.write_all(&msg).await
    }

    pub async fn send_key(
        &mut self,
        action: KeyAction,
        keycode: i32,
        meta_state: i32,
    ) -> io::Result<()> {
        let msg = control_socket::encode_key_event(action, keycode, meta_state);
        self.control_stream.write_all(&msg).await
    }

    pub async fn send_text(&mut self, text: &str) -> io::Result<()> {
        let msg = control_socket::encode_text_event(text);
        self.control_stream.write_all(&msg).await
    }
}

struct SessionCleanup {
    adb_path: PathBuf,
    serial: String,
    local_port: u16,
    server_child: Child,
}

impl Drop for SessionCleanup {
    fn drop(&mut self) {
        // Best-effort only: this runs on WS-drop / session end, not a hot
        // path, and Drop can't be async — a short blocking subprocess call
        // is fine here. `start_kill` is a sync tokio::Child method (no
        // runtime needed).
        let _ = self.server_child.start_kill();
        let _ = StdCommand::new(&self.adb_path)
            .args([
                "-s",
                &self.serial,
                "forward",
                "--remove",
                &format!("tcp:{}", self.local_port),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// Read one raw wire packet from any async byte stream — split out from
/// `MirrorSession` so it can be exercised in tests against a fake in-memory
/// or fake-TCP-server stream, without adb or a real device.
pub async fn read_packet_raw<R: AsyncRead + Unpin>(stream: &mut R) -> io::Result<Vec<u8>> {
    let mut header = [0u8; protocol::HEADER_LENGTH];
    stream.read_exact(&mut header).await?;
    let packet = protocol::parse_header(&header);

    let mut buf = header.to_vec();
    if let protocol::Packet::Frame { payload_size, .. } = packet {
        let mut payload = vec![0u8; payload_size as usize];
        stream.read_exact(&mut payload).await?;
        buf.extend_from_slice(&payload);
    }
    Ok(buf)
}

/// Read the video-socket handshake: one dummy byte, the 64-byte device name,
/// then the 4-byte codec id. Errors if the codec isn't h264 (the only one
/// this feature's browser-side WebCodecs decoder supports).
async fn read_handshake<R: AsyncRead + Unpin>(
    stream: &mut R,
) -> Result<(String, i32), ClientError> {
    let mut dummy = [0u8; 1];
    stream.read_exact(&mut dummy).await?;

    let mut name_buf = [0u8; protocol::DEVICE_NAME_FIELD_LENGTH];
    stream.read_exact(&mut name_buf).await?;
    let nul_pos = name_buf
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(name_buf.len());
    let device_name = String::from_utf8_lossy(&name_buf[..nul_pos]).into_owned();

    let mut codec_buf = [0u8; 4];
    stream.read_exact(&mut codec_buf).await?;
    let codec_id = protocol::decode_codec_id(codec_buf);
    if codec_id != protocol::CODEC_ID_H264 {
        return Err(ClientError::UnsupportedCodec(codec_id));
    }

    Ok((device_name, codec_id))
}

/// Connect AND read the handshake, retrying the whole thing together on
/// failure — not just the raw TCP connect. `adb forward` accepts the local
/// TCP connection immediately regardless of whether anything is listening on
/// the device's abstract socket yet; if the on-device server isn't ready,
/// adb tears the connection down right after accepting it, which surfaces
/// here as an EOF partway through (or at the very start of) the handshake
/// read, not as a failed `connect()`. scrcpy's own client has the exact same
/// race and retries the same way (`connect_to_server`/`connect_and_read_byte`
/// in `app/src/server.c`) — a `connect_with_retry` that only retried the bare
/// TCP connect would "succeed" once, then die on the very first
/// `read_handshake` byte.
async fn connect_and_handshake_with_retry(
    port: u16,
) -> Result<(TcpStream, String, i32), ClientError> {
    let mut last_err = None;
    for attempt in 0..HANDSHAKE_ATTEMPTS {
        let attempt_result = timeout(HANDSHAKE_ATTEMPT_TIMEOUT, async {
            let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
            let (device_name, codec_id) = read_handshake(&mut stream).await?;
            Ok::<_, ClientError>((stream, device_name, codec_id))
        })
        .await
        .unwrap_or_else(|_elapsed| Err(io::Error::from(io::ErrorKind::TimedOut).into()));

        match attempt_result {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_err = Some(e);
                if attempt + 1 < HANDSHAKE_ATTEMPTS {
                    sleep(CONNECT_RETRY_DELAY).await;
                }
            }
        }
    }
    Err(last_err.unwrap_or(ClientError::ConnectTimeout(HANDSHAKE_ATTEMPTS)))
}

/// Deploy and connect to a view-only scrcpy mirror session.
///
/// `device_serial`: `Some(serial)` pins an exact device; `None` requires
/// exactly one connected device (see `device::select_device`).
pub async fn connect(
    device_serial: Option<&str>,
    encoder: EncoderOptions,
) -> Result<MirrorSession, ClientError> {
    let adb_path = device::resolve_adb().await?;
    let devices = device::list_devices(&adb_path).await?;
    let device = device::select_device(&devices, device_serial)?;
    let serial = device.serial.clone();

    // Every session binds the same bare `scrcpy` abstract socket (see the
    // module doc comment on `scid` defaulting to "none") — a prior server
    // process that didn't exit cleanly (e.g. a browser tab closed without a
    // clean WS close, so `SessionCleanup::drop` never ran) is still squatting
    // that socket and would silently eat this session's connection instead
    // (confirmed live: a stale process left the *next* connect attempt
    // failing with an early-EOF handshake read, indistinguishable from the
    // "server not ready yet" race `connect_and_handshake_with_retry` already
    // handles — except retrying never helps here, since the stale process
    // never accepts a second client). Best-effort: nothing to clean up on a
    // fresh device, and a failure here shouldn't block starting the new one.
    let pkill_output = Command::new(&adb_path)
        .args([
            "-s",
            &serial,
            "shell",
            "pkill",
            "-f",
            "com.genymobile.scrcpy.Server",
        ])
        .output()
        .await;
    // `pkill` returning success only means the signal was *sent* — a JVM
    // doesn't release its `LocalServerSocket` bind synchronously with that,
    // so proceeding immediately races the new server's own bind attempt
    // against the old one's teardown (confirmed live: "Address already in
    // use" from `LocalServerSocket.<init>`, which then leaves the *new*
    // launch's connections silently accepted by the dying old process
    // instead — no bind error surfaces to us since we don't capture the
    // launched server's stderr in the non-debug path). Poll for the process
    // to actually disappear instead of a fixed sleep, since JVM teardown
    // time varies with system load; give up and proceed anyway past the
    // deadline (best-effort, matching the kill itself).
    if matches!(&pkill_output, Ok(o) if o.status.success()) {
        for _ in 0..10 {
            let still_alive = Command::new(&adb_path)
                .args([
                    "-s",
                    &serial,
                    "shell",
                    "pgrep",
                    "-f",
                    "com.genymobile.scrcpy.Server",
                ])
                .output()
                .await
                .map(|o| !o.stdout.is_empty())
                .unwrap_or(false);
            if !still_alive {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    }

    // 0. Wake the screen before capturing: scrcpy only encodes frames the
    // display compositor actually produces, and a dozing/locked screen
    // produces none — the on-device server accepts the connection and sends
    // the session (resolution) packet fine, then goes silent forever with no
    // error at all (confirmed live: `dumpsys power` showed `mWakefulness=
    // Dozing` during exactly this symptom). Best-effort: a failure here
    // (e.g. `input` requires the screen-off keyguard to allow it, which it
    // normally does) shouldn't block the rest of the connect sequence.
    // `stay_awake=true` below is the complementary in-session safeguard,
    // though scrcpy's own semantics for it are strongest while charging.
    let _ = Command::new(&adb_path)
        .args(["-s", &serial, "shell", "input", "keyevent", "224"]) // KEYCODE_WAKEUP
        .output()
        .await;

    // 1. Push the vendored jar (write to a temp file first — adb push needs
    // a path, we only have the bytes embedded in the binary).
    let tmp_jar = tempfile::NamedTempFile::new()?;
    tokio::fs::write(tmp_jar.path(), vendor::SCRCPY_SERVER_JAR).await?;
    let push_output = Command::new(&adb_path)
        .args([
            "-s",
            &serial,
            "push",
            &tmp_jar.path().to_string_lossy(),
            vendor::SCRCPY_DEVICE_JAR_PATH,
        ])
        .output()
        .await?;
    if !push_output.status.success() {
        return Err(ClientError::PushFailed(
            String::from_utf8_lossy(&push_output.stderr).into_owned(),
        ));
    }

    // 2. Reserve a free local port (bind-then-drop — `adb forward` binds it
    // for real immediately after; the tiny window is standard practice and
    // acceptable for a single-user local dev tool).
    let probe = TcpListener::bind(("127.0.0.1", 0)).await?;
    let local_port = probe.local_addr()?.port();
    drop(probe);

    let forward_output = Command::new(&adb_path)
        .args([
            "-s",
            &serial,
            "forward",
            &format!("tcp:{local_port}"),
            &format!("localabstract:{}", vendor::SCRCPY_SOCKET_NAME),
        ])
        .output()
        .await?;
    if !forward_output.status.success() {
        return Err(ClientError::ForwardFailed(
            String::from_utf8_lossy(&forward_output.stderr).into_owned(),
        ));
    }

    // 3. Launch the on-device server: video only (default), audio and
    // control explicitly disabled, forward-mode tunnel, quiet logging.
    // `stay_awake=true` keeps the screen from re-dozing mid-session (the
    // keyevent above only handles the awake-before-connecting part).
    let mut server_args = vec![
        "-s".to_string(),
        serial.clone(),
        "shell".to_string(),
        format!("CLASSPATH={}", vendor::SCRCPY_DEVICE_JAR_PATH),
        "app_process".to_string(),
        "/".to_string(),
        "com.genymobile.scrcpy.Server".to_string(),
        vendor::SCRCPY_SERVER_VERSION.to_string(),
        "audio=false".to_string(),
        "control=true".to_string(),
        "tunnel_forward=true".to_string(),
        "log_level=error".to_string(),
        "stay_awake=true".to_string(),
    ];
    if let Some(max_size) = encoder.max_size.filter(|&v| v > 0) {
        server_args.push(format!("max_size={max_size}"));
    }
    if let Some(bit_rate) = encoder.bit_rate.filter(|&v| v > 0) {
        server_args.push(format!("video_bit_rate={bit_rate}"));
    }
    if let Some(max_fps) = encoder.max_fps.filter(|&v| v > 0) {
        server_args.push(format!("max_fps={max_fps}"));
    }

    let server_child = Command::new(&adb_path)
        .args(&server_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| ClientError::ServerLaunchFailed)?;

    // 4. Connect and handshake, retrying the pair together (see
    // `connect_and_handshake_with_retry` doc comment for why).
    let (stream, device_name, codec_id) = match connect_and_handshake_with_retry(local_port).await {
        Ok(v) => v,
        Err(e) => {
            let _ = StdCommand::new(&adb_path)
                .args([
                    "-s",
                    &serial,
                    "forward",
                    "--remove",
                    &format!("tcp:{local_port}"),
                ])
                .status();
            return Err(e);
        }
    };

    // 5. Connect the control channel: a second TCP connection to the same
    // forwarded port. The device's `DesktopConnection.open()` calls
    // `accept()` once per enabled channel in a fixed video/audio/control
    // order — with audio off, the video connection above already consumed
    // the first `accept()`, so this one lands on the control role. No
    // handshake bytes here (those are video-socket-only); a short retry
    // covers the same "adb forward accepted before the far side is ready"
    // race `connect_and_handshake_with_retry` handles for video, though
    // it's far less likely to matter this late — the server was already
    // running long enough to answer the video handshake.
    let control_stream = match connect_control_with_retry(local_port).await {
        Ok(s) => s,
        Err(e) => {
            let _ = StdCommand::new(&adb_path)
                .args([
                    "-s",
                    &serial,
                    "forward",
                    "--remove",
                    &format!("tcp:{local_port}"),
                ])
                .status();
            return Err(e);
        }
    };

    Ok(MirrorSession {
        stream,
        control_stream,
        device_name,
        codec_id,
        _cleanup: SessionCleanup {
            adb_path,
            serial,
            local_port,
            server_child,
        },
    })
}

async fn connect_control_with_retry(port: u16) -> Result<TcpStream, ClientError> {
    let mut last_err = None;
    for attempt in 0..CONNECT_ATTEMPTS {
        match TcpStream::connect(("127.0.0.1", port)).await {
            Ok(stream) => return Ok(stream),
            Err(e) => {
                last_err = Some(e);
                if attempt + 1 < CONNECT_ATTEMPTS {
                    sleep(CONNECT_RETRY_DELAY).await;
                }
            }
        }
    }
    Err(last_err.unwrap_or(io::ErrorKind::TimedOut.into()).into())
}

#[cfg(test)]
mod tests {
    use tokio::{io::AsyncWriteExt, net::TcpListener};

    use super::*;

    /// Builds a fake device-side stream (in a background task) that speaks
    /// exactly what `DesktopConnection`/`Streamer` would write: dummy byte,
    /// 64-byte device name, 4-byte codec id, a session packet, then two
    /// frame packets — and asserts our client-side reading matches.
    #[tokio::test]
    async fn reads_handshake_and_packets_from_a_fake_server() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();

            // Handshake.
            sock.write_all(&[0]).await.unwrap(); // dummy byte
            let mut name = [0u8; protocol::DEVICE_NAME_FIELD_LENGTH];
            name[..6].copy_from_slice(b"Pixel8");
            sock.write_all(&name).await.unwrap();
            sock.write_all(&protocol::CODEC_ID_H264.to_be_bytes())
                .await
                .unwrap();

            // Session packet: flags=0x80000000, width=1080, height=2400.
            let mut session = Vec::with_capacity(12);
            session.extend_from_slice(&0x8000_0000u32.to_be_bytes());
            session.extend_from_slice(&1080u32.to_be_bytes());
            session.extend_from_slice(&2400u32.to_be_bytes());
            sock.write_all(&session).await.unwrap();

            // Config packet (SPS/PPS), 3-byte fake payload.
            let config_header: u64 = 1 << 62;
            let mut config = Vec::with_capacity(15);
            config.extend_from_slice(&config_header.to_be_bytes());
            config.extend_from_slice(&3u32.to_be_bytes());
            config.extend_from_slice(&[1, 2, 3]);
            sock.write_all(&config).await.unwrap();

            // Key frame, 4-byte fake payload.
            let key_header: u64 = (1 << 61) | 42;
            let mut frame = Vec::with_capacity(16);
            frame.extend_from_slice(&key_header.to_be_bytes());
            frame.extend_from_slice(&4u32.to_be_bytes());
            frame.extend_from_slice(&[9, 9, 9, 9]);
            sock.write_all(&frame).await.unwrap();
        });

        let mut client = TcpStream::connect(addr).await.unwrap();
        let (device_name, codec_id) = read_handshake(&mut client).await.unwrap();
        assert_eq!(device_name, "Pixel8");
        assert_eq!(codec_id, protocol::CODEC_ID_H264);

        let session_raw = read_packet_raw(&mut client).await.unwrap();
        assert_eq!(session_raw.len(), 12);
        assert_eq!(
            protocol::parse_header(session_raw[..12].try_into().unwrap()),
            protocol::Packet::Session {
                width: 1080,
                height: 2400,
                is_client_resize: false,
            }
        );

        let config_raw = read_packet_raw(&mut client).await.unwrap();
        assert_eq!(config_raw.len(), 12 + 3);
        assert_eq!(&config_raw[12..], &[1, 2, 3]);

        let frame_raw = read_packet_raw(&mut client).await.unwrap();
        assert_eq!(frame_raw.len(), 12 + 4);
        assert_eq!(&frame_raw[12..], &[9, 9, 9, 9]);

        server.await.unwrap();
    }

    #[tokio::test]
    async fn rejects_unsupported_codec() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            sock.write_all(&[0]).await.unwrap();
            sock.write_all(&[0u8; protocol::DEVICE_NAME_FIELD_LENGTH])
                .await
                .unwrap();
            // "vp8" codec id, not h264.
            sock.write_all(&0x0076_7038i32.to_be_bytes()).await.unwrap();
        });

        let mut client = TcpStream::connect(addr).await.unwrap();
        let err = read_handshake(&mut client).await.unwrap_err();
        assert!(matches!(err, ClientError::UnsupportedCodec(_)));
    }
}
