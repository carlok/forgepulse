import { describe, expect, it } from 'vitest';
import { inBothTopN } from './rank';

const item = (clone_rank: number, rank: number | null) => ({
  clone_rank,
  human_attention: { version: 'v1', score: null, rank, components: { unique_views_7d: null, external_referrer_uniques_14d: null, new_stars_30d: null, new_forks_30d: null } }
});

describe('inBothTopN', () => {
  it('is true only when both ranks are within the top 10', () => {
    expect(inBothTopN(item(1, 10))).toBe(true);
    expect(inBothTopN(item(10, 1))).toBe(true);
  });
  it('is false when either rank falls outside', () => {
    expect(inBothTopN(item(11, 1))).toBe(false);
    expect(inBothTopN(item(1, 11))).toBe(false);
  });
  it('is false when attention has no rank', () => {
    expect(inBothTopN(item(1, null))).toBe(false);
  });
});
