import { useQuery } from '@tanstack/react-query';
import { CircleNotchIcon, UsersThreeIcon } from '@phosphor-icons/react';
import { workspacesApi, type AgentActivity } from '@/shared/lib/api';
import { cn } from '@/shared/lib/utils';

interface AgentWorkPanelProps {
  workspaceId?: string;
}

function formatLease(expiresAt: string | null): string {
  if (!expiresAt) return 'No declaration';
  const remaining = Math.max(
    0,
    Math.round((new Date(expiresAt).getTime() - Date.now()) / 1000)
  );
  if (remaining < 60) return `${remaining}s remaining`;
  return `${Math.ceil(remaining / 60)}min remaining`;
}

function DeclarationRow({ activity }: { activity: AgentActivity }) {
  return (
    <div className="rounded-md border bg-primary px-2 py-2 text-xs">
      <div className="flex items-center gap-1.5">
        <span
          className={cn(
            'size-1.5 rounded-full',
            activity.is_running ? 'bg-success' : 'bg-warning'
          )}
        />
        <span className="truncate font-medium text-normal">
          {activity.agent_name}
        </span>
        <span className="ml-auto shrink-0 text-low">
          {activity.is_running ? 'Running' : 'Declared'} ·{' '}
          {formatLease(activity.lease_expires_at)}
        </span>
      </div>
      <p className="mt-1 text-low leading-4">{activity.intent}</p>
      {activity.files.length > 0 && (
        <div className="mt-1.5 flex flex-wrap gap-1">
          {activity.files.slice(0, 5).map((file) => (
            <code
              key={file}
              className="max-w-full truncate rounded bg-secondary px-1 text-[10px] text-low"
            >
              {file}
            </code>
          ))}
          {activity.files.length > 5 && (
            <span className="text-low">+{activity.files.length - 5}</span>
          )}
        </div>
      )}
      {activity.symbols.length > 0 && (
        <p className="mt-1 truncate font-mono text-[10px] text-low">
          {activity.symbols.slice(0, 3).join(' · ')}
          {activity.symbols.length > 3 ? ' · …' : ''}
        </p>
      )}
      {activity.dependencies.length > 0 && (
        <p className="mt-1 truncate text-[10px] text-low">
          Depends on: {activity.dependencies.slice(0, 3).join(' · ')}
          {activity.dependencies.length > 3 ? ' · …' : ''}
        </p>
      )}
    </div>
  );
}

export function AgentWorkPanel({ workspaceId }: AgentWorkPanelProps) {
  const {
    data = [],
    isLoading,
    isError,
  } = useQuery({
    queryKey: ['agent-work', workspaceId],
    queryFn: () => workspacesApi.listAgentWork(workspaceId!),
    enabled: !!workspaceId,
    refetchInterval: 3000,
    refetchOnWindowFocus: false,
  });
  const runningCount = data.filter((activity) => activity.is_running).length;

  return (
    <div className="border-b bg-secondary px-base py-2">
      <div className="mb-2 flex items-center gap-1.5 text-xs font-medium text-normal">
        <UsersThreeIcon className="size-icon-sm text-low" />
        <span>Active agents</span>
        {runningCount > 0 && (
          <span className="rounded-full bg-tertiary px-1.5 text-[10px] text-low">
            {runningCount}
          </span>
        )}
        {isLoading && (
          <CircleNotchIcon className="ml-auto size-icon-sm animate-spin text-low" />
        )}
      </div>
      {!workspaceId ? (
        <p className="text-xs text-low">No workspace selected.</p>
      ) : isError ? (
        <p className="text-xs text-low">Unable to load agent activity.</p>
      ) : data.length === 0 ? (
        <p className="text-xs text-low">No agent activity detected.</p>
      ) : (
        <div
          className={cn(
            'space-y-1.5',
            data.length > 3 && 'max-h-48 overflow-y-auto'
          )}
        >
          {data.map((activity) => (
            <DeclarationRow
              key={activity.declaration_id ?? activity.execution_process_id}
              activity={activity}
            />
          ))}
        </div>
      )}
    </div>
  );
}
