import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
} from 'react';
import {
  SpinnerIcon,
  DeviceMobileCameraIcon,
  PictureInPictureIcon,
} from '@phosphor-icons/react';
import { cn } from '../lib/cn';
import { PrimaryButton } from './PrimaryButton';
import { IconButton } from './IconButton';

export type AndroidMirrorStatus =
  | 'idle'
  | 'connecting'
  | 'streaming'
  | 'error'
  | 'unsupported';

export interface AndroidMirrorViewHandle {
  /** Feed one raw wire packet (12-byte header + payload) to the decoder. */
  pushFrame: (data: ArrayBuffer) => void;
}

// Mirrors the backend's `AndroidMirrorInboundMessage` (server/routes/
// android_mirror.rs) field-for-field — this is what gets JSON.stringify'd
// and sent over the same WS the video comes in on. `x`/`y` are absolute
// device pixels; this component does the canvas-CSS-size -> device-pixel
// mapping itself since it's the one that knows the canvas's rendered size.
export type AndroidMirrorInputMessage =
  | {
      type: 'touch';
      action: 'down' | 'up' | 'move';
      x: number;
      y: number;
      screen_width: number;
      screen_height: number;
    }
  | { type: 'key'; action: 'down' | 'up'; keycode: number }
  | { type: 'text'; text: string };

export interface AndroidMirrorViewProps {
  status: AndroidMirrorStatus;
  errorMessage: string | null;
  onRetry: () => void;
  className?: string;
  /** Omit to hide the detach button entirely (e.g. unsupported browser, or
   * this instance is already the detached copy rendered inside the
   * floating window — see `AndroidMirrorContainer`). */
  onDetach?: () => void;
  /** Omit to disable touch/keyboard forwarding entirely (view-only). */
  onInput?: (msg: AndroidMirrorInputMessage) => void;
}

// Physical keys forwarded as Android `KEYCODE_*` injection (navigation/
// editing keys with no single-character text representation). Everything
// else with `e.key.length === 1` goes through as `TYPE_INJECT_TEXT`
// instead — that gets shift/dead-key/composition handling for free from
// the browser, which reproducing via raw keycode+metastate emulation
// would not.
const CONTROL_KEYCODES: Record<string, number> = {
  Backspace: 67,
  Enter: 66,
  Tab: 61,
  Escape: 111,
  ArrowLeft: 21,
  ArrowRight: 22,
  ArrowUp: 19,
  ArrowDown: 20,
  Delete: 112,
  Home: 122, // AKEYCODE_MOVE_HOME (cursor movement, not the device Home key)
  End: 123, // AKEYCODE_MOVE_END
};

/** Map a pointer event's CSS-pixel position to the canvas's actual device
 * pixel resolution (`canvas.width`/`height`, set to the decoded frame's
 * size) — the canvas is very likely displayed at a different (scaled)
 * on-screen size via `max-w-full max-h-full`. */
function toDevicePosition(
  canvas: HTMLCanvasElement,
  clientX: number,
  clientY: number
): { x: number; y: number } {
  const rect = canvas.getBoundingClientRect();
  const relX = rect.width > 0 ? (clientX - rect.left) / rect.width : 0;
  const relY = rect.height > 0 ? (clientY - rect.top) / rect.height : 0;
  const x = Math.min(canvas.width - 1, Math.max(0, Math.round(relX * canvas.width)));
  const y = Math.min(canvas.height - 1, Math.max(0, Math.round(relY * canvas.height)));
  return { x, y };
}

// Wire format verified against scrcpy v4.1 source (Apache-2.0,
// github.com/Genymobile/scrcpy) — see
// `services::services::android_mirror::protocol` (Rust) for the
// byte-for-byte-matching parser this mirrors, and its doc comment for the
// full protocol writeup. 12-byte header, big-endian throughout:
//   - Session packet (top bit of byte 0 set): [flags:4][width:4][height:4].
//   - Frame packet (top bit clear): [ptsAndFlags:8][payloadSize:4], where
//     bit62=config(SPS/PPS), bit61=key-frame, bits0..=60=pts (µs).
const SESSION_FLAG_MASK = 0x8000_0000;
const CONFIG_FLAG_MASK = 0x4000_0000; // bit 62 of the 64-bit value, i.e. bit 30 of its high 32 bits
const KEY_FRAME_FLAG_MASK = 0x2000_0000; // bit 61, i.e. bit 29 of the high 32 bits
const PTS_HIGH_MASK = 0x1fff_ffff; // low 29 bits of the high word = bits 32..=60 of pts
const HEADER_LENGTH = 12;

