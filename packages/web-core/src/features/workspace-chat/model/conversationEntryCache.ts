import type { PatchTypeWithKey } from '@/shared/hooks/useConversationHistory/types';
import type { ExecutionProcess } from 'shared/types';

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
const PROCESS_MEMORY = new Map<string, ExecutionProcess[]>();
const STORAGE_KEY = 'vibe-conversation-entries';
const PROCESS_STORAGE_KEY = 'vibe-conversation-processes';
const ENTRY_STORAGE_PREFIX = 'vibe-conversation-entry:';
const PROCESS_STORAGE_PREFIX = 'vibe-conversation-process:';
const MAX_PROCESSES = 60;
const MAX_STORAGE_BYTES = 4 * 1024 * 1024; // 4 MiB guard against quota errors
const MAX_PROCESS_SNAPSHOTS = 60;

let entriesStorageSnapshot: Record<string, PatchTypeWithKey[]> | null = null;
let processStorageSnapshot: Record<string, ExecutionProcess[]> | null = null;

function storageKey(prefix: string, id: string): string {
  return `${prefix}${encodeURIComponent(id)}`;
}

function readEntriesStorage(): Record<string, PatchTypeWithKey[]> {
  if (entriesStorageSnapshot) return entriesStorageSnapshot;

  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    entriesStorageSnapshot = raw
      ? (JSON.parse(raw) as Record<string, PatchTypeWithKey[]>)
      : {};
  } catch {
    entriesStorageSnapshot = {};
  }

  return entriesStorageSnapshot;
}

function readProcessStorage(): Record<string, ExecutionProcess[]> {
  if (processStorageSnapshot) return processStorageSnapshot;

  try {
    const raw = localStorage.getItem(PROCESS_STORAGE_KEY);
    processStorageSnapshot = raw
      ? (JSON.parse(raw) as Record<string, ExecutionProcess[]>)
      : {};
  } catch {
    processStorageSnapshot = {};
  }

  return processStorageSnapshot;
}

function writeEntryStorage(
  processId: string,
  entries: PatchTypeWithKey[]
): void {
  try {
    const serialized = JSON.stringify(entries);
    if (serialized.length > MAX_STORAGE_BYTES) return;
    localStorage.setItem(
      storageKey(ENTRY_STORAGE_PREFIX, processId),
      serialized
    );
  } catch {
    // Quota or serialization failure — memory cache still works.
  }
}

function writeProcessStorage(
  scopeKey: string,
  processes: ExecutionProcess[]
): void {
  try {
    localStorage.setItem(
      storageKey(PROCESS_STORAGE_PREFIX, scopeKey),
      JSON.stringify(processes)
    );
  } catch {
    // Quota or serialization failure — the in-memory snapshot still works.
  }
}

export function getCachedEntries(
  processId: string
): PatchTypeWithKey[] | undefined {
  const fromMemory = MEMORY.get(processId);
  if (fromMemory) return fromMemory;

  let fromStorage: PatchTypeWithKey[] | undefined;
  try {
    const raw = localStorage.getItem(
      storageKey(ENTRY_STORAGE_PREFIX, processId)
    );
    fromStorage = raw ? (JSON.parse(raw) as PatchTypeWithKey[]) : undefined;
  } catch {
    fromStorage = undefined;
  }

  // Read the pre-v2 aggregate only for entries that have not been promoted to
  // their own key yet. This keeps existing caches usable without putting the
  // aggregate JSON on the normal path for newly written entries.
  if (!fromStorage) fromStorage = readEntriesStorage()[processId];
  if (fromStorage) {
    MEMORY.set(processId, fromStorage);
    writeEntryStorage(processId, fromStorage);
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

  writeEntryStorage(processId, entries);
}

/**
 * Store the last known process list for a workspace/session scope.
 *
 * The execution-process WebSocket sends this list asynchronously. Keeping a
 * small snapshot lets the conversation render cached entries before the live
 * snapshot arrives; the live list remains authoritative and replaces it as
 * soon as it is available.
 */
export function setCachedExecutionProcesses(
  scopeKey: string,
  processes: ExecutionProcess[]
): void {
  PROCESS_MEMORY.set(scopeKey, processes);

  if (PROCESS_MEMORY.size > MAX_PROCESS_SNAPSHOTS) {
    const oldest = PROCESS_MEMORY.keys().next().value;
    if (oldest !== undefined) PROCESS_MEMORY.delete(oldest);
  }

  writeProcessStorage(scopeKey, processes);
}

export function getCachedExecutionProcesses(
  scopeKey: string
): ExecutionProcess[] | undefined {
  const fromMemory = PROCESS_MEMORY.get(scopeKey);
  if (fromMemory) return fromMemory;

  let fromStorage: ExecutionProcess[] | undefined;
  try {
    const raw = localStorage.getItem(
      storageKey(PROCESS_STORAGE_PREFIX, scopeKey)
    );
    fromStorage = raw ? (JSON.parse(raw) as ExecutionProcess[]) : undefined;
  } catch {
    fromStorage = undefined;
  }

  if (!fromStorage) fromStorage = readProcessStorage()[scopeKey];
  if (fromStorage) {
    PROCESS_MEMORY.set(scopeKey, fromStorage);
    writeProcessStorage(scopeKey, fromStorage);
    return fromStorage;
  }
  return undefined;
}

export function deleteCachedEntries(processId: string): void {
  MEMORY.delete(processId);
  try {
    localStorage.removeItem(storageKey(ENTRY_STORAGE_PREFIX, processId));
  } catch {
    // ignore
  }
}

function removePrefixedStorageKeys(prefixes: string[]): void {
  try {
    const keysToRemove: string[] = [];
    for (let index = 0; index < localStorage.length; index += 1) {
      const key = localStorage.key(index);
      if (key && prefixes.some((prefix) => key.startsWith(prefix))) {
        keysToRemove.push(key);
      }
    }
    keysToRemove.forEach((key) => localStorage.removeItem(key));
  } catch {
    // ignore
  }
}

export function clearConversationEntryCache(): void {
  MEMORY.clear();
  PROCESS_MEMORY.clear();
  entriesStorageSnapshot = null;
  processStorageSnapshot = null;
  try {
    localStorage.removeItem(STORAGE_KEY);
    localStorage.removeItem(PROCESS_STORAGE_KEY);
  } catch {
    // ignore
  }
  removePrefixedStorageKeys([ENTRY_STORAGE_PREFIX, PROCESS_STORAGE_PREFIX]);
}
