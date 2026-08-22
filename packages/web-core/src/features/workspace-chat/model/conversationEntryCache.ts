import type { PatchTypeWithKey } from '@/shared/hooks/useConversationHistory/types';

// ---------------------------------------------------------------------------
// Conversation-entry cache
// ---------------------------------------------------------------------------
// Per-execution-process cache of normalized conversation entries. The chat
// reloads these over a websocket stream every time a workspace/session is
// (re)mounted — which is what happens when the operator clicks between two
// workspaces. Finished (historic) processes have stable logs, so caching them
// in the browser makes switching back to a previously-viewed workspace
// instant instead of re-streaming the whole chat.
//
// The cache is a process-global singleton (survives remounts within a page
// session) mirrored to localStorage (survives full page reloads). Sending a
// message is unaffected: it goes to the backend via POST /follow-up and never
// reads from this cache.

const MEMORY = new Map<string, PatchTypeWithKey[]>();
const STORAGE_KEY = 'vibe-conversation-entries';
const MAX_PROCESSES = 60;
const MAX_STORAGE_BYTES = 4 * 1024 * 1024; // 4 MiB guard against quota errors

function readStorage(): Record<string, PatchTypeWithKey[]> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    return JSON.parse(raw) as Record<string, PatchTypeWithKey[]>;
  } catch {
    return {};
  }
}

function writeStorage(): void {
  const snapshot = Object.fromEntries(MEMORY.entries());
  try {
    const serialized = JSON.stringify(snapshot);
    if (serialized.length > MAX_STORAGE_BYTES) return; // too big, keep memory only
    localStorage.setItem(STORAGE_KEY, serialized);
  } catch {
    // Quota or serialization failure — memory cache still works.
  }
}

export function getCachedEntries(
  processId: string
): PatchTypeWithKey[] | undefined {
  const fromMemory = MEMORY.get(processId);
  if (fromMemory) return fromMemory;

  const fromStorage = readStorage()[processId];
  if (fromStorage) {
    MEMORY.set(processId, fromStorage);
    return fromStorage;
  }
  return undefined;
}

export function setCachedEntries(
  processId: string,
  entries: PatchTypeWithKey[]
): void {
  MEMORY.set(processId, entries);

  // Evict oldest inserted when over capacity (simple insertion-order LRU).
  if (MEMORY.size > MAX_PROCESSES) {
    const oldest = MEMORY.keys().next().value;
    if (oldest !== undefined) MEMORY.delete(oldest);
  }

  writeStorage();
}

export function deleteCachedEntries(processId: string): void {
  if (MEMORY.delete(processId)) writeStorage();
}

export function clearConversationEntryCache(): void {
  MEMORY.clear();
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // ignore
  }
}
