//! Live Android screen mirroring (view-only) via a vendored scrcpy-server.
//!
//! See `vendor.rs` for provenance/licensing of the vendored jar, `protocol.rs`
//! for the on-the-wire framing, `device.rs` for adb device discovery, and
//! `client.rs` for the deploy/connect orchestration.

pub mod client;
pub mod control;
pub mod control_socket;
pub mod device;
pub mod emulator;
pub mod protocol;
pub mod vendor;
