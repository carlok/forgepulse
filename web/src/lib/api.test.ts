import { afterEach, describe, expect, it, vi } from 'vitest';
import { exportUrl, loadDashboard, loadHealth, loadRepository, loadSyncRuns } from './api';

afterEach(() => vi.unstubAllGlobals());

describe('dashboard API client', () => {
  it('encodes a dashboard query', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ items: [] }) });
    vi.stubGlobal('fetch', fetchMock);
    await loadDashboard('owner/repo name');
    expect(fetchMock).toHaveBeenCalledWith('/api/v1/dashboard?q=owner%2Frepo%20name&page=1&per_page=25&ranking=human_attention');
    expect(exportUrl('owner/repo name')).toBe('/api/v1/export.jsonl?q=owner%2Frepo%20name');
  });

  it('reports an HTTP error', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 500 }));
    await expect(loadDashboard()).rejects.toThrow('500');
  });

  it('appends sort and dir only when given', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ items: [] }) });
    vi.stubGlobal('fetch', fetchMock);
    await loadDashboard('', 1, 25, 'clone_volume', 'stars', 'asc');
    expect(fetchMock).toHaveBeenCalledWith('/api/v1/dashboard?q=&page=1&per_page=25&ranking=clone_volume&sort=stars&dir=asc');
  });

  it('loads an encoded repository detail endpoint', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ summary: {} }) });
    vi.stubGlobal('fetch', fetchMock);
    await loadRepository('carlok/repository name');
    expect(fetchMock).toHaveBeenCalledWith('/api/v1/repositories/carlok/repository%20name');
  });

  it('loads server health with a git ref', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ status: 'ok', git_ref: 'main@abc1234' }) });
    vi.stubGlobal('fetch', fetchMock);
    await expect(loadHealth()).resolves.toEqual({ status: 'ok', git_ref: 'main@abc1234' });
    expect(fetchMock).toHaveBeenCalledWith('/api/health');
  });

  it('reports an HTTP error for health', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 503 }));
    await expect(loadHealth()).rejects.toThrow('503');
  });

  it('loads sync runs', async () => {
    const runs = [{ id: 1, started_at: '2026-09-09T12:00:00Z', finished_at: '2026-09-09T12:01:00Z', status: 'succeeded', repositories_synced: 5, message: null }];
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => runs });
    vi.stubGlobal('fetch', fetchMock);
    await expect(loadSyncRuns()).resolves.toEqual(runs);
    expect(fetchMock).toHaveBeenCalledWith('/api/v1/sync-runs');
  });

  it('reports an HTTP error for sync runs', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 500 }));
    await expect(loadSyncRuns()).rejects.toThrow('500');
  });
});
