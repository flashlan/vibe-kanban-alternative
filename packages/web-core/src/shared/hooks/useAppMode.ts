import { useQuery } from '@tanstack/react-query';
import { handleApiResponse, makeRequest } from '@/shared/lib/api';

export const DEFAULT_AURAPUNK_CLOUD_URL =
  'https://aurapunk-cloud.datapoint.chatgpt.site';

export interface AppModeResponse {
  mode: 'local' | 'cloud';
  cloud: boolean;
  cloud_url: string;
}
async function fetchAppMode(): Promise<AppModeResponse> {
  const response = await makeRequest('/api/app-mode');
  return handleApiResponse<AppModeResponse>(response);
}

/**
 * Read the launch mode from the backend rather than from a build-time flag.
 * This keeps `--cloud` useful for both downloaded binaries and local runs.
 */
export function useIsCloudMode(): boolean {
  const { data } = useQuery({
    queryKey: ['app-mode'],
    queryFn: fetchAppMode,
    staleTime: Infinity,
    retry: false,
  });

  return data?.cloud ?? false;
}

export function useCloudUrl(): string {
  const { data } = useQuery({
    queryKey: ['app-mode'],
    queryFn: fetchAppMode,
    staleTime: Infinity,
    retry: false,
  });

  return data?.cloud_url ?? DEFAULT_AURAPUNK_CLOUD_URL;
}
