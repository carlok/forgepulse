<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import * as echarts from 'echarts';
  import { ArrowLeft, ChevronLeft, ChevronRight, Download, ExternalLink, GitBranch, Search, User } from '@lucide/svelte';
  import { exportUrl, loadDashboard, loadHealth, loadRepository, type Dashboard, type RepositoryDetail } from './lib/api';
  import { formatPercent, formatStatistic, ordinal, selectedStatistics, type StatisticMetric } from './lib/stats';
  import { CHART_COLORS } from './lib/theme';
  import { seriesValues, unionDays } from './lib/charts';

  let dashboard: Dashboard | null = null;
  let detail: RepositoryDetail | null = null;
  let query = '';
  let error = '';
  let chartElement: HTMLDivElement;
  let detailChartElement: HTMLDivElement;
  let chart: echarts.ECharts | undefined;
  let detailChart: echarts.ECharts | undefined;
  let metric: StatisticMetric = 'total';
  let ranking: 'human_attention' | 'clone_volume' = 'human_attention';
  let selectedRepository = repositoryFromPath();
  const PER_PAGE = 25;
  let page = 1;
  let gitRef = 'local';

  $: stats = dashboard ? selectedStatistics(metric, dashboard.total_clone_statistics, dashboard.unique_clone_statistics) : null;
  $: totalPages = dashboard ? Math.max(1, Math.ceil(dashboard.total_count / PER_PAGE)) : 1;
  // Median, across every matched repo, of each repo's OWN daily-clone median — "what does a
  // typical repo's typical day look like." Compared per-row against that same per-repo median,
  // so both sides of the comparison are on the same basis (not a repo's cumulative total against
  // a fleet-wide daily figure). A single global threshold scoped to the current search, not
  // recomputed per page — it's expected for a whole page to land on one side of it.
  $: medianClones = dashboard?.repository_clone_daily_median ?? null;
  $: if (chartElement && dashboard && !selectedRepository) renderDashboardChart();
  $: if (detailChartElement && detail && selectedRepository) renderRepositoryChart();

  onMount(() => {
    void loadCurrentPage();
    void loadHealth().then((health) => { gitRef = health.git_ref; }).catch(() => {});
    const resize = () => { chart?.resize(); detailChart?.resize(); };
    const popstate = () => { disposeCharts(); selectedRepository = repositoryFromPath(); void loadCurrentPage(); };
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
      dashboard = await loadDashboard(query, page, PER_PAGE, ranking);
    } catch (caught) {
      error = caught instanceof Error ? caught.message : 'Could not load analytics';
    }
  }

  function submitSearch() {
    page = 1;
    void refresh();
  }

  function goToPage(next: number) {
    page = next;
    void refresh();
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

  // Plain left-clicks do in-page SPA navigation; anything that should open a new tab/window
  // (middle-click, Cmd/Ctrl/Shift-click) falls through to the browser's native href handling.
  function navigate(event: MouseEvent, action: () => void) {
    if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    action();
  }

  function disposeCharts() {
    chart?.dispose(); chart = undefined;
    detailChart?.dispose(); detailChart = undefined;
  }

  // GitHub reports some referrers as a hostname ("github.com") and others as a bare label
  // ("Google") or a mobile app id ("org.telegram.messenger") — only the former is a real link.
  function referrerUrl(referrer: string): string | null {
    return /^\S+\.\S+$/.test(referrer) ? `https://${referrer}` : null;
  }

  function openRepository(name: string) {
    disposeCharts();
    selectedRepository = name;
    detail = null;
    history.pushState({}, '', `/repositories/${name.split('/').map(encodeURIComponent).join('/')}`);
    void refreshDetail();
  }

  function goHome() {
    disposeCharts();
    selectedRepository = null;
    detail = null;
    page = 1;
    history.pushState({}, '', '/');
    void refresh();
  }

  function clearSearch() {
    query = '';
    page = 1;
    void refresh();
  }

  function setRanking(next: 'human_attention' | 'clone_volume') {
    ranking = next;
    page = 1;
    void refresh();
  }

  function attentionTitle(item: Dashboard['items'][number]): string {
    const c = item.human_attention.components;
    const value = (label: string, number: number | null) => `${label}: ${number === null ? 'N/A' : number}`;
    return [
      `Human attention ${item.human_attention.version}`,
      value('Unique views (7d)', c.unique_views_7d),
      value('External referrer uniques (14d)', c.external_referrer_uniques_14d),
      value('New stars (30d)', c.new_stars_30d),
      value('New forks (30d)', c.new_forks_30d),
    ].join('\n');
  }

  function diagnosisLabel(kind: string): string {
    return kind.split('_').map((word) => word[0].toUpperCase() + word.slice(1)).join(' ');
  }

  function renderDashboardChart() {
    chart ??= echarts.init(chartElement, 'dark');
    const clonePoints = dashboard?.chart ?? [];
    const viewPoints = dashboard?.views_chart ?? [];
    const days = unionDays(clonePoints, viewPoints);
    chart.setOption({
      backgroundColor: 'transparent', color: CHART_COLORS, tooltip: { trigger: 'axis' },
      legend: { data: ['Total clones', 'Unique cloners', 'Views', 'Unique viewers'] },
      grid: { left: 42, right: 18, top: 48, bottom: 30 },
      xAxis: { type: 'category', data: days }, yAxis: { type: 'value', minInterval: 1 },
      series: [
        { name: 'Total clones', type: 'line', smooth: true, data: seriesValues(days, clonePoints, 'total_clones'), areaStyle: { opacity: 0.08 } },
        { name: 'Unique cloners', type: 'line', smooth: true, data: seriesValues(days, clonePoints, 'unique_cloners') },
        { name: 'Views', type: 'line', smooth: true, data: seriesValues(days, viewPoints, 'count') },
        { name: 'Unique viewers', type: 'line', smooth: true, data: seriesValues(days, viewPoints, 'uniques') }
      ]
    }, true);
  }

  function renderRepositoryChart() {
    detailChart ??= echarts.init(detailChartElement, 'dark');
    const clonePoints = detail?.clones ?? [];
    const viewPoints = detail?.views ?? [];
    const days = unionDays(clonePoints, viewPoints);
    detailChart.setOption({
      backgroundColor: 'transparent', color: CHART_COLORS, tooltip: { trigger: 'axis' },
      legend: { data: ['Clones', 'Unique cloners', 'Views', 'Unique viewers'] },
      grid: { left: 42, right: 18, top: 48, bottom: 30 }, xAxis: { type: 'category', data: days }, yAxis: { type: 'value', minInterval: 1 },
      series: [
        { name: 'Clones', type: 'line', smooth: true, data: seriesValues(days, clonePoints, 'count') },
        { name: 'Unique cloners', type: 'line', smooth: true, data: seriesValues(days, clonePoints, 'uniques') },
        { name: 'Views', type: 'line', smooth: true, data: seriesValues(days, viewPoints, 'count') },
        { name: 'Unique viewers', type: 'line', smooth: true, data: seriesValues(days, viewPoints, 'uniques') }
      ]
    }, true);
  }
</script>

<svelte:head><title>{selectedRepository ? `${selectedRepository} · ForgePulse` : 'ForgePulse'}</title></svelte:head>

<main>
  <aside>
    <a class="brand" href="/" on:click={(event) => navigate(event, goHome)}>ForgePulse</a>
    <p>Local repository traffic history, retained beyond the rolling source window.</p>
    <a class="project-link" href="https://github.com/carlok/forgepulse" target="_blank" rel="noreferrer">ForgePulse project<ExternalLink size={13} /></a>
    {#if !selectedRepository}<a class="button export-link" href={exportUrl(query)}><Download size={14} />Export JSONL</a>{/if}
  </aside>
  <section class="content">
    {#if selectedRepository}
      <header><div><span class="eyebrow">Repository detail</span><h1><a class="repository-title-link" href={`https://github.com/${selectedRepository}`} target="_blank" rel="noreferrer">{selectedRepository}<ExternalLink size={18} /></a></h1></div><button class="back" on:click={goHome}><ArrowLeft size={14} />All repositories</button></header>
      {#if error}<p class="error">{error}</p>{/if}
      {#if detail}
        <p class="description">{detail.summary.description || 'No repository description.'}</p>
        <div class="kpis">
          <article in:fly={{ y: 8, duration: 260, delay: 0 }}><span>Total clones</span><strong>{detail.summary.total_clones}</strong></article>
          <article in:fly={{ y: 8, duration: 260, delay: 40 }}><span>Unique cloners</span><strong>{detail.summary.total_clone_uniques}</strong></article>
          <article in:fly={{ y: 8, duration: 260, delay: 80 }}><span>Total views</span><strong>{detail.summary.total_views}</strong></article>
          <article in:fly={{ y: 8, duration: 260, delay: 120 }}><span>Stars</span><strong>{detail.summary.stars}</strong></article>
        </div>
        <section class="panel" in:fade={{ duration: 220 }}><div class="panel-title"><div><h2>Traffic over time</h2><span>Stored clone and view history</span></div></div><div class="detail-chart" bind:this={detailChartElement}></div></section>
        <div class="detail-grid"><section class="panel"><div class="panel-title"><h2>Top referrers</h2><span>{detail.referrers.length} stored</span></div><div class="scroll"><table><thead><tr><th>Referrer</th><th>Views</th><th>Unique</th></tr></thead><tbody>{#each detail.referrers as item}<tr><td>{#if referrerUrl(item.referrer)}<a class="repository-link" href={referrerUrl(item.referrer)} target="_blank" rel="noreferrer">{item.referrer}</a>{:else}{item.referrer}{/if}</td><td>{item.count}</td><td>{item.uniques}</td></tr>{:else}<tr><td colspan="3">No referrer snapshots yet.</td></tr>{/each}</tbody></table></div></section><section class="panel"><div class="panel-title"><h2>Popular paths</h2><span>{detail.paths.length} stored</span></div><div class="scroll"><table><thead><tr><th>Path</th><th>Views</th><th>Unique</th></tr></thead><tbody>{#each detail.paths as item}<tr><td><a class="repository-link" title={item.title} href={`https://github.com${item.path}`} target="_blank" rel="noreferrer">{item.path}</a></td><td>{item.count}</td><td>{item.uniques}</td></tr>{:else}<tr><td colspan="3">No path snapshots yet.</td></tr>{/each}</tbody></table></div></section></div>
      {:else}<p class="loading">Loading repository history…</p>{/if}
    {:else}
      <header><div><span class="eyebrow">Analytics console</span><h1>Repositories</h1></div><span class="status">{gitRef}</span></header>
      <form on:submit|preventDefault={submitSearch} class="search"><input bind:value={query} placeholder="owner/repository" aria-label="Search repositories" />{#if query}<button type="button" class="secondary" on:click={clearSearch}>Cancel</button>{/if}<button><Search size={14} />Search</button></form>
      {#if error}<p class="error">{error}</p>{/if}
      {#if dashboard}
        <div class="kpis">
          <article in:fly={{ y: 8, duration: 260, delay: 0 }}><span>Repositories</span><strong>{dashboard.total_count}</strong></article>
          <article in:fly={{ y: 8, duration: 260, delay: 40 }}><span>Total clones</span><strong>{dashboard.total_clones}</strong></article>
          <article in:fly={{ y: 8, duration: 260, delay: 80 }}><span>Total views</span><strong>{dashboard.total_views}</strong></article>
          <article in:fly={{ y: 8, duration: 260, delay: 120 }}><span>Stars</span><strong>{dashboard.total_stars}</strong></article>
        </div>
        <div class="grid">
          <section class="panel table-panel" in:fade={{ duration: 220 }}><div class="panel-title"><h2>Repository signal</h2><div class="ranking-toggle" aria-label="Ranking view"><button class:active={ranking === 'human_attention'} on:click={() => setRanking('human_attention')}>Human attention</button><button class:active={ranking === 'clone_volume'} on:click={() => setRanking('clone_volume')}>Clone volume</button></div></div><div class="scroll"><table><thead><tr><th class="rank-share">{ranking === 'human_attention' ? 'Attention' : 'Rank'}</th><th>Name</th><th>Stars</th><th>Views</th><th>Clones</th><th>1d</th><th>7d</th><th>30d</th></tr></thead><tbody>{#each dashboard.items as item (item.name)}<tr animate:flip={{ duration: 220 }}><td class="rank-share">{#if ranking === 'human_attention'}<span title={attentionTitle(item)}>{item.human_attention.rank === null ? 'N/A' : ordinal(item.human_attention.rank)}</span><span title={attentionTitle(item)}>{item.human_attention.score === null ? 'N/A' : item.human_attention.score.toFixed(2)}</span>{:else}<span>{ordinal(item.clone_rank)}</span><span>{formatPercent(item.clone_share_percent)}</span>{/if}</td><td><a class="repository-link" href={`/repositories/${item.name}`} on:click={(event) => navigate(event, () => openRepository(item.name))}>{item.name}</a><small>{item.description}</small>{#if item.diagnoses.length}<div class="diagnoses">{#each item.diagnoses as diagnosis}<div class="diagnosis-detail"><span class="diagnosis">{diagnosisLabel(diagnosis.kind)}</span><small>{diagnosis.evidence.join(' · ')}</small></div>{/each}</div>{/if}</td><td>{item.stars}</td><td>{item.total_views}</td><td class:above-median={medianClones !== null && item.clone_daily_median !== null && item.clone_daily_median > medianClones}>{item.total_clones}</td><td>{item.clones_1d}</td><td>{item.clones_7d}</td><td>{item.clones_30d}</td></tr>{/each}</tbody></table></div>{#if totalPages > 1}<div class="pager"><span>Page {page} of {totalPages}</span><div class="pager-controls"><button class="button" disabled={page <= 1} on:click={() => goToPage(page - 1)}><ChevronLeft size={14} />Prev</button><button class="button" disabled={page >= totalPages} on:click={() => goToPage(page + 1)}>Next<ChevronRight size={14} /></button></div></div>{/if}</section>
          <section class="panel chart-panel" in:fade={{ duration: 220, delay: 60 }}><div class="panel-title"><div><h2>Clones &amp; views over time</h2><span>Total events and unique visitors</span></div></div><div class="chart" bind:this={chartElement}></div><div class="stats-header"><h2>Daily clone statistics</h2><div class="segmented"><button class:active={metric === 'total'} on:click={() => metric = 'total'}>Total clones</button><button class:active={metric === 'unique'} on:click={() => metric = 'unique'}>Unique cloners</button></div></div>{#if stats}<div class="statistics"><div><span title="Average value per day across the shown period.">Mean</span><strong>{formatStatistic(stats.mean)}</strong></div><div><span title="Middle value: half the days are at or below this.">Median</span><strong>{formatStatistic(stats.median)}</strong></div><div><span title="Squared spread from the mean — large scale, compare via standard deviation instead.">Variance</span><strong>{formatStatistic(stats.population_variance)}</strong></div><div><span title="Typical spread from the mean, in the same units as the data.">Std. dev.</span><strong>{formatStatistic(stats.population_standard_deviation)}</strong></div><div><span title="Lowest value across the shown period.">Minimum</span><strong>{formatStatistic(stats.minimum, true)}</strong></div><div><span title="Highest value across the shown period.">Maximum</span><strong>{formatStatistic(stats.maximum, true)}</strong></div><div><span title="95th percentile — 95% of days were at or below this value.">P95</span><strong>{formatStatistic(stats.p95, true)}</strong></div></div>{/if}</section>
        </div>
      {:else}<p class="loading">Loading local history…</p>{/if}
    {/if}
  </section>
</main>
<footer class="site-footer">
  <span>ForgePulse v{__APP_VERSION__} · © 2026 <a href="https://github.com/carlok" target="_blank" rel="noreferrer"><User size={12} />Carlo Perassi</a></span>
  <span class="footer-links"><a href="https://github.com/carlok/forgepulse" target="_blank" rel="noreferrer"><GitBranch size={12} />ForgePulse on GitHub</a></span>
</footer>