interface FrameHeader {
  kind: 'session' | 'frame';
  width?: number;
  height?: number;
  config?: boolean;
  keyFrame?: boolean;
  // Raw 61-bit device PTS (elapsed-time-since-boot microseconds — routinely
  // in the tens of billions for any phone that isn't freshly rebooted).
  // Kept as a BigInt: combining the two 32-bit halves with plain
  // floating-point multiplication (`hi * 2**32 + lo`) silently exceeds
  // Number.MAX_SAFE_INTEGER for exactly that realistic PTS range, rounding
  // multiple distinct frames onto the same (or a non-monotonic) timestamp —
  // confirmed live as the actual cause of a real device's stream reliably
  // hitting `VideoDecoder`'s `"EncodingError: Decoding error"` a few frames
  // in, well past whatever a small synthetic test PTS would ever expose.
  // WebCodecs' `timestamp` field requires a plain `number`, not a BigInt, so
  // the caller narrows this to a stream-relative value (see `pushFrame`)
  // instead of ever converting the raw PTS directly.
  ptsRaw?: bigint;
  payloadSize?: number;
}

function parseHeader(view: DataView): FrameHeader {
  const hi = view.getUint32(0, false);
  if (hi & SESSION_FLAG_MASK) {
    return {
      kind: 'session',
      width: view.getUint32(4, false),
      height: view.getUint32(8, false),
    };
  }
  const lo = view.getUint32(4, false);
  const ptsRaw =
    (BigInt(hi & PTS_HIGH_MASK) << 32n) | BigInt(lo >>> 0);
  return {
    kind: 'frame',
    config: (hi & CONFIG_FLAG_MASK) !== 0,
    keyFrame: (hi & KEY_FRAME_FLAG_MASK) !== 0,
    ptsRaw,
    payloadSize: view.getUint32(8, false),
  };
}

/**
 * Find the first SPS NAL (`nal_unit_type === 7`) in an Annex-B config
 * packet and return its `profile_idc`/`constraint_flags`/`level_idc` bytes
 * (bytes 1-3, right after the 1-byte NAL header) — enough to build the
 * WebCodecs `avc1.PPCCLL` codec string. Scans for any `00 00 01` start-code
 * suffix, which matches both 3- and 4-byte Annex-B start codes.
 */
function findSpsProfileLevelBytes(configPacket: Uint8Array): Uint8Array | null {
  for (let i = 0; i + 2 < configPacket.length; i++) {
    if (
      configPacket[i] === 0 &&
      configPacket[i + 1] === 0 &&
      configPacket[i + 2] === 1
    ) {
      const nalHeaderIdx = i + 3;
      if (nalHeaderIdx >= configPacket.length) continue;
      const nalType = configPacket[nalHeaderIdx] & 0x1f;
      if (nalType === 7 && nalHeaderIdx + 3 < configPacket.length) {
        return configPacket.slice(nalHeaderIdx + 1, nalHeaderIdx + 4);
      }
    }
  }
  return null;
}

function buildAvcCodecString(spsBytes: Uint8Array): string {
  const hex = (n: number) => n.toString(16).padStart(2, '0');
  return `avc1.${hex(spsBytes[0])}${hex(spsBytes[1])}${hex(spsBytes[2])}`;
}

export const AndroidMirrorView = forwardRef<
  AndroidMirrorViewHandle,
  AndroidMirrorViewProps
