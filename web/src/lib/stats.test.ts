import { describe, expect, it } from 'vitest';
import { formatPercent, formatStatistic, ordinal, selectedStatistics } from './stats';

describe('dashboard formatting', () => {
  it('formats ordinals and percentages', () => {
    expect(ordinal(1)).toBe('1st');
    expect(ordinal(12)).toBe('12th');
    expect(ordinal(4)).toBe('4th');
    expect(formatPercent(8.125)).toBe('8.13%');
  });

  it('selects total or unique statistics', () => {
    const total = { mean: 4 } as never;
    const unique = { mean: 2 } as never;
    expect(selectedStatistics('total', total, unique)).toBe(total);
    expect(selectedStatistics('unique', total, unique)).toBe(unique);
    expect(formatStatistic(3.456)).toBe('3.46');
    expect(formatStatistic(3, true)).toBe('3');
  });
});
