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
}

const MAX_RETRIES = 6;
const MAX_RETRY_DELAY_MS = 8000;
const BASE_RETRY_DELAY_MS = 500;

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
                  setErrorMessage(msg.message);
                  setStatus('error');
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
              // in that case. A clean close with no prior error is just a
              // normal end of session.
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
    teardown();
    connect(generationRef.current);
  }, [enabled, isSupported, teardown, connect]);

  return { status, errorMessage, retry };
}
