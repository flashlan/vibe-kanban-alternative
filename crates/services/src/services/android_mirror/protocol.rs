//! Pure parsing of scrcpy's on-device server wire protocol (video socket
//! only — no audio/control). Verified directly against scrcpy v4.1 source:
//! `server/src/main/java/com/genymobile/scrcpy/device/{DesktopConnection,Streamer}.java`
//! (Apache-2.0, github.com/Genymobile/scrcpy).
//!
//! Wire sequence on the video socket, once accepted:
//! 1. One dummy byte (`DesktopConnection.open`, `sendDummyByte`, default
//!    true) — written on the first-ever-accepted socket, which is the video
//!    socket here since audio/control are disabled.
//! 2. Device name: 64 bytes, UTF-8, NUL-padded (`sendDeviceMeta`,
//!    `DEVICE_NAME_FIELD_LENGTH = 64`).
//! 3. Codec ID: 4 bytes, big-endian i32 (`Streamer.writeVideoHeader`).
//! 4. Then a stream of 12-byte headers, each either:
//!    - a **session** packet (`Streamer.writeSessionMeta`): top bit of byte 0
//!      set (`PACKET_FLAG_SESSION = 1i64 << 63`) — 4-byte flags (bit31 set,
//!      +bit0 if client-resize) + 4-byte width + 4-byte height, all
//!      big-endian. No payload follows; sent once at stream start and again
//!      on resolution change (e.g. rotation).
//!    - a **frame** packet (`Streamer.writeFrameMeta`): top bit of byte 0
//!      clear — 8-byte big-endian `ptsAndFlags` (bit62 = config/SPS-PPS
//!      packet, bit61 = key frame, bits 0..=60 = PTS) + 4-byte big-endian
//!      payload size, followed by that many bytes of raw H264 Annex-B NAL
//!      data.

pub const DEVICE_NAME_FIELD_LENGTH: usize = 64;
pub const HEADER_LENGTH: usize = 12;

const FLAG_SESSION: u64 = 1 << 63;
const FLAG_CONFIG: u64 = 1 << 62;
const FLAG_KEY_FRAME: u64 = 1 << 61;
const PTS_MASK: u64 = FLAG_KEY_FRAME - 1; // low 61 bits

/// H264 codec id, as scrcpy's `VideoCodec.java` encodes it: the 4 ASCII
/// bytes "h264" packed big-endian into an i32 (`0x68323634`).
pub const CODEC_ID_H264: i32 = 0x6832_3634;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    Session {
        width: u32,
        height: u32,
        is_client_resize: bool,
    },
    Frame {
        pts: u64,
        config: bool,
        key_frame: bool,
        payload_size: u32,
    },
}

/// Parse one 12-byte header. Does not touch the payload — callers read
/// `payload_size` more bytes themselves for `Packet::Frame`.
pub fn parse_header(bytes: &[u8; HEADER_LENGTH]) -> Packet {
    let first_u64 = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
    let last_u32 = u32::from_be_bytes(bytes[8..12].try_into().unwrap());

    if first_u64 & FLAG_SESSION != 0 {
        // Session packet: reinterpret the same 12 bytes as [flags:4][w:4][h:4].
        let flags = (first_u64 >> 32) as u32;
        let width = (first_u64 & 0xFFFF_FFFF) as u32;
        let height = last_u32;
        Packet::Session {
            width,
            height,
            is_client_resize: flags & 1 != 0,
        }
    } else {
        Packet::Frame {
            config: first_u64 & FLAG_CONFIG != 0,
            key_frame: first_u64 & FLAG_KEY_FRAME != 0,
            pts: first_u64 & PTS_MASK,
            payload_size: last_u32,
        }
    }
}

pub fn decode_codec_id(bytes: [u8; 4]) -> i32 {
    i32::from_be_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_session_packet() {
        // flags = 0x80000000 (session bit only, no client-resize), width=1080, height=2400
        let mut bytes = [0u8; HEADER_LENGTH];
        bytes[0..4].copy_from_slice(&0x8000_0000u32.to_be_bytes());
        bytes[4..8].copy_from_slice(&1080u32.to_be_bytes());
        bytes[8..12].copy_from_slice(&2400u32.to_be_bytes());

        assert_eq!(
            parse_header(&bytes),
            Packet::Session {
                width: 1080,
                height: 2400,
                is_client_resize: false,
            }
        );
    }

    #[test]
    fn parses_session_packet_with_client_resize_bit() {
        let mut bytes = [0u8; HEADER_LENGTH];
        bytes[0..4].copy_from_slice(&0x8000_0001u32.to_be_bytes());
        bytes[4..8].copy_from_slice(&640u32.to_be_bytes());
        bytes[8..12].copy_from_slice(&480u32.to_be_bytes());

        assert_eq!(
            parse_header(&bytes),
            Packet::Session {
                width: 640,
                height: 480,
                is_client_resize: true,
            }
        );
    }

    #[test]
    fn parses_config_packet() {
        // PACKET_FLAG_CONFIG only, size = 23 bytes of SPS/PPS.
        let pts_and_flags: u64 = FLAG_CONFIG;
        let mut bytes = [0u8; HEADER_LENGTH];
        bytes[0..8].copy_from_slice(&pts_and_flags.to_be_bytes());
        bytes[8..12].copy_from_slice(&23u32.to_be_bytes());

        assert_eq!(
            parse_header(&bytes),
            Packet::Frame {
                pts: 0,
                config: true,
                key_frame: false,
                payload_size: 23,
            }
        );
    }

    #[test]
    fn parses_key_frame_packet() {
        let pts: u64 = 123_456_789;
        let pts_and_flags = pts | FLAG_KEY_FRAME;
        let mut bytes = [0u8; HEADER_LENGTH];
        bytes[0..8].copy_from_slice(&pts_and_flags.to_be_bytes());
        bytes[8..12].copy_from_slice(&4096u32.to_be_bytes());

        assert_eq!(
            parse_header(&bytes),
            Packet::Frame {
                pts,
                config: false,
                key_frame: true,
                payload_size: 4096,
            }
        );
    }

    #[test]
    fn parses_delta_frame_packet() {
        let pts: u64 = 987_654_321;
        let mut bytes = [0u8; HEADER_LENGTH];
        bytes[0..8].copy_from_slice(&pts.to_be_bytes());
        bytes[8..12].copy_from_slice(&512u32.to_be_bytes());

        assert_eq!(
            parse_header(&bytes),
            Packet::Frame {
                pts,
                config: false,
                key_frame: false,
                payload_size: 512,
            }
        );
    }

    #[test]
    fn codec_id_h264_matches_ascii_bytes() {
        assert_eq!(decode_codec_id(*b"h264"), CODEC_ID_H264);
    }
}
