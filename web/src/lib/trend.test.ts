import { Minus, TrendingDown, TrendingUp } from '@lucide/svelte';
import { describe, expect, it } from 'vitest';
import { trendPresentation } from './trend';

describe('trendPresentation', () => {
  it('renders up as a green upward trend', () => {
    const presentation = trendPresentation('up');
    expect(presentation.icon).toBe(TrendingUp);
    expect(presentation.color).toBe('var(--success)');
  });

  it('renders down as a red downward trend', () => {
    const presentation = trendPresentation('down');
    expect(presentation.icon).toBe(TrendingDown);
    expect(presentation.color).toBe('var(--danger)');
  });

  it('renders stable as a muted dash', () => {
    const presentation = trendPresentation('stable');
    expect(presentation.icon).toBe(Minus);
    expect(presentation.color).toBe('var(--text-muted)');
  });

  it('renders unknown the same as stable, but with a distinct label', () => {
    const presentation = trendPresentation('unknown');
    expect(presentation.icon).toBe(Minus);
    expect(presentation.color).toBe('var(--text-muted)');
    expect(presentation.label).not.toBe(trendPresentation('stable').label);
  });
});
