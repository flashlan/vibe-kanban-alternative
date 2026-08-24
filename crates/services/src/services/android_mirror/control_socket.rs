//! Encodes scrcpy control-channel messages (touch/key/text injection) — the
//! wire format scrcpy's own client sends over the *control* socket, a
//! second connection separate from the video socket `protocol.rs` parses.
//!
//! Byte layouts verified directly against the vendored server jar (v4.1) by
//! disassembling `com.genymobile.scrcpy.control.ControlMessageReader` with
//! `dexdump -d` — not from memory of the (unvendored, C) desktop client
//! source, since a wrong field order/width here fails silently (the server
//! just misparses the next message) rather than erroring immediately.
//! `ControlMessage`'s `TYPE_*` constants and `Controller.POINTER_ID_MOUSE`
//! were read the same way from the jar's static field tables.
//!
//! All multi-byte fields are big-endian (`DataInputStream`, like the video
//! socket's header).

/// `Controller.POINTER_ID_MOUSE` (`-1` as a `long` in Java, i.e. all bits
/// set) — the sentinel scrcpy's own desktop client uses for a
/// mouse-simulated single touch, as opposed to a real per-finger id.
const POINTER_ID_MOUSE: u64 = u64::MAX;

const TYPE_INJECT_KEYCODE: u8 = 0;
const TYPE_INJECT_TEXT: u8 = 1;
const TYPE_INJECT_TOUCH_EVENT: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchAction {
    Down,
    Up,
    Move,
}

