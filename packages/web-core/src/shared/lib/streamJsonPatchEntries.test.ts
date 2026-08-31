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

  it('notifies cancellation when the consumer closes an unfinished stream', async () => {
    const socket = createFakeSocket();
    vi.mocked(openLocalApiWebSocket).mockResolvedValue(socket);
    const onClosed = vi.fn();

    const stream = streamJsonPatchEntries('/logs', { onClosed });

    await Promise.resolve();
    stream.close();
    stream.close();

    expect(onClosed).toHaveBeenCalledTimes(1);
  });

  it('applies direct entry patches without replacing the entries array', async () => {
    const socket = createFakeSocket();
    vi.mocked(openLocalApiWebSocket).mockResolvedValue(socket);
    const animationFrames: FrameRequestCallback[] = [];
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      animationFrames.push(callback);
      return animationFrames.length;
    });

    const stream = streamJsonPatchEntries<string>('/logs');

    await Promise.resolve();
    socket.emit('message', {
      data: JSON.stringify({
        JsonPatch: [
          { op: 'add', path: '/entries/0', value: 'first' },
          { op: 'add', path: '/entries/1', value: 'second' },
        ],
      }),
    } as MessageEvent);
    animationFrames.shift()?.(0);

    const entries = stream.getEntries();
    expect(entries).toEqual(['first', 'second']);

    socket.emit('message', {
      data: JSON.stringify({
        JsonPatch: [{ op: 'replace', path: '/entries/1', value: 'updated' }],
      }),
    } as MessageEvent);
    animationFrames.shift()?.(0);

    expect(stream.getEntries()).toBe(entries);
    expect(stream.getEntries()).toEqual(['first', 'updated']);
    stream.close();
    vi.unstubAllGlobals();
  });
});
