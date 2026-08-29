import { useQuery } from '@tanstack/react-query';
import { handleApiResponse, makeRequest } from '@/shared/lib/api';

interface AppModeResponse {
  mode: 'local' | 'cloud';
  cloud: boolean;
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
