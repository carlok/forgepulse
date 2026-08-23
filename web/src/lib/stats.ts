import type { CloneStatistics } from './api';

export type StatisticMetric = 'total' | 'unique';

export function ordinal(value: number): string {
  const mod100 = value % 100;
  if (mod100 < 11 || mod100 > 13) {
    const suffix = ({ 1: 'st', 2: 'nd', 3: 'rd' } as Record<number, string>)[value % 10];
    if (suffix) return `${value}${suffix}`;
  }
  return `${value}th`;
}

export function formatPercent(value: number): string {
  return `${value.toFixed(2)}%`;
}

export function formatStatistic(value: number, integer = false): string {
  return integer ? String(value) : value.toFixed(2);
}

export function selectedStatistics(
  metric: StatisticMetric,
  total: CloneStatistics | null,
  unique: CloneStatistics | null
): CloneStatistics | null {
  return metric === 'total' ? total : unique;
}

