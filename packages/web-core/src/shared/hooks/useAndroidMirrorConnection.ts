import { useCallback, useEffect, useRef, useState } from 'react';
import { openLocalApiWebSocket } from '@/shared/lib/localApiTransport';

export type AndroidMirrorStatus =
  | 'idle'
  | 'connecting'
  | 'streaming'
  | 'error'
  | 'unsupported';

export interface AndroidMirrorEncoderOptions {
  maxSize: number | null;
  bitRateKbps: number | null;
  maxFps: number | null;
}

interface UseAndroidMirrorConnectionResult {
  status: AndroidMirrorStatus;
  errorMessage: string | null;
  retry: () => void;
  /** Send a JSON message over the WS (e.g. touch/key input) — a no-op if
   * the socket isn't currently open. */
  send: (data: object) => void;
}

const MAX_RETRIES = 6;
const MAX_RETRY_DELAY_MS = 8000;
const BASE_RETRY_DELAY_MS = 500;

// A cold-booting (or `-no-window` headless) emulator can take a good while
// before its scrcpy server is even reachable — every attempt in that window
// fails with one of these on the *very first* connect, before any WS
// packets flow, which `close_with_error`'s clean 1000 close otherwise turns
// into a hard `'error'` requiring a manual click. That's the wrong default
// for something that resolves on its own once the device finishes booting:
// retry automatically instead, on a fixed interval (not exponential —
// there's no server load to back off from, just a boot clock to wait out).
//
// The interval can't be short: each retry re-runs `client::connect()`'s
// *entire* deploy sequence (push the jar, `adb forward`, launch a fresh
// on-device server), not a lightweight reconnect. Firing that too often
// races a still-shutting-down previous attempt's server process against the
// next one's launch — confirmed live as the same "orphan squats the
// abstract socket" failure mode `client.rs`'s pre-launch `pkill` already
// exists to avoid, just self-inflicted by retrying too fast instead of by a
// leftover session.
const TRANSIENT_CONNECT_ERROR_RE = /early eof|timed out|connection refused/i;
const BOOT_RETRY_MAX = 15;
const BOOT_RETRY_DELAY_MS = 7000;

/**
 * Connects to `GET /api/android-mirror/ws`, streaming binary scrcpy packets.
 * Reconnect logic mirrors `TerminalProvider.tsx`'s `createTerminalConnection`
 * (exponential backoff capped at 8s, max 6 retries, no reconnect on a clean
 * code-1000 close) — simplified for view-only use: no `send`/`resize`.
 *
 * `status` goes straight to `'unsupported'` without ever opening a socket if
 * the browser lacks WebCodecs — there's nothing useful to stream to a
 * decoder that doesn't exist.
 *
 * Binary video packets are handed to `onFrame` directly from this hook's own
 * message listener — which is attached synchronously the moment the socket
 * is created — rather than returning the raw `WebSocket` for a consumer to
 * attach a second, later listener to. A consumer only receives the socket
 * reference after a subsequent render (once `setWs`/state propagates), which
 * is one tick too late: `EventTarget` never buffers events for listeners
 * that didn't exist yet when they fired, so any frames arriving in that gap
 * — routinely including the very first SPS/PPS config packet a decoder needs
 * before it can decode anything — would be silently dropped, leaving the
 * view stuck showing nothing.
 */
