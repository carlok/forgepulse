import { describe, expect, it } from 'vitest';
import { formatUpdatedAt } from './time';

describe('formatUpdatedAt', () => {
  it('formats the UTC time and the relative "ago" duration', () => {
    const then = new Date('2026-09-09T12:00:00Z');
    const now = new Date('2026-09-09T15:12:00Z');
    expect(formatUpdatedAt(then.toISOString(), now)).toContain('2026-09-09 12:00 UTC');
    expect(formatUpdatedAt(then.toISOString(), now)).toContain('(3h 12m ago)');
  });

  it('includes the browser-local time alongside UTC', () => {
    const then = new Date('2026-09-09T12:00:00Z');
    const pad = (value: number) => value.toString().padStart(2, '0');
    const expectedLocal = `${then.getFullYear()}-${pad(then.getMonth() + 1)}-${pad(then.getDate())} ${pad(then.getHours())}:${pad(then.getMinutes())}`;
    expect(formatUpdatedAt(then.toISOString(), then)).toContain(`${expectedLocal} local`);
  });

  it('shows minutes only when under an hour has elapsed', () => {
    const then = new Date('2026-09-09T12:00:00Z');
    const now = new Date('2026-09-09T12:05:00Z');
    expect(formatUpdatedAt(then.toISOString(), now)).toContain('(5m ago)');
  });

  it('clamps a future or skewed timestamp to "0m ago" instead of going negative', () => {
    const then = new Date('2026-09-09T12:05:00Z');
    const now = new Date('2026-09-09T12:00:00Z');
    expect(formatUpdatedAt(then.toISOString(), now)).toContain('(0m ago)');
  });
});
