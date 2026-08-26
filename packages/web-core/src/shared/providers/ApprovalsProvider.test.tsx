// @vitest-environment jsdom

import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ApprovalsProvider, useApprovals } from './ApprovalsProvider';

const useJsonPatchWsStream = vi.hoisted(() => vi.fn());

vi.mock('@/shared/hooks/useJsonPatchWsStream', () => ({
  useJsonPatchWsStream,
}));

function ApprovalConsumer() {
  const { isConnected } = useApprovals();
  return <span>{isConnected ? 'connected' : 'disconnected'}</span>;
}

describe('ApprovalsProvider', () => {
  it('shares one approval stream across all consumers', () => {
    useJsonPatchWsStream.mockReturnValue({
      data: { pending: {} },
      isConnected: true,
    });

    render(
      <ApprovalsProvider>
        <ApprovalConsumer />
        <ApprovalConsumer />
      </ApprovalsProvider>
    );

    expect(useJsonPatchWsStream).toHaveBeenCalledTimes(1);
    expect(useJsonPatchWsStream).toHaveBeenCalledWith(
      '/api/approvals/stream/ws',
      true,
      expect.any(Function)
    );
  });
});
