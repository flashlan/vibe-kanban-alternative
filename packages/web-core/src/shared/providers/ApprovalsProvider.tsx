import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  type ReactNode,
} from 'react';
import type { ApprovalInfo } from 'shared/types';
import { useJsonPatchWsStream } from '@/shared/hooks/useJsonPatchWsStream';

export interface ApprovalsContextValue {
  pendingApprovals: ApprovalInfo[];
  getPendingForProcess: (executionProcessId: string) => ApprovalInfo | null;
  getPendingById: (approvalId: string) => ApprovalInfo | null;
  isConnected: boolean;
}

type ApprovalState = {
  pending: Record<string, ApprovalInfo>;
};

const ApprovalsContext = createContext<ApprovalsContextValue | null>(null);

export function ApprovalsProvider({ children }: { children: ReactNode }) {
  const initialData = useCallback<() => ApprovalState>(
    () => ({ pending: {} }),
    []
  );
  const { data, isConnected } = useJsonPatchWsStream<ApprovalState>(
    '/api/approvals/stream/ws',
    true,
    initialData
  );

  const pendingById = useMemo(() => data?.pending ?? {}, [data?.pending]);
  const pendingApprovals = useMemo(
    () => Object.values(pendingById),
    [pendingById]
  );

  const getPendingForProcess = useCallback(
    (executionProcessId: string): ApprovalInfo | null => {
      for (const info of pendingApprovals) {
        if (info.execution_process_id === executionProcessId) {
          return info;
        }
      }
      return null;
    },
    [pendingApprovals]
  );

  const getPendingById = useCallback(
    (approvalId: string): ApprovalInfo | null => {
      return pendingById[approvalId] ?? null;
    },
    [pendingById]
  );

  const value = useMemo<ApprovalsContextValue>(
    () => ({
      pendingApprovals,
      getPendingForProcess,
      getPendingById,
      isConnected,
    }),
    [pendingApprovals, getPendingForProcess, getPendingById, isConnected]
  );

  return (
    <ApprovalsContext.Provider value={value}>
      {children}
    </ApprovalsContext.Provider>
  );
}

export function useApprovals(): ApprovalsContextValue {
  const context = useContext(ApprovalsContext);
  if (!context) {
    throw new Error('useApprovals must be used within ApprovalsProvider');
  }
  return context;
}
