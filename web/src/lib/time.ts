function pad(value: number): string {
  return value.toString().padStart(2, '0');
}

function formatUtc(date: Date): string {
  return `${date.getUTCFullYear()}-${pad(date.getUTCMonth() + 1)}-${pad(date.getUTCDate())} ${pad(date.getUTCHours())}:${pad(date.getUTCMinutes())}`;
}

function formatLocal(date: Date): string {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function formatAgo(elapsedMs: number): string {
  const totalMinutes = Math.max(0, Math.floor(elapsedMs / 60_000));
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return hours > 0 ? `${hours}h ${minutes}m ago` : `${minutes}m ago`;
}

/** e.g. "Updated 2026-09-09 14:32 UTC · 2026-09-09 16:32 local (3h 12m ago)". */
export function formatUpdatedAt(iso: string, now: Date = new Date()): string {
  const then = new Date(iso);
  return `Updated ${formatUtc(then)} UTC · ${formatLocal(then)} local (${formatAgo(now.getTime() - then.getTime())})`;
}
