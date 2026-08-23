import { useState } from 'react';
import {
  CheckIcon,
  DeviceMobileIcon,
  HouseIcon,
  ArrowLeftIcon,
  SquaresFourIcon,
  StopCircleIcon,
  PlugsIcon,
  PlugsConnectedIcon,
} from '@phosphor-icons/react';
import { cn } from '@/shared/lib/utils';
import { useAndroidMirrorDevices } from '@/shared/hooks/useAndroidMirrorDevices';
import { useAndroidMirrorSettings } from '@/shared/hooks/useAndroidMirrorSettings';
import { androidMirrorApi } from '@/shared/lib/api';
import {
  IconButtonGroup,
  IconButtonGroupItem,
} from '@vibe/ui/components/IconButtonGroup';
import { PrimaryButton } from '@vibe/ui/components/PrimaryButton';

// scrcpy `max_size` presets (longest side, px). `0` = native resolution —
// the sharpest but also the most bandwidth-hungry option by far, since
// encode cost scales roughly with pixel count.
const MAX_SIZE_OPTIONS: Array<{ label: string; value: number }> = [
  { label: 'Native', value: 0 },
  { label: '1280px', value: 1280 },
  { label: '1024px', value: 1024 },
  { label: '720px', value: 720 },
  { label: '480px', value: 480 },
];

// Encoder bitrate presets in kbps. `0` = scrcpy's own default (8 Mbps) —
// the single biggest lever for a choppy stream over a wireless/VPN `adb`
// connection (as opposed to USB), since it directly caps how much encoded
// video has to cross the network per second.
const BIT_RATE_OPTIONS: Array<{ label: string; value: number }> = [
  { label: 'Default (8 Mbps)', value: 0 },
  { label: '4 Mbps', value: 4000 },
  { label: '2 Mbps', value: 2000 },
  { label: '1 Mbps', value: 1000 },
  { label: '500 Kbps', value: 500 },
];

const MAX_FPS_OPTIONS: Array<{ label: string; value: number }> = [
  { label: 'Uncapped', value: 0 },
  { label: '60 fps', value: 60 },
  { label: '30 fps', value: 30 },
  { label: '15 fps', value: 15 },
];

interface AndroidMirrorControlsContainerProps {
  workspaceId: string;
  className?: string;
}

// Reasonable default so the force-stop field isn't empty on first use for
// this project's own app — not persisted (unlike the device pin), just a
// starting point; any package name can be typed in.
const DEFAULT_PACKAGE = 'com.desk.ocrnotes';

/**
 * Device picker + simple device actions for the mirror panel's right-sidebar
 * section — the mirror equivalent of Preview's URL bar living in its own
 * toolbar. Lists `adb`-visible devices and persists the pinned one per
 * workspace (see `useAndroidMirrorSettings`); "Auto" clears the pin
 * (auto-select the only connected device, erroring if there's more than
 * one). Nav actions (Home/Back/Recents) and force-stop are plain `adb shell`
 * commands (`input keyevent` / `am force-stop`) — no scrcpy control-socket
 * protocol involved, so they work independently of whether the video stream
 * itself is currently connected.
 */
