import { describe, expect, it } from 'vitest';
import { CHART_PALETTES, THEME_OPTIONS, isDarkTheme, isValidTheme } from './theme';

describe('theme selection', () => {
  it('validates known theme names only', () => {
    expect(isValidTheme('terminal')).toBe(true);
    expect(isValidTheme('editorial')).toBe(true);
    expect(isValidTheme('geometric')).toBe(true);
    expect(isValidTheme('dark')).toBe(false);
    expect(isValidTheme('light')).toBe(false);
    expect(isValidTheme(null)).toBe(false);
  });

  it('flags only the terminal theme as dark for ECharts', () => {
    expect(isDarkTheme('terminal')).toBe(true);
    expect(isDarkTheme('editorial')).toBe(false);
    expect(isDarkTheme('geometric')).toBe(false);
  });

  it('gives every listed theme a 4-color chart palette', () => {
    for (const option of THEME_OPTIONS) {
      expect(CHART_PALETTES[option.value]).toHaveLength(4);
    }
  });

  it('keeps every theme palette free of near-duplicate hues', () => {
    // Regression check: "Total clones" and "Unique viewers" once shared almost the same hue.
    for (const colors of Object.values(CHART_PALETTES)) {
      expect(new Set(colors).size).toBe(colors.length);
    }
  });
});
