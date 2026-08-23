<script lang="ts">
  import { onMount } from 'svelte';
  import * as echarts from 'echarts';
  import { exportUrl, loadDashboard, loadRepository, type Dashboard, type RepositoryDetail } from './lib/api';
  import { formatPercent, formatStatistic, ordinal, selectedStatistics, type StatisticMetric } from './lib/stats';

  let dashboard: Dashboard | null = null;
  let detail: RepositoryDetail | null = null;
  let query = '';
  let error = '';
  let chartElement: HTMLDivElement;
  let detailChartElement: HTMLDivElement;
  let chart: echarts.ECharts | undefined;
  let detailChart: echarts.ECharts | undefined;
  let metric: StatisticMetric = 'total';
  let dark = localStorage.getItem('forgepulse-theme') !== 'light';
  let selectedRepository = repositoryFromPath();

  $: stats = dashboard ? selectedStatistics(metric, dashboard.total_clone_statistics, dashboard.unique_clone_statistics) : null;
  $: if (chartElement && dashboard && !selectedRepository) renderDashboardChart();
  $: if (detailChartElement && detail && selectedRepository) renderRepositoryChart();

  onMount(() => {
    void loadCurrentPage();
    const resize = () => { chart?.resize(); detailChart?.resize(); };
    const popstate = () => { selectedRepository = repositoryFromPath(); void loadCurrentPage(); };
    window.addEventListener('resize', resize);
    window.addEventListener('popstate', popstate);
    return () => {
      window.removeEventListener('resize', resize);
      window.removeEventListener('popstate', popstate);
      chart?.dispose();
      detailChart?.dispose();
    };
  });

  function repositoryFromPath(): string | null {
    const match = window.location.pathname.match(/^\/repositories\/([^/]+)\/([^/]+)$/);
    return match ? `${decodeURIComponent(match[1])}/${decodeURIComponent(match[2])}` : null;
  }

  async function loadCurrentPage() {
    if (selectedRepository) await refreshDetail();
    else await refresh();
  }

  async function refresh() {
    try {
      error = '';
      dashboard = await loadDashboard(query);
    } catch (caught) {
      error = caught instanceof Error ? caught.message : 'Could not load analytics';
    }
  }

  async function refreshDetail() {
    if (!selectedRepository) return;
    try {
      error = '';
      detail = await loadRepository(selectedRepository);
    } catch (caught) {
      error = caught instanceof Error ? caught.message : 'Could not load repository data';
    }
  }

  function openRepository(name: string) {
    selectedRepository = name;
    detail = null;
    history.pushState({}, '', `/repositories/${name.split('/').map(encodeURIComponent).join('/')}`);
    void refreshDetail();
  }

  function goHome() {
    selectedRepository = null;
    detail = null;
    history.pushState({}, '', '/');
    void refresh();
  }

  function clearSearch() {
    query = '';
    void refresh();
  }

  function renderDashboardChart() {
    chart ??= echarts.init(chartElement, dark ? 'dark' : undefined);
    chart.setOption({
      backgroundColor: 'transparent', tooltip: { trigger: 'axis' }, legend: { data: ['Total clones', 'Unique cloners'] },
      grid: { left: 42, right: 18, top: 48, bottom: 30 },
      xAxis: { type: 'category', data: dashboard?.chart.map((point) => point.day) }, yAxis: { type: 'value', minInterval: 1 },
      series: [
        { name: 'Total clones', type: 'line', smooth: true, data: dashboard?.chart.map((point) => point.total_clones), areaStyle: { opacity: 0.08 } },
        { name: 'Unique cloners', type: 'line', smooth: true, data: dashboard?.chart.map((point) => point.unique_cloners) }
      ]
    }, true);
  }

  function renderRepositoryChart() {
    detailChart ??= echarts.init(detailChartElement, dark ? 'dark' : undefined);
    const days = [...new Set([...(detail?.clones ?? []), ...(detail?.views ?? [])].map((point) => point.day))].sort();
    const values = (points: { day: string; count: number; uniques: number }[], key: 'count' | 'uniques') => days.map((day) => points.find((point) => point.day === day)?.[key] ?? 0);
    detailChart.setOption({
      backgroundColor: 'transparent', tooltip: { trigger: 'axis' }, legend: { data: ['Clones', 'Unique cloners', 'Views', 'Unique viewers'] },
      grid: { left: 42, right: 18, top: 48, bottom: 30 }, xAxis: { type: 'category', data: days }, yAxis: { type: 'value', minInterval: 1 },
      series: [
        { name: 'Clones', type: 'line', smooth: true, data: values(detail?.clones ?? [], 'count') },
        { name: 'Unique cloners', type: 'line', smooth: true, data: values(detail?.clones ?? [], 'uniques') },
        { name: 'Views', type: 'line', smooth: true, data: values(detail?.views ?? [], 'count') },
        { name: 'Unique viewers', type: 'line', smooth: true, data: values(detail?.views ?? [], 'uniques') }
      ]
    }, true);
  }

  function toggleTheme() {
    dark = !dark;
    localStorage.setItem('forgepulse-theme', dark ? 'dark' : 'light');
    chart?.dispose(); chart = undefined;
    detailChart?.dispose(); detailChart = undefined;
    if (dashboard && !selectedRepository) renderDashboardChart();
    if (detail && selectedRepository) renderRepositoryChart();
  }
</script>

<svelte:head><title>{selectedRepository ? `${selectedRepository} · ForgePulse` : 'ForgePulse'}</title></svelte:head>

