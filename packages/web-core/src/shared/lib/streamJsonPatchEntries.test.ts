// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { openLocalApiWebSocket } from '@/shared/lib/localApiTransport';
import { streamJsonPatchEntries } from './streamJsonPatchEntries';

vi.mock('@/shared/lib/localApiTransport', () => ({
  openLocalApiWebSocket: vi.fn(),
}));

type FakeSocket = WebSocket & {
  emit: (type: string, event?: Event) => void;
};

function createFakeSocket(): FakeSocket {
  const listeners = new Map<string, Set<(event: Event) => void>>();

  return {
    addEventListener(type, listener) {
      const callbacks = listeners.get(type) ?? new Set();
      callbacks.add(listener as (event: Event) => void);
      listeners.set(type, callbacks);
    },
    close: vi.fn(),
    emit(type, event = new Event(type)) {
      listeners.get(type)?.forEach((listener) => listener(event));
    },
  } as unknown as FakeSocket;
}

describe('streamJsonPatchEntries', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('reports an error when the socket closes before finished', async () => {
    const socket = createFakeSocket();
    vi.mocked(openLocalApiWebSocket).mockResolvedValue(socket);
    const onError = vi.fn();

    streamJsonPatchEntries('/logs', { onError });

    await Promise.resolve();
    socket.emit('close');

    expect(onError).toHaveBeenCalledWith(expect.any(Error));
  });
});
