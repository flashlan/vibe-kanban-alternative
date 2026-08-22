import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { FontWeight } from '@xterm/xterm';

export interface TerminalPreferences {
  fontSize: number;
  fontFamily: string;
  fontWeight: FontWeight;
  fontWeightBold: FontWeight;
  lineHeight: number;
  letterSpacing: number;
  scrollback: number;
  /** Last manually resized height of the bottom terminal drawer window. */
  terminalDrawerHeight: number;
}

const DEFAULT_PREFS: TerminalPreferences = {
  fontSize: 14,
  fontFamily: '"IBM Plex Mono", "Fira Code", "Cascadia Code", monospace',
  fontWeight: '400',
  fontWeightBold: '700',
  lineHeight: 1.2,
  letterSpacing: 0,
  scrollback: 5000,
  terminalDrawerHeight: 320,
};

interface TerminalPreferencesStore extends TerminalPreferences {
  setFontSize: (size: number) => void;
  setFontFamily: (family: string) => void;
  setFontWeight: (weight: FontWeight) => void;
  setFontWeightBold: (weight: FontWeight) => void;
  setLineHeight: (height: number) => void;
  setLetterSpacing: (spacing: number) => void;
  setScrollback: (lines: number) => void;
  setTerminalDrawerHeight: (height: number) => void;
  resetDefaults: () => void;
}

export const useTerminalPreferences = create<TerminalPreferencesStore>()(
  persist(
    (set) => ({
      ...DEFAULT_PREFS,
      setFontSize: (size) => set({ fontSize: size }),
      setFontFamily: (family) => set({ fontFamily: family }),
      setFontWeight: (weight) => set({ fontWeight: weight }),
      setFontWeightBold: (weight) => set({ fontWeightBold: weight }),
      setLineHeight: (height) => set({ lineHeight: height }),
      setLetterSpacing: (spacing) => set({ letterSpacing: spacing }),
      setScrollback: (lines) => set({ scrollback: lines }),
      setTerminalDrawerHeight: (height) =>
        set({ terminalDrawerHeight: height }),
      resetDefaults: () => set(DEFAULT_PREFS),
    }),
    {
      name: 'vibe-kanban-terminal-preferences',
    }
  )
);
