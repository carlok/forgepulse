import { describe, expect, it } from 'vitest';
import { attentionRankLabel, inBothTopN, showAttentionTrend } from './rank';

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

describe('attention rank presentation', () => {
  const ordinal = (rank: number) => `#${rank}`;
  const make = (score: number | null, rank: number | null, trend: 'up' | 'down' | 'stable' | 'unknown' = 'stable') => ({
    human_attention: { ...item(1, rank).human_attention, score },
    attention_rank_trend: trend
  });

  it('labels ranked, zero and unknown scores differently', () => {
    expect(attentionRankLabel(make(3.4, 2), ordinal)).toBe('#2');
    expect(attentionRankLabel(make(0, null), ordinal)).toBe('—');
    expect(attentionRankLabel(make(null, null), ordinal)).toBe('N/A');
  });

  it('hides the trend arrow on unranked rows unless they just dropped off', () => {
    expect(showAttentionTrend(make(3.4, 2))).toBe(true);
    expect(showAttentionTrend(make(0, null, 'stable'))).toBe(false);
    expect(showAttentionTrend(make(0, null, 'down'))).toBe(true);
    expect(showAttentionTrend(make(null, null, 'unknown'))).toBe(false);
  });
});
