import type { RepositorySummary } from './api';

export const TOP_N = 10;

export function inBothTopN(item: Pick<RepositorySummary, 'clone_rank' | 'human_attention'>, n = TOP_N): boolean {
  const attention = item.human_attention.rank;
  return attention !== null && attention <= n && item.clone_rank <= n;
}

type AttentionItem = Pick<RepositorySummary, 'human_attention' | 'attention_rank_trend'>;

export const NO_SIGNAL_TITLE = 'No viewers, referrals, stars or forks in the window';

export function attentionRankLabel(item: AttentionItem, ordinal: (rank: number) => string): string {
  const { score, rank } = item.human_attention;
  if (score === null) return 'N/A';
  if (rank === null) return '—';
  return ordinal(rank);
}

// Unranked rows only show an arrow on the day they drop off the chart.
export function showAttentionTrend(item: AttentionItem): boolean {
  return item.human_attention.rank !== null || item.attention_rank_trend === 'down';
}