export function AndroidMirrorControlsContainer({
  workspaceId,
  className,
}: AndroidMirrorControlsContainerProps) {
  const { data: devices, isLoading } = useAndroidMirrorDevices(true);
  const {
    deviceSerial,
    setDeviceSerial,
    maxSize,
    setMaxSize,
    bitRateKbps,
    setBitRateKbps,
    maxFps,
    setMaxFps,
    manuallyDisconnected,
    setManuallyDisconnected,
  } = useAndroidMirrorSettings(workspaceId);
  const [packageName, setPackageName] = useState(DEFAULT_PACKAGE);
  const [actionError, setActionError] = useState<string | null>(null);
  const [isStopping, setIsStopping] = useState(false);

  const runNavAction = async (action: 'home' | 'back' | 'recents') => {
    setActionError(null);
    try {
      await androidMirrorApi.sendNavAction({
        device_serial: deviceSerial,
        action,
      });
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Action failed');
    }
  };

  const runForceStop = async () => {
    const trimmed = packageName.trim();
    if (!trimmed) return;
    setActionError(null);
    setIsStopping(true);
    try {
      await androidMirrorApi.forceStop({
        device_serial: deviceSerial,
        package: trimmed,
      });
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Force stop failed');
    } finally {
      setIsStopping(false);
    }
  };

  return (
    <div className={cn('flex flex-col gap-double p-base', className)}>
      <div className="flex flex-col gap-half">
        <button
          type="button"
          onClick={() => setDeviceSerial(null)}
          className={cn(
            'flex items-center gap-half rounded-sm px-base py-half text-sm text-left hover:bg-hover',
            deviceSerial === null && 'bg-selected'
          )}
        >
          {deviceSerial === null && <CheckIcon className="size-icon-sm" />}
          <span
            className={cn(deviceSerial !== null && 'ml-[calc(1em+0.5rem)]')}
          >
            Auto (only connected device)
          </span>
        </button>

        {isLoading && (
          <p className="px-base py-half text-xs text-low">
            Looking for devices…
          </p>
        )}

        {!isLoading && devices?.length === 0 && (
          <p className="px-base py-half text-xs text-low">
            No devices found via adb.
          </p>
        )}

        {devices?.map((device) => (
          <button
            key={device.serial}
            type="button"
            onClick={() => setDeviceSerial(device.serial)}
            className={cn(
              'flex items-center gap-half rounded-sm px-base py-half text-sm text-left hover:bg-hover',
              deviceSerial === device.serial && 'bg-selected'
            )}
          >
            {deviceSerial === device.serial ? (
              <CheckIcon className="size-icon-sm shrink-0" />
            ) : (
              <DeviceMobileIcon className="size-icon-sm shrink-0" />
            )}
            <span className="truncate font-mono text-xs">{device.serial}</span>
            {device.state !== 'device' && (
              <span className="text-low text-xs">({device.state})</span>
            )}
          </button>
        ))}
      </div>

      <div className="flex flex-col gap-half border-t border-border pt-double">
        <h4 className="px-base text-xs font-medium text-low">Controls</h4>
        <div className="px-base">
          <IconButtonGroup>
            <IconButtonGroupItem
              icon={ArrowLeftIcon}
              onClick={() => void runNavAction('back')}
              title="Back"
              aria-label="Back"
            />
            <IconButtonGroupItem
              icon={HouseIcon}
              onClick={() => void runNavAction('home')}
              title="Home"
              aria-label="Home"
            />
            <IconButtonGroupItem
              icon={SquaresFourIcon}
              onClick={() => void runNavAction('recents')}
              title="Recent apps"
              aria-label="Recent apps"
            />
          </IconButtonGroup>
        </div>

        <div className="px-base">
          <PrimaryButton
            variant="tertiary"
            value={manuallyDisconnected ? 'Connect' : 'Disconnect'}
            actionIcon={manuallyDisconnected ? PlugsConnectedIcon : PlugsIcon}
            onClick={() => setManuallyDisconnected(!manuallyDisconnected)}
            className="w-full"
          />
        </div>

        <div className="flex flex-col gap-half px-base">
          <label className="text-xs text-low" htmlFor="mirror-force-stop-pkg">
            Force stop app (package name)
          </label>
          <div className="flex gap-half">
            <input
              id="mirror-force-stop-pkg"
              type="text"
              value={packageName}
              onChange={(e) => setPackageName(e.target.value)}
              placeholder="com.example.app"
              className="flex-1 min-w-0 rounded-sm border border-border bg-primary px-base py-half font-mono text-xs"
            />
            <PrimaryButton
              variant="tertiary"
              value="Stop"
              actionIcon={StopCircleIcon}
              disabled={isStopping || !packageName.trim()}
              onClick={() => void runForceStop()}
            />
          </div>
        </div>

        {actionError && (
          <p className="px-base text-xs text-red-500">{actionError}</p>
        )}
      </div>

      <div className="flex flex-col gap-half border-t border-border pt-double">
        <h4
          className="px-base text-xs font-medium text-low"
          title="Lower these if the stream stutters over Wi-Fi/remote adb. Takes effect on the next (re)connect."
        >
          Quality
        </h4>

        <div className="flex items-center justify-between gap-half px-base">
          <label
            className="shrink-0 text-xs text-low"
            htmlFor="mirror-max-size"
          >
            Resolution
          </label>
          <select
            id="mirror-max-size"
            value={maxSize ?? 0}
            onChange={(e) => {
              const v = Number(e.target.value);
              setMaxSize(v === 0 ? null : v);
            }}
            className="min-w-0 flex-1 rounded-sm border border-border bg-primary px-half py-half text-xs"
          >
            {MAX_SIZE_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center justify-between gap-half px-base">
          <label
            className="shrink-0 text-xs text-low"
            htmlFor="mirror-bit-rate"
          >
            Bitrate
          </label>
          <select
            id="mirror-bit-rate"
            value={bitRateKbps ?? 0}
            onChange={(e) => {
              const v = Number(e.target.value);
              setBitRateKbps(v === 0 ? null : v);
            }}
            className="min-w-0 flex-1 rounded-sm border border-border bg-primary px-half py-half text-xs"
          >
            {BIT_RATE_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center justify-between gap-half px-base">
          <label className="shrink-0 text-xs text-low" htmlFor="mirror-max-fps">
            Frame rate
          </label>
          <select
            id="mirror-max-fps"
            value={maxFps ?? 0}
            onChange={(e) => {
              const v = Number(e.target.value);
              setMaxFps(v === 0 ? null : v);
            }}
            className="min-w-0 flex-1 rounded-sm border border-border bg-primary px-half py-half text-xs"
          >
            {MAX_FPS_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>
      </div>
    </div>
  );
}
