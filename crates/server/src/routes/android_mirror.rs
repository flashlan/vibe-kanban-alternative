//! Live Android screen mirror — see `services::services::android_mirror`
//! for the scrcpy deploy/protocol client this route wraps. Two endpoints:
//!
//! - `GET /api/android-mirror/devices` — REST, lists `adb`-visible devices
//!   for the frontend's device picker.
//! - `GET /api/android-mirror/ws?device_serial=<optional>` — WS. Outbound:
//!   raw scrcpy video-socket packets (12-byte header, +payload for frame
//!   packets), byte-for-byte as binary frames. Inbound: JSON touch/key/text
//!   messages (see `AndroidMirrorInboundMessage`), forwarded to the scrcpy
//!   control socket. No `workspace_id` here: which phone is plugged into
//!   this machine has nothing to do with which workspace is open.
//!   Per-workspace device pinning (which serial to request) is a
//!   frontend-only concern.

use axum::{
    Router,
    extract::{
        Query, State,
        ws::{CloseFrame, Message, Utf8Bytes},
    },
    response::{IntoResponse, Json as ResponseJson},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use services::services::android_mirror::{
    client::{self, EncoderOptions, MirrorSession},
    control::{self, NavAction},
    control_socket::{KeyAction, TouchAction},
    device::DeviceError,
    emulator::{self, EmulatorError},
};
use ts_rs::TS;
use utils::response::ApiResponse;

use crate::{
    DeploymentImpl,
    error::ApiError,
    middleware::signed_ws::{MaybeSignedWebSocket, SignedWsUpgrade},
};

#[derive(Debug, Serialize, TS)]
pub struct AndroidMirrorDevice {
    pub serial: String,
    pub state: String,
}

async fn list_devices(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<AndroidMirrorDevice>>>, ApiError> {
    let adb_path = services::services::android_mirror::device::resolve_adb()
        .await
        .map_err(|e: DeviceError| ApiError::BadRequest(e.to_string()))?;
    let devices = services::services::android_mirror::device::list_devices(&adb_path)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .into_iter()
        .map(|d| AndroidMirrorDevice {
            serial: d.serial,
            state: d.state,
        })
        .collect();
    Ok(ResponseJson(ApiResponse::success(devices)))
}

async fn list_avds(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<String>>>, ApiError> {
    let emulator_path = emulator::resolve_emulator()
        .await
        .map_err(|e: EmulatorError| ApiError::BadRequest(e.to_string()))?;
    let avds = emulator::list_avds(&emulator_path)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(ResponseJson(ApiResponse::success(avds)))
}

#[derive(Debug, Deserialize, TS)]
pub struct AndroidMirrorLaunchAvdRequest {
    avd_name: String,
}

async fn launch_avd(
    State(_deployment): State<DeploymentImpl>,
    axum::Json(req): axum::Json<AndroidMirrorLaunchAvdRequest>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let emulator_path = emulator::resolve_emulator()
        .await
        .map_err(|e: EmulatorError| ApiError::BadRequest(e.to_string()))?;
    emulator::launch_avd(&emulator_path, &req.avd_name)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(ResponseJson(ApiResponse::success(())))
}

#[derive(Debug, Deserialize)]
struct AndroidMirrorWsQuery {
    #[serde(default)]
    device_serial: Option<String>,
    /// scrcpy `max_size` (longest side, px). Absent/0 = native resolution.
    #[serde(default)]
    max_size: Option<u32>,
    /// Encoder bitrate in kbps (converted to bps before reaching scrcpy).
    /// Absent/0 = scrcpy's own default (8 Mbps).
    #[serde(default)]
    bit_rate_kbps: Option<u32>,
    /// scrcpy `max_fps`. Absent/0 = uncapped.
    #[serde(default)]
    max_fps: Option<u32>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum NavActionRequest {
    Home,
    Back,
    Recents,
}

impl From<NavActionRequest> for NavAction {
    fn from(a: NavActionRequest) -> Self {
        match a {
            NavActionRequest::Home => NavAction::Home,
            NavActionRequest::Back => NavAction::Back,
            NavActionRequest::Recents => NavAction::Recents,
        }
    }
}

#[derive(Debug, Deserialize, TS)]
pub struct AndroidMirrorNavRequest {
    #[serde(default)]
    device_serial: Option<String>,
    action: NavActionRequest,
}

async fn send_nav_action(
    State(_deployment): State<DeploymentImpl>,
    axum::Json(req): axum::Json<AndroidMirrorNavRequest>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    control::send_nav_action(req.device_serial.as_deref(), req.action.into())
        .await
        .map_err(|e: DeviceError| ApiError::BadRequest(e.to_string()))?;
    Ok(ResponseJson(ApiResponse::success(())))
}

#[derive(Debug, Deserialize, TS)]
pub struct AndroidMirrorForceStopRequest {
    #[serde(default)]
    device_serial: Option<String>,
    package: String,
}

async fn force_stop_app(
    State(_deployment): State<DeploymentImpl>,
    axum::Json(req): axum::Json<AndroidMirrorForceStopRequest>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    control::force_stop(req.device_serial.as_deref(), &req.package)
        .await
        .map_err(|e: DeviceError| ApiError::BadRequest(e.to_string()))?;
    Ok(ResponseJson(ApiResponse::success(())))
}

async fn android_mirror_ws(
    ws: SignedWsUpgrade,
    Query(query): Query<AndroidMirrorWsQuery>,
) -> impl IntoResponse {
    let encoder = EncoderOptions {
        max_size: query.max_size,
        bit_rate: query.bit_rate_kbps.map(|kbps| kbps * 1000),
        max_fps: query.max_fps,
    };
    ws.on_upgrade(move |socket| handle_android_mirror_ws(socket, query.device_serial, encoder))
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AndroidMirrorMessage {
    Ready { device_name: String },
    Error { message: String },
}

/// Inbound WS messages from the browser — touch/key input to inject via the
/// control socket (see `client::MirrorSession::send_touch`/`send_key`/
/// `send_text`). `x`/`y`/`screen_width`/`screen_height` are already in
/// absolute device pixels; the browser computes that mapping since it's the
/// one that knows the canvas's on-screen size vs. the decoded frame's actual
/// resolution.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AndroidMirrorInboundMessage {
    Touch {
        action: TouchActionWire,
        x: i32,
        y: i32,
        screen_width: u16,
        screen_height: u16,
    },
    Key {
        action: KeyActionWire,
        keycode: i32,
        #[serde(default)]
        meta_state: i32,
    },
    Text {
        text: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TouchActionWire {
    Down,
    Up,
    Move,
}

impl From<TouchActionWire> for TouchAction {
    fn from(a: TouchActionWire) -> Self {
        match a {
            TouchActionWire::Down => TouchAction::Down,
            TouchActionWire::Up => TouchAction::Up,
            TouchActionWire::Move => TouchAction::Move,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum KeyActionWire {
    Down,
    Up,
}

impl From<KeyActionWire> for KeyAction {
    fn from(a: KeyActionWire) -> Self {
        match a {
            KeyActionWire::Down => KeyAction::Down,
            KeyActionWire::Up => KeyAction::Up,
        }
    }
}

/// Best-effort: a single malformed/dropped input event shouldn't tear down
/// the whole mirror session the way a video-stream error does.
async fn handle_inbound_control_message(session: &mut MirrorSession, text: &str) {
    let msg: AndroidMirrorInboundMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            tracing::debug!("[android-mirror] bad inbound message: {e}");
            return;
        }
    };
    let result = match msg {
        AndroidMirrorInboundMessage::Touch {
            action,
            x,
            y,
            screen_width,
            screen_height,
        } => {
            session
                .send_touch(action.into(), x, y, screen_width, screen_height)
                .await
        }
        AndroidMirrorInboundMessage::Key {
            action,
            keycode,
            meta_state,
        } => session.send_key(action.into(), keycode, meta_state).await,
        AndroidMirrorInboundMessage::Text { text } => session.send_text(&text).await,
    };
    if let Err(e) = result {
        tracing::debug!("[android-mirror] failed to send control message: {e}");
    }
}

async fn handle_android_mirror_ws(
    mut socket: MaybeSignedWebSocket,
    device_serial: Option<String>,
    encoder: EncoderOptions,
) {
    let mut session = match client::connect(device_serial.as_deref(), encoder).await {
        Ok(session) => session,
        Err(e) => {
            close_with_error(&mut socket, &e.to_string()).await;
            return;
        }
    };

    let ready = AndroidMirrorMessage::Ready {
        device_name: session.device_name.clone(),
    };
    if let Ok(json) = serde_json::to_string(&ready) {
        let _ = socket.send(Message::Text(json.into())).await;
    }

    loop {
        tokio::select! {
            packet = session.read_packet() => {
                match packet {
                    Ok(bytes) => {
                        if socket.send(Message::Binary(bytes.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        close_with_error(&mut socket, &format!("mirror stream ended: {e}")).await;
                        break;
                    }
                }
            }
            inbound = socket.recv() => {
                match inbound {
                    Ok(Some(Message::Close(_))) | Ok(None) | Err(_) => break,
                    Ok(Some(Message::Text(text))) => {
                        handle_inbound_control_message(&mut session, &text).await;
                    }
                    Ok(Some(_)) => {} // no binary/ping inbound messages expected
                }
            }
        }
    }
}

/// Send an error message then close with code 1000 (`wasClean`) — mirrors
/// `terminal.rs`'s `close_with_error`: the frontend only suppresses
/// reconnection on a clean 1000 close, so deploy/connect failures must use
/// this rather than a bare `close()` or a failed HTTP upgrade (which the
/// frontend's WS client would otherwise treat as transient and retry
/// forever against, e.g., a permanently absent device).
async fn close_with_error(socket: &mut MaybeSignedWebSocket, message: &str) {
    let msg = AndroidMirrorMessage::Error {
        message: message.to_string(),
    };
    if let Ok(json) = serde_json::to_string(&msg) {
        let _ = socket.send(Message::Text(json.into())).await;
    }
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code: 1000,
            reason: Utf8Bytes::from_static("mirror session ended"),
        })))
        .await;
}

pub(super) fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/android-mirror/devices", get(list_devices))
        .route("/android-mirror/avds", get(list_avds))
        .route("/android-mirror/avds/launch", post(launch_avd))
        .route("/android-mirror/ws", get(android_mirror_ws))
        .route("/android-mirror/nav", post(send_nav_action))
        .route("/android-mirror/force-stop", post(force_stop_app))
}
