import type { RepositorySummary } from './api';

export const TOP_N = 10;

export function inBothTopN(item: Pick<RepositorySummary, 'clone_rank' | 'human_attention'>, n = TOP_N): boolean {
  const attention = item.human_attention.rank;
  return attention !== null && attention <= n && item.clone_rank <= n;
}
