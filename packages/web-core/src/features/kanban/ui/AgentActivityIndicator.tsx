import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  ArrowSquareOutIcon,
  CircleNotchIcon,
  GitBranchIcon,
  ShieldCheckIcon,
  UsersThreeIcon,
} from '@phosphor-icons/react';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@vibe/ui/components/Popover';
import { useAppNavigation } from '@/shared/hooks/useAppNavigation';
import { useWorkspaceContext } from '@/shared/hooks/useWorkspaceContext';
import { workspacesApi, type AgentActivity } from '@/shared/lib/api';
import { cn } from '@/shared/lib/utils';

interface AgentActivityIndicatorProps {
  projectId: string;
}

function formatRemaining(expiresAt: string | null): string {
  if (!expiresAt) return 'No declaration';
  const remaining = Math.max(
    0,
    Math.round((new Date(expiresAt).getTime() - Date.now()) / 1000)
  );
  if (remaining < 60) return `${remaining}s remaining`;
  return `${Math.ceil(remaining / 60)}min remaining`;
}

function hasOverlap(left: AgentActivity, right: AgentActivity) {
  const overlaps = (values: string[], otherValues: string[]) =>
    values.some((value) => otherValues.includes(value));

  return (
    overlaps(left.files, right.files) ||
    overlaps(left.symbols, right.symbols) ||
    overlaps(left.dependencies, right.symbols) ||
    overlaps(right.dependencies, left.symbols)
  );
}

function ActivityRow({
  activity,
  workspaceName,
  branch,
  onOpen,
}: {
  activity: AgentActivity;
  workspaceName: string;
  branch: string;
  onOpen: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onOpen}
      className="w-full rounded-sm border border-border bg-primary p-half text-left transition-colors hover:border-brand/60 hover:bg-secondary"
    >
      <div className="flex items-center gap-half">
        <span
          className={cn(
            'size-1.5 shrink-0 rounded-full',
            activity.is_running ? 'bg-success' : 'bg-warning'
          )}
        />
        <span className="min-w-0 flex-1 truncate text-xs font-medium text-normal">
          {activity.agent_name}
        </span>
        <ArrowSquareOutIcon className="size-icon-xs shrink-0 text-low" />
      </div>
      <p className="mt-quarter truncate text-[11px] text-low">
        {workspaceName} · {activity.is_running ? 'Running' : 'Declared'}
      </p>
      <div className="mt-quarter flex items-center gap-quarter text-[10px] text-low">
        <GitBranchIcon className="size-icon-xs" />
        <span className="truncate">{branch}</span>
        <span className="ml-auto shrink-0">
          {formatRemaining(activity.lease_expires_at)}
        </span>
      </div>
      <p className="mt-quarter truncate text-[11px] text-low">
        {activity.intent}
      </p>
    </button>
  );
}

export function AgentActivityIndicator({
  projectId,
}: AgentActivityIndicatorProps) {
  const appNavigation = useAppNavigation();
  const { activeWorkspaces } = useWorkspaceContext();
  const {
    data = [],
    isLoading,
    isError,
  } = useQuery({
    queryKey: ['agent-work', 'project', projectId],
    queryFn: () => workspacesApi.listAgentWorkForProject(projectId),
    refetchInterval: 3000,
    refetchOnWindowFocus: false,
  });

  const workspaceById = useMemo(
    () =>
      new Map(activeWorkspaces.map((workspace) => [workspace.id, workspace])),
    [activeWorkspaces]
  );
  const runningCount = useMemo(
    () => data.filter((activity) => activity.is_running).length,
    [data]
  );
  const declaredOnlyCount = data.length - runningCount;
  const overlapCount = useMemo(() => {
    let count = 0;
    for (let index = 0; index < data.length; index += 1) {
      for (
        let otherIndex = index + 1;
        otherIndex < data.length;
        otherIndex += 1
      ) {
        if (hasOverlap(data[index], data[otherIndex])) count += 1;
      }
    }
    return count;
  }, [data]);

  const openWorkspace = (workspaceId: string) => {
    appNavigation.goToWorkspace(workspaceId);
  };

  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          className={cn(
            'flex min-w-0 items-center gap-half rounded-sm border px-base py-half text-left transition-colors',
            runningCount > 0
              ? 'border-success/40 bg-success/5 hover:border-success/70'
              : 'border-border bg-secondary hover:border-brand/50'
          )}
          aria-label="View active agents"
        >
          {isLoading ? (
            <CircleNotchIcon className="size-icon-sm animate-spin text-low" />
          ) : (
            <ShieldCheckIcon
              className={cn(
                'size-icon-sm',
                runningCount > 0 ? 'text-success' : 'text-low'
              )}
              weight="fill"
            />
          )}
          <span className="min-w-0">
            <span className="block truncate text-xs font-medium text-normal">
              Agent Activity
            </span>
            <span className="flex items-center gap-quarter text-[10px] text-low">
              <UsersThreeIcon className="size-icon-xs" />
              {isError
                ? 'Unable to load activity'
                : `${runningCount} running agent${runningCount === 1 ? '' : 's'}`}
              {declaredOnlyCount > 0 && (
                <span> · {declaredOnlyCount} declared</span>
              )}
              {overlapCount > 0 && (
                <span className="text-warning">
                  · {overlapCount} overlap warning
                  {overlapCount === 1 ? '' : 's'}
                </span>
              )}
            </span>
          </span>
        </button>
      </PopoverTrigger>
      <PopoverContent side="bottom" align="end" className="w-80">
        <div className="space-y-base">
          <div className="flex items-center justify-between">
            <div>
              <h4 className="text-sm font-medium text-normal">
                Agent Activity
              </h4>
              <p className="text-[11px] text-low">Across this project</p>
            </div>
            <span className="rounded-full bg-secondary px-half text-[10px] text-low">
              {runningCount}
            </span>
          </div>
          {data.length === 0 ? (
            <p className="text-xs text-low">No agent activity detected.</p>
          ) : (
            <div className="max-h-64 space-y-half overflow-y-auto">
              {data.map((activity) => {
                const workspace = workspaceById.get(activity.workspace_id);
                return (
                  <ActivityRow
                    key={
                      activity.declaration_id ?? activity.execution_process_id
                    }
                    activity={activity}
                    workspaceName={workspace?.name ?? 'Workspace'}
                    branch={workspace?.branch ?? 'Unknown branch'}
                    onOpen={() => openWorkspace(activity.workspace_id)}
                  />
                );
              })}
            </div>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}
