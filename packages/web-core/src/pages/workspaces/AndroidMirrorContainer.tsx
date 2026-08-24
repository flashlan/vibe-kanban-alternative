import { useCallback, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import {
  ArrowLeftIcon,
  ArrowsInSimpleIcon,
  HouseIcon,
  SquaresFourIcon,
} from '@phosphor-icons/react';
import { cn } from '@/shared/lib/utils';
import {
  AndroidMirrorView,
  type AndroidMirrorViewHandle,
  type AndroidMirrorInputMessage,
} from '@vibe/ui/components/AndroidMirrorView';
import {
  IconButtonGroup,
  IconButtonGroupItem,
} from '@vibe/ui/components/IconButtonGroup';
import { PrimaryButton } from '@vibe/ui/components/PrimaryButton';
import { useAndroidMirrorSettings } from '@/shared/hooks/useAndroidMirrorSettings';
import { useAndroidMirrorConnection } from '@/shared/hooks/useAndroidMirrorConnection';
import { useDocumentPictureInPicture } from '@/shared/hooks/useDocumentPictureInPicture';
import { androidMirrorApi } from '@/shared/lib/api';

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
 *
 * "Detach" moves the mirror into a Document Picture-in-Picture window (an
 * always-on-top window separate from this tab). This component always
 * renders exactly one `createPortal(...)` at a fixed position in its own
 * tree; only *which* DOM node that portal targets changes (an inline
 * placeholder `<div>` here vs. the floating window's `<body>`). That's
 * deliberate: conditionally swapping between "render `<AndroidMirrorView>`
 * directly" and "render it inside a portal" — or giving it a different
 * sibling index each time — reads as a *different* element to React's
 * reconciler and remounts it, tearing down its `VideoDecoder` mid-stream.
 * scrcpy only sends one SPS/PPS config packet up front (no periodic
 * resend without a control-channel keyframe request, which v1 doesn't use)
 * — a remounted decoder would have nothing left to configure itself with
 * and the video would go blank until a full reconnect. Keeping the portal's
 * target as the only thing that changes avoids that entirely.
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
  const { status, errorMessage, retry, send } = useAndroidMirrorConnection(
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
  const onInput = useCallback(
    (msg: AndroidMirrorInputMessage) => send(msg),
    [send]
  );

  const {
    isSupported: pipSupported,
    pipWindow,
    open: openPip,
    close: closePip,
  } = useDocumentPictureInPicture();
  const isDetached = !!pipWindow;
  const onDetach = useCallback(() => {
    void openPip({ width: 360, height: 720 });
  }, [openPip]);

  const runNavAction = (action: 'home' | 'back' | 'recents') => {
    void androidMirrorApi
      .sendNavAction({ device_serial: deviceSerial, action })
      .catch((e) => console.error('[AndroidMirrorContainer] nav failed:', e));
  };

  // The portal's target when *not* detached — an otherwise-empty placeholder
  // sized by the caller's `className` (the actual layout slot within
  // WorkspacesLayout). A plain ref isn't enough here: it's `null` on the
  // very first render (before the DOM node exists to portal into), and a
  // callback ref assigned via `useState`'s setter is the standard way to
  // get a re-render once the node is actually attached.
  const [inlineContainer, setInlineContainer] = useState<HTMLDivElement | null>(
    null
  );
  const targetContainer = isDetached
    ? pipWindow.document.body
    : inlineContainer;

  return (
    <>
      <div
        ref={setInlineContainer}
        className={cn('h-full w-full', className, isDetached && 'hidden')}
      />
      {targetContainer &&
        createPortal(
          <div className="flex h-full w-full flex-col bg-primary">
            <div
              className={cn(
                'flex items-center justify-between gap-half border-b border-border p-half',
                !isDetached && 'hidden'
              )}
            >
              <IconButtonGroup>
                <IconButtonGroupItem
                  icon={ArrowLeftIcon}
                  onClick={() => runNavAction('back')}
                  title="Back"
                  aria-label="Back"
                />
                <IconButtonGroupItem
                  icon={HouseIcon}
                  onClick={() => runNavAction('home')}
                  title="Home"
                  aria-label="Home"
                />
                <IconButtonGroupItem
                  icon={SquaresFourIcon}
                  onClick={() => runNavAction('recents')}
                  title="Recent apps"
                  aria-label="Recent apps"
                />
              </IconButtonGroup>
              <PrimaryButton
                variant="tertiary"
                value="Reattach"
                actionIcon={ArrowsInSimpleIcon}
                onClick={closePip}
              />
            </div>
            <AndroidMirrorView
              ref={viewRef}
              status={status}
              errorMessage={errorMessage}
              onRetry={onRetry}
              className="min-h-0 flex-1"
              onDetach={!isDetached && pipSupported ? onDetach : undefined}
              onInput={onInput}
            />
          </div>,
          targetContainer
        )}
    </>
  );
}
