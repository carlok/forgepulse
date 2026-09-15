import { Minus, TrendingDown, TrendingUp } from '@lucide/svelte';
import type { RankTrend } from './api';

export interface TrendPresentation {
  icon: typeof TrendingUp;
  color: string;
  label: string;
}

/** `unknown` renders the same as `stable` (a muted dash) — "no history yet" and "no change"
 * look the same at a glance; the label still tells them apart on hover. */
export function trendPresentation(trend: RankTrend): TrendPresentation {
  switch (trend) {
    case 'up':
      return { icon: TrendingUp, color: 'var(--success)', label: 'Rank improved since yesterday' };
    case 'down':
      return { icon: TrendingDown, color: 'var(--danger)', label: 'Rank worsened since yesterday' };
    case 'stable':
      return { icon: Minus, color: 'var(--text-muted)', label: 'Rank unchanged since yesterday' };
    case 'unknown':
      return { icon: Minus, color: 'var(--text-muted)', label: 'Not enough history to compare' };
  }
}
