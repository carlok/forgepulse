import { describe, expect, it } from 'vitest';
import { seriesValues, unionDays } from './charts';

describe('chart day merging', () => {
  it('unions and sorts days across series with different ranges', () => {
    const clones = [{ day: '2026-01-02' }, { day: '2026-01-01' }];
    const views = [{ day: '2026-01-03' }, { day: '2026-01-01' }];
    expect(unionDays(clones, views)).toEqual(['2026-01-01', '2026-01-02', '2026-01-03']);
  });

  it('returns an empty list when every series is empty', () => {
    expect(unionDays([], [])).toEqual([]);
  });

  it('zero-fills days missing from a series', () => {
    const points = [
      { day: '2026-01-01', count: 5 },
      { day: '2026-01-03', count: 9 }
    ];
    const days = ['2026-01-01', '2026-01-02', '2026-01-03'];
    expect(seriesValues(days, points, 'count')).toEqual([5, 0, 9]);
  });

  it('reads a different key per call, e.g. clone vs unique columns', () => {
    const points = [{ day: '2026-01-01', total_clones: 10, unique_cloners: 4 }];
    expect(seriesValues(['2026-01-01'], points, 'total_clones')).toEqual([10]);
    expect(seriesValues(['2026-01-01'], points, 'unique_cloners')).toEqual([4]);
  });
});