impl TouchAction {
    fn android_action(self) -> u8 {
        match self {
            TouchAction::Down => 0, // AMOTION_EVENT_ACTION_DOWN
            TouchAction::Up => 1,   // AMOTION_EVENT_ACTION_UP
            TouchAction::Move => 2, // AMOTION_EVENT_ACTION_MOVE
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Down,
    Up,
}

impl KeyAction {
    fn android_action(self) -> u8 {
        match self {
            KeyAction::Down => 0, // AKEY_EVENT_ACTION_DOWN
            KeyAction::Up => 1,   // AKEY_EVENT_ACTION_UP
        }
    }
}

/// `ControlMessageReader.parseInjectTouchEvent` — 32 bytes total (including
/// the leading type byte): action(u8) pointerId(u64) x(i32) y(i32)
/// screenWidth(u16) screenHeight(u16) pressure(u16 fixed-point)
/// actionButton(i32) buttons(i32).
///
/// `x`/`y` are absolute device pixel coordinates, already mapped from
/// wherever the frame was displayed (the browser side does that, since it's
/// the one that knows the canvas's on-screen size vs. the frame's actual
/// decoded resolution).
pub fn encode_touch_event(
    action: TouchAction,
    x: i32,
    y: i32,
    screen_width: u16,
    screen_height: u16,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(32);
    buf.push(TYPE_INJECT_TOUCH_EVENT);
    buf.push(action.android_action());
    buf.extend_from_slice(&POINTER_ID_MOUSE.to_be_bytes());
    buf.extend_from_slice(&x.to_be_bytes());
    buf.extend_from_slice(&y.to_be_bytes());
    buf.extend_from_slice(&screen_width.to_be_bytes());
    buf.extend_from_slice(&screen_height.to_be_bytes());
    // `Binary.u16FixedPointToFloat`: 0xffff is special-cased to exactly
    // 1.0f; anything else is `raw / 65536.0f`. 0x0000 on release matches
    // scrcpy's own client (no pressure once the finger/button is up).
    let pressure: u16 = if action == TouchAction::Up {
        0x0000
    } else {
        0xffff
    };
    buf.extend_from_slice(&pressure.to_be_bytes());
    // AMOTION_EVENT_BUTTON_PRIMARY. `action_button` is which button
    // triggered *this* event (only meaningful for down/up); `buttons` is
    // the currently-held set. A move has neither.
    let button_mask: i32 = if action == TouchAction::Move { 0 } else { 1 };
    let held_mask: i32 = if action == TouchAction::Up { 0 } else { 1 };
    buf.extend_from_slice(&button_mask.to_be_bytes());
    buf.extend_from_slice(&held_mask.to_be_bytes());
    buf
}

/// `ControlMessageReader.parseInjectKeycode` — 14 bytes total: action(u8)
/// keycode(i32) repeat(i32) metaState(i32).
pub fn encode_key_event(action: KeyAction, keycode: i32, meta_state: i32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(14);
    buf.push(TYPE_INJECT_KEYCODE);
    buf.push(action.android_action());
    buf.extend_from_slice(&keycode.to_be_bytes());
    buf.extend_from_slice(&0i32.to_be_bytes()); // repeat
    buf.extend_from_slice(&meta_state.to_be_bytes());
    buf
}

/// `ControlMessageReader.parseInjectText` (via its 4-byte-length-prefixed
/// `parseString()`) — u32 length, then that many UTF-8 bytes.
pub fn encode_text_event(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut buf = Vec::with_capacity(5 + bytes.len());
    buf.push(TYPE_INJECT_TEXT);
    buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    buf.extend_from_slice(bytes);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_down_is_32_bytes_with_full_pressure_and_button_set() {
        let msg = encode_touch_event(TouchAction::Down, 540, 1170, 1080, 2340);
        assert_eq!(msg.len(), 32);
        assert_eq!(msg[0], TYPE_INJECT_TOUCH_EVENT);
        assert_eq!(msg[1], 0); // ACTION_DOWN
        assert_eq!(&msg[2..10], &u64::MAX.to_be_bytes()); // pointer id
        assert_eq!(&msg[10..14], &540i32.to_be_bytes());
        assert_eq!(&msg[14..18], &1170i32.to_be_bytes());
        assert_eq!(&msg[18..20], &1080u16.to_be_bytes());
        assert_eq!(&msg[20..22], &2340u16.to_be_bytes());
        assert_eq!(&msg[22..24], &0xffffu16.to_be_bytes()); // full pressure
        assert_eq!(&msg[24..28], &1i32.to_be_bytes()); // action_button
        assert_eq!(&msg[28..32], &1i32.to_be_bytes()); // buttons held
    }

    #[test]
    fn touch_up_has_zero_pressure_and_no_buttons_held() {
        let msg = encode_touch_event(TouchAction::Up, 0, 0, 1080, 2340);
        assert_eq!(msg[1], 1); // ACTION_UP
        assert_eq!(&msg[22..24], &0u16.to_be_bytes());
        assert_eq!(&msg[24..28], &1i32.to_be_bytes()); // action_button still set on up
        assert_eq!(&msg[28..32], &0i32.to_be_bytes()); // buttons now empty
    }

    #[test]
    fn touch_move_has_no_action_button() {
        let msg = encode_touch_event(TouchAction::Move, 10, 20, 1080, 2340);
        assert_eq!(msg[1], 2); // ACTION_MOVE
        assert_eq!(&msg[24..28], &0i32.to_be_bytes());
        assert_eq!(&msg[28..32], &1i32.to_be_bytes());
    }

    #[test]
    fn key_event_is_14_bytes() {
        let msg = encode_key_event(KeyAction::Down, 66, 0); // AKEYCODE_ENTER
        assert_eq!(msg.len(), 14);
        assert_eq!(msg[0], TYPE_INJECT_KEYCODE);
        assert_eq!(msg[1], 0);
        assert_eq!(&msg[2..6], &66i32.to_be_bytes());
        assert_eq!(&msg[6..10], &0i32.to_be_bytes());
        assert_eq!(&msg[10..14], &0i32.to_be_bytes());
    }

    #[test]
    fn text_event_is_length_prefixed_utf8() {
        let msg = encode_text_event("olá");
        assert_eq!(msg[0], TYPE_INJECT_TEXT);
        let len = u32::from_be_bytes(msg[1..5].try_into().unwrap());
        assert_eq!(len as usize, "olá".len()); // 4 bytes: o, l, á(2 bytes)
        assert_eq!(&msg[5..], "olá".as_bytes());
    }
}
