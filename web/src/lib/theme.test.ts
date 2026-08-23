import { describe, expect, it } from 'vitest';
import { CHART_COLORS } from './theme';

describe('chart palette', () => {
  it('provides four distinct series colors', () => {
    expect(CHART_COLORS).toHaveLength(4);
    expect(new Set(CHART_COLORS).size).toBe(4);
  });
});
