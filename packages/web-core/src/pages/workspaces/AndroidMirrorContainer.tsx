import { useCallback, useRef } from 'react';
import {
  AndroidMirrorView,
  type AndroidMirrorViewHandle,
} from '@vibe/ui/components/AndroidMirrorView';
import { useAndroidMirrorSettings } from '@/shared/hooks/useAndroidMirrorSettings';
import { useAndroidMirrorConnection } from '@/shared/hooks/useAndroidMirrorConnection';

interface AndroidMirrorContainerProps {
  workspaceId: string | undefined;
  className: string;
}

/**
 * Wires the per-workspace pinned device (`useAndroidMirrorSettings`) and the
 * live WS connection (`useAndroidMirrorConnection`) into the presentational
 * `AndroidMirrorView`. The device picker itself lives in the right sidebar
 * (mirrors Preview's URL bar living in its own toolbar, separate from the
 * main canvas) — see `RightSidebar.tsx`'s `MIRROR` case.
 *
 * Frames are pushed into the view via a ref (`pushFrame`) rather than handing
 * the raw `WebSocket` down as a prop — see `useAndroidMirrorConnection`'s doc
 * comment for why that would drop the decoder's first (config) packet.
 */
export function AndroidMirrorContainer({
  workspaceId,
  className,
}: AndroidMirrorContainerProps) {
  const {
    deviceSerial,
    maxSize,
    bitRateKbps,
    maxFps,
    manuallyDisconnected,
    setManuallyDisconnected,
  } = useAndroidMirrorSettings(workspaceId);
  const viewRef = useRef<AndroidMirrorViewHandle>(null);
  const onFrame = useCallback((data: ArrayBuffer) => {
    if (!viewRef.current) {
      console.debug(
        '[AndroidMirrorContainer] onFrame called but viewRef.current is null'
      );
      return;
    }
    viewRef.current.pushFrame(data);
  }, []);
  const { status, errorMessage, retry } = useAndroidMirrorConnection(
    deviceSerial,
    !!workspaceId && !manuallyDisconnected,
    onFrame,
    { maxSize, bitRateKbps, maxFps }
  );
  // Also clears a user-initiated disconnect: while `manuallyDisconnected` is
  // true, `enabled` above is false and `retry()` alone is a no-op (see its
  // early return in `useAndroidMirrorConnection`) — the connection effect
  // only re-runs once this flag flips and the settings scratch refetches.
  const onRetry = useCallback(() => {
    setManuallyDisconnected(false);
    retry();
  }, [setManuallyDisconnected, retry]);

  return (
    <AndroidMirrorView
      ref={viewRef}
      status={status}
      errorMessage={errorMessage}
      onRetry={onRetry}
      className={className}
    />
  );
}
