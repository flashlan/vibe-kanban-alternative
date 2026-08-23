import { useCallback, useEffect, useRef, useState } from 'react';

interface UseDocumentPictureInPictureResult {
  isSupported: boolean;
  pipWindow: Window | null;
  open: (options?: DocumentPictureInPictureOptions) => Promise<void>;
  close: () => void;
}

/**
 * Wraps the Document Picture-in-Picture API (Chrome/Edge 116+) — an
 * always-on-top window, separate from the tab, that can host arbitrary DOM
 * (not just a `<video>`, unlike classic PiP). Used to "detach" the Android
 * mirror into a floating window, the way Android Studio's device panel can.
 *
 * The returned `pipWindow` is meant to be used as a React portal target
 * (`createPortal(..., pipWindow.document.body)`) rather than having callers
 * move existing DOM nodes into it directly — a portal keeps the moved
 * subtree under React's own reconciliation, avoiding "insertBefore on a
 * node React no longer expects" errors from manual DOM surgery on a node
 * React still thinks lives in the original document.
 */
export function useDocumentPictureInPicture(): UseDocumentPictureInPictureResult {
  const [pipWindow, setPipWindow] = useState<Window | null>(null);
  const isSupported =
    typeof window !== 'undefined' && !!window.documentPictureInPicture;

  const open = useCallback(
    async (options?: DocumentPictureInPictureOptions) => {
      if (!window.documentPictureInPicture) return;
      const win = await window.documentPictureInPicture.requestWindow(options);

      // The PiP window starts with a blank document — no stylesheets, no
      // theme class/attributes — copy both over once at open time so
      // portaled content renders with the app's actual styling instead of
      // unstyled HTML. A one-time copy is enough here: the app's CSS isn't
      // expected to change while a session is open.
      win.document.documentElement.className =
        document.documentElement.className;
      Object.entries(document.documentElement.dataset).forEach(
        ([key, value]) => {
          if (value !== undefined) {
            win.document.documentElement.dataset[key] = value;
          }
        }
      );
      document
        .querySelectorAll('style, link[rel="stylesheet"]')
        .forEach((node) => {
          win.document.head.appendChild(node.cloneNode(true));
        });
      // The app's own `height: 100%` chain (html/body -> #root -> ...) is
      // scoped through `#root`, which doesn't exist in this window — content
      // is portaled straight into `body`, so without this the chain never
      // resolves and anything relying on `h-full` collapses to its content
      // size instead of filling the window (confirmed live: the mirror's
      // video area only filled a sliver at the top, the rest just empty).
      win.document.documentElement.style.height = '100%';
      win.document.body.style.height = '100%';
      win.document.body.style.margin = '0';

      // Fires on both an explicit `close()` and the user closing the
      // floating window via its own chrome — either way this is the single
      // signal to fall back to "reattached" state.
      win.addEventListener('pagehide', () => {
        setPipWindow(null);
      });

      setPipWindow(win);
    },
    []
  );

  const close = useCallback(() => {
    pipWindow?.close();
  }, [pipWindow]);

  // If the owning component unmounts while still detached (e.g. the user
  // navigates away from this workspace entirely), close the floating window
  // rather than leaving an orphaned one with no "reattach" control left
  // anywhere to bring it back.
  const pipWindowRef = useRef(pipWindow);
  pipWindowRef.current = pipWindow;
  useEffect(() => {
    return () => {
      pipWindowRef.current?.close();
    };
  }, []);

  return { isSupported, pipWindow, open, close };
}
