import { useQuery } from '@tanstack/react-query';
import { androidMirrorApi } from '@/shared/lib/api';

/**
 * `adb`-visible devices for the mirror panel's device picker. Polled while
 * enabled so a device plugged in after the panel was opened shows up
 * without needing a manual refresh.
 */
export function useAndroidMirrorDevices(enabled: boolean) {
  return useQuery({
    queryKey: ['androidMirrorDevices'],
    queryFn: () => androidMirrorApi.listDevices(),
    enabled,
    refetchInterval: enabled ? 5000 : false,
  });
}
