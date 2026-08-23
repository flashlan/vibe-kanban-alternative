// Document Picture-in-Picture API (Chrome/Edge 116+) — not yet in TS's
// bundled DOM lib. Shape matches the spec:
// https://wicg.github.io/document-picture-in-picture/
export {};

declare global {
  interface DocumentPictureInPictureOptions {
    width?: number;
    height?: number;
    disallowReturnToOpener?: boolean;
    preferInitialWindowPlacement?: boolean;
  }

  interface DocumentPictureInPictureEvent extends Event {
    readonly window: Window;
  }

  interface DocumentPictureInPicture extends EventTarget {
    requestWindow(options?: DocumentPictureInPictureOptions): Promise<Window>;
    readonly window: Window | null;
    onenter:
      | ((
          this: DocumentPictureInPicture,
          ev: DocumentPictureInPictureEvent
        ) => unknown)
      | null;
  }

  interface Window {
    documentPictureInPicture?: DocumentPictureInPicture;
  }
}
