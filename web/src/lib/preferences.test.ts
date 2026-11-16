import { describe, expect, it } from 'vitest';
import { loadSkipArchived, saveSkipArchived } from './preferences';

function memoryStorage() {
  const data = new Map<string, string>();
  return { getItem: (key: string) => data.get(key) ?? null, setItem: (key: string, value: string) => { data.set(key, value); } };
}

describe('skip-archived preference', () => {
  it('defaults to showing archived repositories', () => {
    expect(loadSkipArchived(memoryStorage())).toBe(false);
    expect(loadSkipArchived(undefined)).toBe(false);
  });

  it('round-trips through storage', () => {
    const storage = memoryStorage();
    saveSkipArchived(true, storage);
    expect(loadSkipArchived(storage)).toBe(true);
    saveSkipArchived(false, storage);
    expect(loadSkipArchived(storage)).toBe(false);
  });

  it('survives storage that throws', () => {
    const broken = { getItem: () => { throw new Error('blocked'); }, setItem: () => { throw new Error('blocked'); } };
    expect(loadSkipArchived(broken)).toBe(false);
    expect(() => saveSkipArchived(true, broken)).not.toThrow();
  });
});
