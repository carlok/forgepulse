export interface Repository {
  name: string;
  description: string;
  stars: number;
  forks: number;
}

export interface RepositorySummary extends Repository {
  total_views: number;
  total_view_uniques: number;
  total_clones: number;
  total_clone_uniques: number;
  clones_1d: number;
  clones_7d: number;
  clones_30d: number;
  clone_rank: number;
  clone_share_percent: number;
  clone_daily_median: number | null;
  human_attention: HumanAttention;
  diagnoses: Diagnosis[];
}

export interface HumanAttentionComponents {
  unique_views_7d: number | null;
  external_referrer_uniques_14d: number | null;
  new_stars_30d: number | null;
  new_forks_30d: number | null;
}
export interface HumanAttention { version: string; score: number | null; rank: number | null; components: HumanAttentionComponents; }
export interface Diagnosis { kind: string; evidence: string[]; }

export interface CloneChartPoint {
  day: string;
  total_clones: number;
  unique_cloners: number;
}

export interface CloneStatistics {
  mean: number;
  median: number;
  population_variance: number;
  population_standard_deviation: number;
  minimum: number;
  maximum: number;
  p95: number;
}

export interface DayPoint { day: string; count: number; uniques: number; }
export interface ReferrerPoint { captured_on: string; referrer: string; count: number; uniques: number; }
export interface PathPoint { captured_on: string; path: string; title: string; count: number; uniques: number; }
export interface StarPoint { day: string; total: number; }

export interface Dashboard {
  ranking: 'human_attention' | 'clone_volume';
  items: RepositorySummary[];
  total_count: number;
  total_stars: number;
  total_forks: number;
  total_views: number;
  total_clones: number;
  chart: CloneChartPoint[];
  views_chart: DayPoint[];
  total_clone_statistics: CloneStatistics | null;
  unique_clone_statistics: CloneStatistics | null;
  repository_clone_daily_median: number | null;
}

export interface RepositoryDetail {
  summary: RepositorySummary;
  clones: DayPoint[];
  views: DayPoint[];
  referrers: ReferrerPoint[];
  paths: PathPoint[];
  stars: StarPoint[];
  forks: StarPoint[];
}

export interface HealthStatus {
  status: string;
  git_ref: string;
}

export async function loadHealth(): Promise<HealthStatus> {
  const response = await fetch('/api/health');
  if (!response.ok) throw new Error(`Health request failed: ${response.status}`);
  return response.json() as Promise<HealthStatus>;
}

export async function loadDashboard(
  query = '',
  page = 1,
  perPage = 25,
  ranking: 'human_attention' | 'clone_volume' = 'human_attention',
  sort: string | null = null,
  dir: 'asc' | 'desc' | null = null
): Promise<Dashboard> {
  let url = `/api/v1/dashboard?q=${encodeURIComponent(query)}&page=${page}&per_page=${perPage}&ranking=${ranking}`;
  if (sort) url += `&sort=${encodeURIComponent(sort)}`;
  if (dir) url += `&dir=${dir}`;
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Dashboard request failed: ${response.status}`);
  return response.json() as Promise<Dashboard>;
}

export async function loadRepository(name: string): Promise<RepositoryDetail> {
  const [owner, repository] = name.split('/', 2);
  const response = await fetch(`/api/v1/repositories/${encodeURIComponent(owner)}/${encodeURIComponent(repository)}`);
  if (!response.ok) throw new Error(`Repository request failed: ${response.status}`);
  return response.json() as Promise<RepositoryDetail>;
}

export function exportUrl(query = ''): string {
  return `/api/v1/export.jsonl?q=${encodeURIComponent(query)}`;
}
