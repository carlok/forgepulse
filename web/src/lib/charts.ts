export interface DayValue {
  day: string;
}

/** Sorted, deduplicated union of every `day` across one or more independently-fetched series. */
export function unionDays(...series: DayValue[][]): string[] {
  return [...new Set(series.flat().map((point) => point.day))].sort();
}

/** Reads `key` for each day in `days`, zero-filling days missing from `points`. */
export function seriesValues<T extends DayValue>(days: string[], points: T[], key: keyof T): number[] {
  return days.map((day) => {
    const value = points.find((point) => point.day === day)?.[key];
    return typeof value === 'number' ? value : 0;
  });
}
