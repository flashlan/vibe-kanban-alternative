import { describe, expect, it } from 'vitest';
import { resolveScrollIntent } from './conversation-scroll-commands';

describe('resolveScrollIntent', () => {
  it('preserves the current anchor while older history is inserted', () => {
    expect(resolveScrollIntent('historic', false, true)).toEqual({
      type: 'preserve-anchor',
    });
  });

  it('follows live messages when the user is at the bottom', () => {
    expect(resolveScrollIntent('running', false, true)).toEqual({
      type: 'follow-bottom',
      behavior: 'auto',
    });
  });

  it('preserves the reader position when live messages arrive above it', () => {
    expect(resolveScrollIntent('running', false, false)).toEqual({
      type: 'preserve-anchor',
    });
  });
});
