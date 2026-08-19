const STORAGE_KEY = 'vibe_chat_prompt_history';
const MAX_HISTORY = 100;

export function getPromptHistory(): string[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function addPromptToHistory(prompt: string): void {
  const trimmed = prompt.trim();
  if (!trimmed) return;

  try {
    const history = getPromptHistory();
    // Avoid immediate duplicate at the top of history
    if (history.length > 0 && history[history.length - 1] === trimmed) {
      return;
    }
    const updated = [...history.filter((item) => item !== trimmed), trimmed];
    if (updated.length > MAX_HISTORY) {
      updated.splice(0, updated.length - MAX_HISTORY);
    }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
  } catch (err) {
    console.error('Failed to save prompt history', err);
  }
}

export function clearPromptHistory(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Ignore storage errors
  }
}
