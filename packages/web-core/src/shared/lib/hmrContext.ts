import { createContext, type Context } from 'react';

/**
 * Creates a React context that preserves its identity across Vite HMR updates.
 *
 * During HMR, module re-execution creates a new context object via createContext(),
 * but already-mounted providers still hold the old one. Consumers then read from
 * the new context, find no matching provider, and get the default value (typically
 * null/undefined), which causes "must be used within Provider" errors.
 *
 * This helper stashes the context in import.meta.hot.data so the same object is
 * reused across HMR re-executions of the module.
 *
 * @param key - A unique string key to identify this context in HMR data
 * @param defaultValue - The default context value (same as createContext's argument)
 */
export function createHmrContext<T>(key: string, defaultValue: T): Context<T> {
  const existing = import.meta.hot?.data?.[key] as Context<T> | undefined;
  const ctx = existing ?? createContext<T>(defaultValue);
  if (import.meta.hot) {
    const hot = import.meta.hot as unknown as {
      data?: Record<string, unknown>;
    };
    // In Vite, import.meta.hot.data is always provided; some non-Vite
    // environments (e.g. vitest) expose import.meta.hot without `.data`, so
    // skip HMR persistence there rather than throwing on a missing object.
    if (hot.data) {
      hot.data[key] = ctx;
    }
  }
  return ctx;
}
