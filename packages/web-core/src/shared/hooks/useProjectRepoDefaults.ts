import { scratchApi, ApiError } from '@/shared/lib/api';
import {
  ScratchType,
  type DraftWorkspaceRepo,
  type ScratchPayload,
} from 'shared/types';

const SCRATCH_TYPE = ScratchType.PROJECT_REPO_DEFAULTS;

/**
 * Read project repo defaults from scratch storage.
 * Returns null if no defaults have been saved for this project.
 */
export async function getProjectRepoDefaults(
  projectId: string
): Promise<DraftWorkspaceRepo[] | null> {
  try {
    const scratch = await scratchApi.get(SCRATCH_TYPE, projectId);
    const payload = scratch.payload as ScratchPayload;
    if (payload?.type === 'PROJECT_REPO_DEFAULTS') {
      return payload.data.repos;
    }
    return null;
  } catch (error) {
    // No defaults saved yet is the expected, common case for a project that
    // hasn't had a workspace created — not an error. The scratch endpoint
    // returns 400 (`crates/server/src/routes/scratch.rs`), not 404, for a
    // missing row.
    if (error instanceof ApiError && error.status === 400) {
      return null;
    }
    console.error('[useProjectRepoDefaults] Failed to read defaults:', error);
    return null;
  }
}

/**
 * Save project repo defaults to scratch storage (upsert).
 */
export async function saveProjectRepoDefaults(
  projectId: string,
  repos: DraftWorkspaceRepo[]
): Promise<void> {
  await scratchApi.update(SCRATCH_TYPE, projectId, {
    payload: {
      type: 'PROJECT_REPO_DEFAULTS',
      data: { repos },
    },
  });
}

/**
 * Read project repo defaults and filter out repos that no longer exist.
 * Returns an empty array if no defaults are saved or all saved repos are stale.
 */
export async function getValidProjectRepoDefaults(
  projectId: string,
  availableRepoIds: Set<string>
): Promise<DraftWorkspaceRepo[]> {
  const defaults = await getProjectRepoDefaults(projectId);
  if (!defaults) {
    return [];
  }
  return defaults.filter((repo) => availableRepoIds.has(repo.repo_id));
}
