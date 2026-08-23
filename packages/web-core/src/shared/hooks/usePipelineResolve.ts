import { useQuery } from '@tanstack/react-query';
import { workspacesApi } from '@/shared/lib/api';

/**
 * Server-resolved pipeline for a workspace's linked card — the same source
 * of truth the `get_pipeline` MCP tool reads (`GET
 * /api/workspaces/{id}/pipeline/resolve`, driven by `extension_metadata`,
 * not the card description text). Used to render live "stage N of M"
 * progress for cards whose description now carries only the compact
 * `get_pipeline` pointer (see `cardPipeline.ts`), which no longer has a
 * numbered stage list for `parsePipelineStages` to parse.
 */
export function usePipelineResolve(workspaceId?: string) {
  return useQuery({
    queryKey: ['pipelineResolve', workspaceId],
    queryFn: () => workspacesApi.resolvePipeline(workspaceId!),
    enabled: !!workspaceId,
  });
}
