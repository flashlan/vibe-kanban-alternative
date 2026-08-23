import { useCallback, useRef } from 'react';
import { useScratch } from '@/shared/hooks/useScratch';
import { ScratchType, type ScratchPayload } from 'shared/types';

interface MirrorSettingsData {
  device_serial: string | null;
  max_size: number | null;
  bit_rate_kbps: number | null;
  max_fps: number | null;
  manually_disconnected: boolean;
}

const DEFAULT_DATA: MirrorSettingsData = {
  device_serial: null,
  max_size: null,
  bit_rate_kbps: null,
  max_fps: null,
  manually_disconnected: false,
};

interface UseAndroidMirrorSettingsResult {
  /** `null` = auto-select the only connected device. */
  deviceSerial: string | null;
  setDeviceSerial: (serial: string | null) => void;
  /** scrcpy `max_size` (longest side, px). `null` = native resolution. */
  maxSize: number | null;
  setMaxSize: (size: number | null) => void;
  /** Encoder bitrate in kbps. `null` = scrcpy's default (8 Mbps). */
  bitRateKbps: number | null;
  setBitRateKbps: (kbps: number | null) => void;
  /** scrcpy `max_fps`. `null` = uncapped. */
  maxFps: number | null;
  setMaxFps: (fps: number | null) => void;
  /** User-initiated disconnect, persisted until they hit "Connect" again. */
  manuallyDisconnected: boolean;
  setManuallyDisconnected: (disconnected: boolean) => void;
  isLoading: boolean;
}

/**
 * Per-workspace mirror settings (pinned device, encoder tuning, manual
 * connect/disconnect), persisted the same way Preview's
 * `overrideUrl`/`screenSize` are (see `usePreviewSettings`) — via the
 * scratch system, keyed by workspace id.
 */
export function useAndroidMirrorSettings(
  workspaceId: string | undefined
): UseAndroidMirrorSettingsResult {
  const enabled = !!workspaceId;

  const { scratch, updateScratch, isLoading } = useScratch(
    ScratchType.ANDROID_MIRROR_SETTINGS,
    workspaceId ?? '',
    {
      enabled,
    }
  );

  const payload = scratch?.payload as ScratchPayload | undefined;
  const data: MirrorSettingsData =
    payload?.type === 'ANDROID_MIRROR_SETTINGS'
      ? { ...DEFAULT_DATA, ...payload.data }
      : DEFAULT_DATA;

  // Two (or more) setters called back-to-back — e.g. changing resolution
  // then bitrate before the first PUT's refetch has landed — would each
  // build their patch by merging against this render's `data`, which is
  // stale for the second call. That silently reverts whatever the first
  // call just changed once its payload overwrites the scratch row. Tracking
  // in-flight saves lets `latestDataRef` chain patches onto each other while
  // any are outstanding, and only trust fresh server `data` again once
  // they've all settled (and the query has had a chance to refetch it).
  const inFlightRef = useRef(0);
  const latestDataRef = useRef(data);
  if (inFlightRef.current === 0) {
    latestDataRef.current = data;
  }

  const save = useCallback(
    (patch: Partial<MirrorSettingsData>) => {
      if (!workspaceId) return;
      const merged = { ...latestDataRef.current, ...patch };
      latestDataRef.current = merged;
      inFlightRef.current += 1;
      updateScratch({
        payload: { type: 'ANDROID_MIRROR_SETTINGS', data: merged },
      })
        .catch((e) => {
          console.error('[useAndroidMirrorSettings] Failed to save:', e);
        })
        .finally(() => {
          inFlightRef.current -= 1;
        });
    },
    [workspaceId, updateScratch]
  );

  const setDeviceSerial = useCallback(
    (serial: string | null) => save({ device_serial: serial }),
    [save]
  );
  const setMaxSize = useCallback(
    (size: number | null) => save({ max_size: size }),
    [save]
  );
  const setBitRateKbps = useCallback(
    (kbps: number | null) => save({ bit_rate_kbps: kbps }),
    [save]
  );
  const setMaxFps = useCallback(
    (fps: number | null) => save({ max_fps: fps }),
    [save]
  );
  const setManuallyDisconnected = useCallback(
    (disconnected: boolean) => save({ manually_disconnected: disconnected }),
    [save]
  );

  return {
    deviceSerial: data.device_serial,
    setDeviceSerial,
    maxSize: data.max_size,
    setMaxSize,
    bitRateKbps: data.bit_rate_kbps,
    setBitRateKbps,
    maxFps: data.max_fps,
    setMaxFps,
    manuallyDisconnected: data.manually_disconnected,
    setManuallyDisconnected,
    isLoading,
  };
}