>(function AndroidMirrorView(
  { status, errorMessage, onRetry, className, onDetach, onInput },
  ref
) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const decoderRef = useRef<VideoDecoder | null>(null);
  const configuredRef = useRef(false);
  // First frame's raw device PTS, used to rebase every subsequent timestamp
  // to a small, safe-integer, stream-relative value (see `ptsRaw` doc
  // comment above for why the raw value can't be used directly).
  const firstPtsRef = useRef<bigint | null>(null);
  // The config packet's raw SPS/PPS bytes, held until the very next frame
  // packet (the keyframe scrcpy always sends right after it) arrives. A
  // config-only chunk with no coded picture in it is rejected outright by
  // `decode()` ("A key frame is required after configure() or flush()" —
  // confirmed live), so the parameter sets can't be fed to the decoder on
  // their own; they have to be prepended onto the first keyframe's payload
  // as one combined Annex-B access unit instead.
  const pendingConfigPayloadRef = useRef<Uint8Array | null>(null);
  const isPointerDownRef = useRef(false);
  const [hasDrawnFrame, setHasDrawnFrame] = useState(false);
  // Separate from the `status`/`errorMessage` props (which only know about
  // the WS connection) — the decoder can fail entirely independently of a
  // perfectly healthy connection. `'VideoDecoder' in window` only proves the
  // *class* exists, not that this browser's implementation actually accepts
  // the specific config used below; Safari is a known case of the former
  // without the latter (its WebCodecs support predates broad Annex-B/H264
  // config compatibility with what a raw scrcpy stream sends).
  const [decoderError, setDecoderError] = useState<string | null>(null);

  // One decoder per mount, independent of when frames start arriving. Frames
  // are pushed in via `pushFrame` (imperative handle) rather than this
  // component listening on the WebSocket itself: the socket is created
  // asynchronously inside a hook one level up, so a `ws` prop would only
  // reach this component's own `addEventListener` a render cycle AFTER the
  // socket exists — any frames the server sends in that gap (routinely
  // including the SPS/PPS config packet needed to configure the decoder at
  // all) would be silently dropped, since `EventTarget` never buffers events
  // for listeners that don't exist yet. A ref set during mount/commit is
  // guaranteed to be in place before any effect (and therefore before the
  // async connection in the parent hook can possibly resolve and start
  // receiving messages), so routing frames through an imperative handle
  // instead eliminates the race entirely.
  useEffect(() => {
    if (typeof window === 'undefined' || !('VideoDecoder' in window)) {
      console.debug('[AndroidMirrorView] mount: VideoDecoder not in window');
      return;
    }
    console.debug('[AndroidMirrorView] mount: creating decoder');

    configuredRef.current = false;
    firstPtsRef.current = null;
    pendingConfigPayloadRef.current = null;
    setHasDrawnFrame(false);
    setDecoderError(null);
    const decoder = new VideoDecoder({
      output: (frame) => {
        console.debug(
          '[AndroidMirrorView] decoder output frame',
          frame.displayWidth,
          frame.displayHeight
        );
        const canvas = canvasRef.current;
        if (canvas) {
          if (canvas.width !== frame.displayWidth) {
            canvas.width = frame.displayWidth;
          }
          if (canvas.height !== frame.displayHeight) {
            canvas.height = frame.displayHeight;
          }
          const ctx = canvas.getContext('2d');
          ctx?.drawImage(frame, 0, 0);
        }
        frame.close();
        setHasDrawnFrame(true);
      },
      error: (e) => {
        console.error('[AndroidMirrorView] VideoDecoder error:', e);
        setDecoderError(
          "This browser's video decoder rejected the stream — try Chrome or Edge."
        );
      },
    });
    decoderRef.current = decoder;
    console.debug('[AndroidMirrorView] decoder created, state:', decoder.state);

    return () => {
      if (decoder.state !== 'closed') {
        decoder.close();
      }
      decoderRef.current = null;
    };
  }, []);

  useImperativeHandle(
    ref,
    () => ({
      pushFrame(buffer: ArrayBuffer) {
        const decoder = decoderRef.current;
        if (!decoder) {
          console.debug(
            '[AndroidMirrorView] pushFrame called but decoder is null'
          );
          return;
        }
        if (buffer.byteLength < HEADER_LENGTH) return;

        const header = parseHeader(new DataView(buffer, 0, HEADER_LENGTH));
        if (header.kind === 'session') {
          console.debug(
            '[AndroidMirrorView] session packet',
            header.width,
            header.height
          );
          return;
        }

        const payload = new Uint8Array(
          buffer,
          HEADER_LENGTH,
          header.payloadSize
        );

        if (header.config) {
          const sps = findSpsProfileLevelBytes(payload);
          const codecString = sps ? buildAvcCodecString(sps) : null;
          console.debug(
            '[AndroidMirrorView] config packet, sps found:',
            !!sps,
            'codec:',
            codecString,
            'decoder.state:',
            decoder.state
          );
          if (sps && decoder.state === 'unconfigured') {
            try {
              decoder.configure({
                codec: codecString!,
                // NOT passing codedWidth/codedHeight from the session
                // packet: that's the *display* resolution, but H264's
                // internal coded size is macroblock-aligned (padded up to a
                // multiple of 16) — 1080 isn't one. Passing the wrong
                // (smaller) coded size here produced an actual
                // "EncodingError: Decoding error" after a few frames
                // (confirmed live), so leave this to the SPS itself, which
                // encodes the real padded dimensions correctly.
                // @ts-expect-error -- `avc.format` (Annex-B extension) isn't
                // in TS's bundled WebCodecs types yet; scrcpy emits Annex-B
                // NALs (start-code-prefixed) and this tells the decoder to
                // accept them as-is instead of requiring length-prefixed
                // AVCC.
                avc: { format: 'annexb' },
              });
              configuredRef.current = true;
              // Held for the next frame packet — see the ref's doc comment
              // for why this can't just be `decode()`-d here on its own.
              pendingConfigPayloadRef.current = payload;
              console.debug(
                '[AndroidMirrorView] configure() succeeded, decoder.state:',
                decoder.state
              );
            } catch (e) {
              // `configure()` can throw synchronously for a config the
              // implementation flat-out rejects (seen on Safari, whose
              // WebCodecs support predates broad compatibility with a raw
              // Annex-B scrcpy stream) — the async `error` callback above
              // only catches failures *after* a successful configure().
              console.error('[AndroidMirrorView] configure() failed:', e);
              setDecoderError(
                "This browser's video decoder doesn't support this stream — try Chrome or Edge."
              );
            }
          }
          return;
        }

        if (!configuredRef.current || decoder.state !== 'configured') {
          console.debug(
            '[AndroidMirrorView] frame packet dropped: configuredRef=',
            configuredRef.current,
            'decoder.state=',
            decoder.state
          );
          return;
        }

        if (firstPtsRef.current === null) {
          firstPtsRef.current = header.ptsRaw ?? 0n;
        }
        const relativePts = Number(
          (header.ptsRaw ?? 0n) - firstPtsRef.current
        );

        let chunkData = payload;
        if (pendingConfigPayloadRef.current) {
          // Prepend the SPS/PPS bytes onto this (the keyframe that always
          // immediately follows a config packet) so the decoder gets one
          // combined Annex-B access unit containing both the parameter sets
          // and the actual coded picture in a single `decode()` call.
          const config = pendingConfigPayloadRef.current;
          const combined = new Uint8Array(config.length + payload.length);
          combined.set(config, 0);
          combined.set(payload, config.length);
          chunkData = combined;
          pendingConfigPayloadRef.current = null;
        }

        console.debug(
          '[AndroidMirrorView] decoding frame, key=',
          header.keyFrame,
          'size=',
          header.payloadSize,
          'pts=',
          relativePts
        );
        try {
          decoder.decode(
            new EncodedVideoChunk({
              type: header.keyFrame ? 'key' : 'delta',
              timestamp: relativePts,
              data: chunkData,
            })
          );
        } catch (e) {
          console.error('[AndroidMirrorView] decode() failed:', e);
          setDecoderError(
            "This browser's video decoder rejected a frame — try Chrome or Edge."
          );
        }
      },
    }),
    []
  );

  // Gated on an actually-drawn frame, not just the connection reaching
  // 'streaming' — that status flips as soon as the server's JSON "ready"
  // message arrives, which says nothing about whether video has decoded yet.
  const showCanvas = status === 'streaming' && hasDrawnFrame && !decoderError;

  const handlePointerDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas || !onInput || !showCanvas) return;
    e.preventDefault();
    // `preventDefault()` on mousedown suppresses the browser's default
    // focus-on-click behavior in some engines — focus explicitly so
    // keyboard forwarding (below) actually has something to attach to.
    canvas.focus();
    isPointerDownRef.current = true;
    const { x, y } = toDevicePosition(canvas, e.clientX, e.clientY);
    onInput({
      type: 'touch',
      action: 'down',
      x,
      y,
      screen_width: canvas.width,
      screen_height: canvas.height,
    });
  };

  const handlePointerMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas || !onInput || !isPointerDownRef.current) return;
    const { x, y } = toDevicePosition(canvas, e.clientX, e.clientY);
    onInput({
      type: 'touch',
      action: 'move',
      x,
      y,
      screen_width: canvas.width,
      screen_height: canvas.height,
    });
  };

  const handlePointerUp = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas || !onInput || !isPointerDownRef.current) return;
    isPointerDownRef.current = false;
    const { x, y } = toDevicePosition(canvas, e.clientX, e.clientY);
    onInput({
      type: 'touch',
      action: 'up',
      x,
      y,
      screen_width: canvas.width,
      screen_height: canvas.height,
    });
  };

  // A drag that ends by leaving the canvas (rather than a mouseup inside
  // it) would otherwise leave the device thinking a finger is still down —
  // release it here too.
  const handlePointerLeave = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (isPointerDownRef.current) handlePointerUp(e);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLCanvasElement>) => {
    if (!onInput) return;
    const keycode = CONTROL_KEYCODES[e.key];
    if (keycode !== undefined) {
      e.preventDefault();
      onInput({ type: 'key', action: 'down', keycode });
      return;
    }
    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      onInput({ type: 'text', text: e.key });
    }
  };

  const handleKeyUp = (e: React.KeyboardEvent<HTMLCanvasElement>) => {
    if (!onInput) return;
    const keycode = CONTROL_KEYCODES[e.key];
    if (keycode !== undefined) {
      e.preventDefault();
      onInput({ type: 'key', action: 'up', keycode });
    }
  };

  return (
    <div
      className={cn(
        'bg-brand/20 relative w-full h-full flex items-center justify-center overflow-hidden',
        className
      )}
    >
      {onDetach && (
        <IconButton
          icon={PictureInPictureIcon}
          onClick={onDetach}
          aria-label="Detach into a floating window"
          title="Detach into a floating window"
          variant="tertiary"
          className="absolute right-base top-base z-10"
        />
      )}
      <canvas
        ref={canvasRef}
        tabIndex={onInput ? 0 : undefined}
        onMouseDown={handlePointerDown}
        onMouseMove={handlePointerMove}
        onMouseUp={handlePointerUp}
        onMouseLeave={handlePointerLeave}
        onKeyDown={handleKeyDown}
        onKeyUp={handleKeyUp}
        className={cn(
          'max-w-full max-h-full outline-none',
          onInput && showCanvas && 'cursor-pointer',
          !showCanvas && 'hidden'
        )}
      />
      {!showCanvas && (
        <div className="flex flex-col items-center gap-base text-low">
          {decoderError ? (
            <>
              <DeviceMobileCameraIcon className="size-icon-lg" />
              <p className="text-sm">{decoderError}</p>
            </>
          ) : status === 'unsupported' ? (
            <>
              <DeviceMobileCameraIcon className="size-icon-lg" />
              <p className="text-sm">
                This browser doesn&apos;t support WebCodecs — the Android
                mirror needs Chrome or Edge.
              </p>
            </>
          ) : status === 'error' ? (
            <>
              <DeviceMobileCameraIcon className="size-icon-lg" />
              <p className="text-sm">{errorMessage ?? 'Mirror disconnected.'}</p>
              <PrimaryButton value="Retry" onClick={onRetry} />
            </>
          ) : status === 'idle' ? (
            // Deliberately distinct from the connecting/stuck spinner below —
            // `idle` here means the user hit "Disconnect" (see the Controls
            // section's device picker), not a connection in flight. A
            // spinner claiming to be "Connecting…" would be actively wrong.
            <>
              <DeviceMobileCameraIcon className="size-icon-lg" />
              <p className="text-sm">Disconnected.</p>
              <PrimaryButton value="Connect" onClick={onRetry} />
            </>
          ) : (
            <>
              <SpinnerIcon className="size-icon-lg animate-spin text-brand" />
              <p className="text-sm">Connecting to device…</p>
              {/* Set (and kept non-null) while `useAndroidMirrorConnection`
                  is auto-retrying a boot-time connection failure — e.g. a
                  cold-booting or `-no-window` emulator whose scrcpy server
                  isn't reachable yet. Status stays 'connecting' the whole
                  time on purpose (see that hook), so this is the only
                  visible sign a retry is happening rather than the panel
                  being silently stuck. */}
              {errorMessage && (
                <p className="text-xs text-low">{errorMessage}</p>
              )}
              {/* No error is guaranteed to ever fire here: the connection
                  itself can be perfectly healthy (WS open, "ready" received)
                  while the phone simply never sends a frame — e.g. the
                  screen dozed/locked before the socket connected, so scrcpy
                  captures nothing. Without a manual way out, that reads as a
                  permanently stuck panel with no recourse, so this button is
                  always available here too, not just on `status === 'error'`. */}
              <PrimaryButton
                variant="tertiary"
                value="Reconnect"
                onClick={onRetry}
              />
            </>
          )}
        </div>
      )}
    </div>
  );
});
