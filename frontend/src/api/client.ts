import type {
  Paginated, SessionSummary, SessionDetail, MessageDetail,
  ToolCallDetail, FileReference, SearchHit, AnalyticsOverview,
  DailyStats, ModelUsage, ToolFrequency, ProjectBreakdown,
  ProjectDetail, OutcomeStats, StorageOverview, ContentBlob,
  IndexCoverage
} from '@/types'

const BASE = '/api'

async function get<T>(path: string, params?: Record<string, string | number | undefined>): Promise<T> {
  const url = new URL(path, window.location.origin)
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      if (v !== undefined && v !== null && v !== '') {
        url.searchParams.set(k, String(v))
      }
    }
  }
  const res = await fetch(url.toString())
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(body.error || res.statusText)
  }
  return res.json()
}

export const api = {
  sessions: {
    list: (params?: { project?: string; from?: string; to?: string; limit?: number; offset?: number }) =>
      get<Paginated<SessionSummary>>(`${BASE}/sessions`, params),
    get: (id: string) =>
      get<SessionDetail>(`${BASE}/sessions/${id}`),
    messages: (id: string, params?: { limit?: number; offset?: number }) =>
      get<Paginated<MessageDetail>>(`${BASE}/sessions/${id}/messages`, params),
    tools: (id: string) =>
      get<ToolCallDetail[]>(`${BASE}/sessions/${id}/tools`),
    files: (id: string) =>
      get<FileReference[]>(`${BASE}/sessions/${id}/files`),
    raw: async (id: string): Promise<string> => {
      const res = await fetch(`${BASE}/sessions/${id}/raw`)
      if (!res.ok) {
        const body = await res.json().catch(() => ({ error: res.statusText }))
        throw new Error(body.error || res.statusText)
      }
      const text = await res.text()
      if (text.startsWith('<!DOCTYPE') || text.startsWith('<html')) {
        throw new Error('Server returned HTML instead of JSONL — restart the server to pick up the new /raw endpoint')
      }
      return text
    },
  },

  search: (params: { q: string; kind?: string; project?: string; limit?: number; offset?: number }) =>
    get<Paginated<SearchHit>>(`${BASE}/search`, params),

  analytics: {
    overview: () => get<AnalyticsOverview>(`${BASE}/analytics/overview`),
    daily: (params?: { from?: string; to?: string }) =>
      get<DailyStats[]>(`${BASE}/analytics/daily`, params),
    models: () => get<ModelUsage[]>(`${BASE}/analytics/models`),
    tools: (params?: { limit?: number }) =>
      get<ToolFrequency[]>(`${BASE}/analytics/tools`, params),
    projects: () => get<ProjectBreakdown[]>(`${BASE}/analytics/projects`),
    outcomes: () => get<OutcomeStats[]>(`${BASE}/analytics/outcomes`),
    coverage: () => get<IndexCoverage>(`${BASE}/analytics/coverage`),
  },

  content: {
    get: (hash: string) => get<ContentBlob>(`${BASE}/content/${hash}`),
  },

  projects: () => get<ProjectDetail[]>(`${BASE}/projects`),

  files: (params?: { path?: string; session?: string; limit?: number; offset?: number }) =>
    get<Paginated<FileReference>>(`${BASE}/files`, params),

  storage: () => get<StorageOverview>(`${BASE}/storage`),
}