<main class:dark>
  <aside>
    <a class="brand" href="/" on:click|preventDefault={goHome}>ForgePulse</a>
    <p>Local repository traffic history, retained beyond the rolling source window.</p>
    <a class="project-link" href="https://github.com/carlok/forgepulse" target="_blank" rel="noreferrer">ForgePulse project ↗</a>
    <div class="sidebar-bottom">
      {#if !selectedRepository}<a class="button" href={exportUrl(query)}>Export JSONL</a>{/if}
      <button class="button secondary" on:click={toggleTheme}>{dark ? 'Light' : 'Dark'}</button>
    </div>
  </aside>
  <section class="content">
    {#if selectedRepository}
      <header><div><span class="eyebrow">Repository detail</span><h1>{selectedRepository}</h1></div><button class="back" on:click={goHome}>← All repositories</button></header>
      {#if error}<p class="error">{error}</p>{/if}
      {#if detail}
        <p class="description">{detail.summary.description || 'No repository description.'}</p>
        <div class="kpis"><article><span>Total clones</span><strong>{detail.summary.total_clones}</strong></article><article><span>Unique cloners</span><strong>{detail.summary.total_clone_uniques}</strong></article><article><span>Total views</span><strong>{detail.summary.total_views}</strong></article><article><span>Stars</span><strong>{detail.summary.stars}</strong></article></div>
        <section class="panel"><div class="panel-title"><div><h2>Traffic over time</h2><span>Stored clone and view history</span></div><span>#{ordinal(detail.summary.clone_rank)} · {formatPercent(detail.summary.clone_share_percent)}</span></div><div class="detail-chart" bind:this={detailChartElement}></div></section>
        <div class="detail-grid"><section class="panel"><div class="panel-title"><h2>Top referrers</h2><span>{detail.referrers.length} stored</span></div><div class="scroll"><table><thead><tr><th>Referrer</th><th>Views</th><th>Unique</th></tr></thead><tbody>{#each detail.referrers as item}<tr><td>{item.referrer}</td><td>{item.count}</td><td>{item.uniques}</td></tr>{:else}<tr><td colspan="3">No referrer snapshots yet.</td></tr>{/each}</tbody></table></div></section><section class="panel"><div class="panel-title"><h2>Popular paths</h2><span>{detail.paths.length} stored</span></div><div class="scroll"><table><thead><tr><th>Path</th><th>Views</th><th>Unique</th></tr></thead><tbody>{#each detail.paths as item}<tr><td title={item.title}>{item.path}</td><td>{item.count}</td><td>{item.uniques}</td></tr>{:else}<tr><td colspan="3">No path snapshots yet.</td></tr>{/each}</tbody></table></div></section></div>
      {:else}<p class="loading">Loading repository history…</p>{/if}
    {:else}
      <header><div><span class="eyebrow">Analytics console</span><h1>Repositories</h1></div><span class="status">LOCAL</span></header>
      <form on:submit|preventDefault={refresh} class="search"><input bind:value={query} placeholder="owner/repository" aria-label="Search repositories" />{#if query}<button type="button" class="secondary" on:click={clearSearch}>Cancel</button>{/if}<button>Search</button></form>
      {#if error}<p class="error">{error}</p>{/if}
      {#if dashboard}
        <div class="kpis"><article><span>Repositories</span><strong>{dashboard.total_count}</strong></article><article><span>Total clones</span><strong>{dashboard.total_clones}</strong></article><article><span>Total views</span><strong>{dashboard.total_views}</strong></article><article><span>Stars</span><strong>{dashboard.total_stars}</strong></article></div>
        <div class="grid">
          <section class="panel table-panel"><div class="panel-title"><h2>Repository signal</h2><span>{dashboard.total_count} tracked</span></div><div class="scroll"><table><thead><tr><th class="rank-share"># - %</th><th>Name</th><th>Stars</th><th>Views</th><th>Clones</th><th>1d</th><th>7d</th><th>30d</th></tr></thead><tbody>{#each dashboard.items as item}<tr><td class="rank-share"><span>{ordinal(item.clone_rank)}</span><span>{formatPercent(item.clone_share_percent)}</span></td><td><a class="repository-link" href={`/repositories/${item.name}`} on:click|preventDefault={() => openRepository(item.name)}>{item.name}</a><small>{item.description}</small></td><td>{item.stars}</td><td>{item.total_views}</td><td>{item.total_clones}</td><td>{item.clones_1d}</td><td>{item.clones_7d}</td><td>{item.clones_30d}</td></tr>{/each}</tbody></table></div></section>
          <section class="panel chart-panel"><div class="panel-title"><div><h2>Clones over time</h2><span>Total events and unique cloners</span></div></div><div class="chart" bind:this={chartElement}></div><div class="stats-header"><h2>Daily clone statistics</h2><div class="segmented"><button class:active={metric === 'total'} on:click={() => metric = 'total'}>Total clones</button><button class:active={metric === 'unique'} on:click={() => metric = 'unique'}>Unique cloners</button></div></div>{#if stats}<div class="statistics"><div><span>Mean</span><strong>{formatStatistic(stats.mean)}</strong></div><div><span>Median</span><strong>{formatStatistic(stats.median)}</strong></div><div><span>Variance</span><strong>{formatStatistic(stats.population_variance)}</strong></div><div><span>Std. dev.</span><strong>{formatStatistic(stats.population_standard_deviation)}</strong></div><div><span>Minimum</span><strong>{formatStatistic(stats.minimum, true)}</strong></div><div><span>Maximum</span><strong>{formatStatistic(stats.maximum, true)}</strong></div><div><span>P95</span><strong>{formatStatistic(stats.p95, true)}</strong></div></div>{/if}</section>
        </div>
      {:else}<p class="loading">Loading local history…</p>{/if}
    {/if}
  </section>
</main>
