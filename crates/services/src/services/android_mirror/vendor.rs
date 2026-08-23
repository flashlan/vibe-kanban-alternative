//! Vendored `scrcpy-server` binary.
//!
//! Source: <https://github.com/Genymobile/scrcpy/releases/download/v4.1/scrcpy-server-v4.1>
//! License: Apache-2.0 (github.com/Genymobile/scrcpy/blob/master/LICENSE).
//! SHA-256 (verified against the release's published `SHA256SUMS.txt` when
//! vendored): `deacb991ed2509715160ffdc7907e47b4160eb30d1566217e9047fd5b8850cae`
//!
//! The version string below MUST exactly match the release the jar was
//! built from — the on-device server aborts immediately if the command-line
//! version argument doesn't match its own compiled `BuildConfig.VERSION_NAME`
//! (`server/src/main/java/com/genymobile/scrcpy/Server.java`). scrcpy ships
//! the client and server jar in lockstep per release tag, so the version is
//! simply the release tag without its `v` prefix — no extraction step is
//! needed (unlike e.g. ws-scrcpy's older patched fork, which bakes in its own
//! ad-hoc version string; that fork's jar was evaluated for this feature but
//! rejected in favor of the official jar since its wire protocol/launch
//! arguments diverge from vanilla scrcpy in ways not worth reverse-engineering
//! when the official binary + documented protocol are both freely available).
pub const SCRCPY_SERVER_VERSION: &str = "4.1";

/// Path the server jar is pushed to on the device, matching scrcpy's own
/// hardcoded `SC_DEVICE_SERVER_PATH` (`app/src/server.c`).
pub const SCRCPY_DEVICE_JAR_PATH: &str = "/data/local/tmp/scrcpy-server.jar";

/// The abstract local socket name the on-device server listens on
/// (`tunnel_forward=true` mode) when no `scid` is passed on the command line
/// (`DesktopConnection.getSocketName`, `server/.../device/DesktopConnection.java`
/// falls back to the bare `"scrcpy"` name when `scid == -1`, its default).
/// Fine for v1: only one mirror session runs at a time, so there's no
/// multi-instance collision to guard against with an explicit `scid`.
pub const SCRCPY_SOCKET_NAME: &str = "scrcpy";

pub const SCRCPY_SERVER_JAR: &[u8] = include_bytes!("../../../vendor/scrcpy/scrcpy-server.jar");
