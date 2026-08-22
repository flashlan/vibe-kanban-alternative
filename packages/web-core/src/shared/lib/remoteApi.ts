import type {
  UpdateIssueRequest,
  UpdateProjectRequest,
  UpdateProjectStatusRequest,
} from 'shared/remote-types';

const REMOTE_API_BASE = '';

export const makeRequest = async (
  path: string,
  options: RequestInit = {},
  _retryOn401 = true
): Promise<Response> => {
  return makeAuthenticatedRequest(REMOTE_API_BASE, path, options);
};

async function makeAuthenticatedRequest(
  baseUrl: string,
  path: string,
  options: RequestInit = {}
): Promise<Response> {
  const headers = new Headers(options.headers ?? {});
  if (!headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }
  headers.set('X-Client-Version', __APP_VERSION__);
  headers.set('X-Client-Type', 'frontend');

  return fetch(`${baseUrl}${path}`, {
    ...options,
    headers,
    credentials: 'include',
  });
}

export interface BulkUpdateIssueItem {
  id: string;
  changes: Partial<UpdateIssueRequest>;
}

export interface BulkUpdateProjectItem {
  id: string;
  changes: Partial<UpdateProjectRequest>;
}

export async function bulkUpdateProjects(
  updates: BulkUpdateProjectItem[]
): Promise<void> {
  const response = await makeRequest('/v1/projects/bulk', {
    method: 'POST',
    body: JSON.stringify({
      updates: updates.map((u) => ({ id: u.id, ...u.changes })),
    }),
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message || 'Failed to bulk update projects');
  }
}

export async function bulkUpdateIssues(
  updates: BulkUpdateIssueItem[]
): Promise<void> {
  const response = await makeRequest('/v1/issues/bulk', {
    method: 'POST',
    body: JSON.stringify({
      updates: updates.map((u) => ({ id: u.id, ...u.changes })),
    }),
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message || 'Failed to bulk update issues');
  }
}

/**
 * Delete an issue. `cleanupWorkspaces` also removes the on-disk worktree
 * dirs/branches of the issue's linked workspaces — otherwise they're left
 * as orphaned dirs (deleting an issue only cascades the issue<->workspace
 * link row, not the workspace row or its files). Mirrors the project
 * delete's `cleanup_workspaces` query param.
 */
export async function deleteIssue(
  id: string,
  options?: { cleanupWorkspaces?: boolean }
): Promise<void> {
  const query = options?.cleanupWorkspaces ? '?cleanup_workspaces=true' : '';
  const response = await makeRequest(`/v1/issues/${id}${query}`, {
    method: 'DELETE',
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message || 'Failed to delete issue');
  }
}

/** Soft-delete an issue: hide it from the active board but keep it in the
 *  database so it can be recovered from the archive view. */
export async function archiveIssue(id: string): Promise<void> {
  const response = await makeRequest(`/v1/issues/${id}/archive`, {
    method: 'POST',
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message || 'Failed to archive issue');
  }
}

/** Restore a previously archived issue back to the active board. */
export async function restoreIssue(id: string): Promise<void> {
  const response = await makeRequest(`/v1/issues/${id}/restore`, {
    method: 'POST',
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message || 'Failed to restore issue');
  }
}

/** List archived issues for a project (for the archive recovery view). */
export async function listArchivedIssues(
  projectId: string
): Promise<Array<{ id: string } & Record<string, unknown>>> {
  const response = await makeRequest(
    `/v1/issues/archived?project_id=${encodeURIComponent(projectId)}`,
    { method: 'GET' }
  );
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message || 'Failed to list archived issues');
  }
  const body = (await response.json()) as {
    success: boolean;
    data: { issues: Array<{ id: string } & Record<string, unknown>> };
  };
  return body.data.issues;
}

export interface BulkUpdateProjectStatusItem {
  id: string;
  changes: Partial<UpdateProjectStatusRequest>;
}

export async function bulkUpdateProjectStatuses(
  updates: BulkUpdateProjectStatusItem[]
): Promise<void> {
  const response = await makeRequest('/v1/project_statuses/bulk', {
    method: 'POST',
    body: JSON.stringify({
      updates: updates.map((u) => ({ id: u.id, ...u.changes })),
    }),
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.message || 'Failed to bulk update project statuses');
  }
}
