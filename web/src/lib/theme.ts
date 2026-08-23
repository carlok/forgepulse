export type Theme = 'terminal' | 'editorial' | 'geometric';

export const THEME_OPTIONS: { value: Theme; label: string }[] = [
  { value: 'terminal', label: 'Terminal' },
  { value: 'editorial', label: 'Editorial' },
  { value: 'geometric', label: 'Geometric' }
];

export const CHART_PALETTES: Record<Theme, string[]> = {
  terminal: ['#f2a93b', '#5fb0d9', '#4ade80', '#f2685c'],
  editorial: ['#b5502f', '#2f6e6a', '#7c8f3f', '#5b4b8a'],
  geometric: ['#2b4bf2', '#ff6b35', '#4c9a2a', '#e0218a']
};

export function isValidTheme(value: string | null): value is Theme {
  return value === 'terminal' || value === 'editorial' || value === 'geometric';
}

export function isDarkTheme(value: Theme): boolean {
  return value === 'terminal';
}