export function useAndroidMirrorConnection(
  deviceSerial: string | null,
  enabled: boolean,
  onFrame: (data: ArrayBuffer) => void,
  encoder?: AndroidMirrorEncoderOptions
): UseAndroidMirrorConnectionResult {
  const [status, setStatus] = useState<AndroidMirrorStatus>('idle');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const generationRef = useRef(0);
  const retryCountRef = useRef(0);
  const bootRetryCountRef = useRef(0);
  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const onFrameRef = useRef(onFrame);
  onFrameRef.current = onFrame;

  const isSupported = typeof window !== 'undefined' && 'VideoDecoder' in window;

  const connect = useCallback(
    (generation: number) => {
      if (generationRef.current !== generation) return;

      setStatus('connecting');
      setErrorMessage(null);

      const searchParams = new URLSearchParams();
      if (deviceSerial) searchParams.set('device_serial', deviceSerial);
      if (encoder?.maxSize)
        searchParams.set('max_size', String(encoder.maxSize));
      if (encoder?.bitRateKbps) {
        searchParams.set('bit_rate_kbps', String(encoder.bitRateKbps));
      }
      if (encoder?.maxFps) searchParams.set('max_fps', String(encoder.maxFps));
      const paramsString = searchParams.toString();
      const params = paramsString ? `?${paramsString}` : '';

      void (async () => {
        try {
          const socket = await openLocalApiWebSocket(
            `/api/android-mirror/ws${params}`
          );
          if (generationRef.current !== generation) {
            socket.close();
            return;
          }
          socket.binaryType = 'arraybuffer';
          wsRef.current = socket;

          socket.onopen = () => {
            retryCountRef.current = 0;
          };

          socket.addEventListener('message', (event) => {
            if (typeof event.data === 'string') {
              try {
                const msg = JSON.parse(event.data);
                if (msg.type === 'ready') {
                  setStatus('streaming');
                } else if (msg.type === 'error' && msg.message) {
                  if (
                    TRANSIENT_CONNECT_ERROR_RE.test(msg.message) &&
                    bootRetryCountRef.current < BOOT_RETRY_MAX
                  ) {
                    bootRetryCountRef.current += 1;
                    // Status deliberately stays 'connecting' — this isn't a
                    // failure from the user's point of view, just a longer
                    // wait. `errorMessage` doubles as the hint text shown
                    // under the spinner in that state (see
                    // AndroidMirrorView).
                    setErrorMessage(
                      'Waiting for the device to finish booting…'
                    );
                    retryTimerRef.current = setTimeout(() => {
                      connect(generation);
                    }, BOOT_RETRY_DELAY_MS);
                  } else {
                    setErrorMessage(msg.message);
                    setStatus('error');
                  }
                }
              } catch {
                // ignore
              }
              return;
            }
            if (event.data instanceof ArrayBuffer) {
              onFrameRef.current(event.data);
            } else {
              console.debug(
                '[useAndroidMirrorConnection] binary message but not ArrayBuffer, typeof:',
                typeof event.data,
                event.data?.constructor?.name
              );
            }
          });

          socket.onerror = () => {
            // followed by onclose, which drives reconnection
          };

          socket.onclose = (event) => {
            if (generationRef.current !== generation) return;
            wsRef.current = null;

            if (event.code === 1000 && event.wasClean) {
              // Server sends a `{type: 'error'}` text message before a clean
              // close on deploy/connect failure — status is already 'error'
              // in that case (or a boot-retry is already scheduled, in
              // which case `retryTimerRef` is already set — leave it alone
              // rather than clobbering 'connecting' back to 'idle'). A
              // clean close with no prior error is just a normal end of
              // session.
              if (retryTimerRef.current) return;
              setStatus((prev) => (prev === 'error' ? prev : 'idle'));
              return;
            }

            if (retryCountRef.current >= MAX_RETRIES) {
              setStatus('error');
              setErrorMessage((prev) => prev ?? 'Connection lost.');
              return;
            }

            const delay = Math.min(
              MAX_RETRY_DELAY_MS,
              BASE_RETRY_DELAY_MS * 2 ** retryCountRef.current
            );
            retryCountRef.current += 1;
            retryTimerRef.current = setTimeout(() => {
              connect(generation);
            }, delay);
          };
        } catch {
          if (generationRef.current !== generation) return;
          if (retryCountRef.current >= MAX_RETRIES) {
            setStatus('error');
            setErrorMessage('Could not open the mirror connection.');
            return;
          }
          const delay = Math.min(
            MAX_RETRY_DELAY_MS,
            BASE_RETRY_DELAY_MS * 2 ** retryCountRef.current
          );
          retryCountRef.current += 1;
          retryTimerRef.current = setTimeout(() => connect(generation), delay);
        }
      })();
    },
    [deviceSerial, encoder?.maxSize, encoder?.bitRateKbps, encoder?.maxFps]
  );

  const teardown = useCallback(() => {
    if (retryTimerRef.current) {
      clearTimeout(retryTimerRef.current);
      retryTimerRef.current = null;
    }
    wsRef.current?.close();
    wsRef.current = null;
  }, []);

  useEffect(() => {
    if (!enabled) {
      teardown();
      setStatus('idle');
      return;
    }
    if (!isSupported) {
      setStatus('unsupported');
      return;
    }

    generationRef.current += 1;
    retryCountRef.current = 0;
    bootRetryCountRef.current = 0;
    connect(generationRef.current);

    return () => {
      generationRef.current += 1; // invalidate any in-flight connect/retry
      teardown();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    enabled,
    deviceSerial,
    isSupported,
    encoder?.maxSize,
    encoder?.bitRateKbps,
    encoder?.maxFps,
  ]);

  const retry = useCallback(() => {
    if (!enabled || !isSupported) return;
    generationRef.current += 1;
    retryCountRef.current = 0;
    bootRetryCountRef.current = 0;
    teardown();
    connect(generationRef.current);
  }, [enabled, isSupported, teardown, connect]);

  // Fire-and-forget: a dropped touch/key event isn't worth surfacing as a
  // connection error, and the socket briefly not being open (still
  // connecting, or a reconnect in flight) is routine, not exceptional.
  const send = useCallback((data: object) => {
    const socket = wsRef.current;
    if (socket?.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify(data));
    }
  }, []);

  return { status, errorMessage, retry, send };
}
