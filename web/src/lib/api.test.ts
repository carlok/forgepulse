import { afterEach, describe, expect, it, vi } from 'vitest';
import { exportUrl, loadDashboard, loadRepository } from './api';

afterEach(() => vi.unstubAllGlobals());

describe('dashboard API client', () => {
  it('encodes a dashboard query', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ items: [] }) });
    vi.stubGlobal('fetch', fetchMock);
    await loadDashboard('owner/repo name');
    expect(fetchMock).toHaveBeenCalledWith('/api/v1/dashboard?q=owner%2Frepo%20name');
    expect(exportUrl('owner/repo name')).toBe('/api/v1/export.jsonl?q=owner%2Frepo%20name');
  });

  it('reports an HTTP error', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 500 }));
    await expect(loadDashboard()).rejects.toThrow('500');
  });

  it('loads an encoded repository detail endpoint', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ summary: {} }) });
    vi.stubGlobal('fetch', fetchMock);
    await loadRepository('carlok/repository name');
    expect(fetchMock).toHaveBeenCalledWith('/api/v1/repositories/carlok/repository%20name');
  });
});
