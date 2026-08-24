import { useQuery } from '@tanstack/react-query';
import { androidMirrorApi } from '@/shared/lib/api';

/**
 * Locally-defined AVDs (`emulator -list-avds`) for the "launch emulator"
 * picker. Unlike `useAndroidMirrorDevices`, this doesn't poll — the set of
 * defined AVDs essentially never changes while the panel is open.
 */
export function useAndroidMirrorAvds(enabled: boolean) {
  return useQuery({
    queryKey: ['androidMirrorAvds'],
    queryFn: () => androidMirrorApi.listAvds(),
    enabled,
  });
}
