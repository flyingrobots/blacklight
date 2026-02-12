<template>
  <div class="dashboard">
    <h2>Dashboard</h2>

    <div v-if="loading" class="loading">Loading...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <template v-else>
      <div class="stats-grid" v-if="overview">
        <div class="stat-card">
          <div class="stat-value">{{ overview.total_sessions.toLocaleString() }}</div>
          <div class="stat-label">Sessions</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{{ overview.total_messages.toLocaleString() }}</div>
          <div class="stat-label">Messages</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{{ overview.total_blobs.toLocaleString() }}</div>
          <div class="stat-label">Content Blobs</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{{ formatBytes(overview.total_blob_bytes) }}</div>
          <div class="stat-label">Content Size</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{{ formatBytes(overview.db_size_bytes) }}</div>
          <div class="stat-label">Database Size</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{{ overview.first_session?.slice(0, 10) ?? 'N/A' }}</div>
          <div class="stat-label">First Session</div>
        </div>
      </div>

      <div class="section" v-if="coverage">
        <h3>Index Coverage</h3>
        <div class="coverage-grid">
          <div class="coverage-card">
            <div class="coverage-header">
              <span class="coverage-label">Files Indexed</span>
              <span class="coverage-pct">{{ coverage.file_pct.toFixed(1) }}%</span>
            </div>
            <div class="progress-bar">
              <div class="progress-fill" :style="{ width: coverage.file_pct + '%' }"></div>
            </div>
            <div class="coverage-detail">
              {{ coverage.indexed_files.toLocaleString() }} of {{ coverage.source_files.toLocaleString() }} files
              ({{ formatBytes(coverage.indexed_bytes) }} of {{ formatBytes(coverage.source_bytes) }})
            </div>
          </div>

          <div class="coverage-card">
            <div class="coverage-header">
              <span class="coverage-label">Data Indexed</span>
              <span class="coverage-pct">{{ coverage.byte_pct.toFixed(1) }}%</span>
            </div>
            <div class="progress-bar">
              <div class="progress-fill green" :style="{ width: coverage.byte_pct + '%' }"></div>
            </div>
            <div class="coverage-detail">
              {{ formatBytes(coverage.indexed_bytes) }} of {{ formatBytes(coverage.source_bytes) }} on disk
            </div>
          </div>

          <div class="coverage-card">
            <div class="coverage-header">
              <span class="coverage-label">Searchable (FTS5)</span>
              <span class="coverage-pct">{{ coverage.search_pct.toFixed(1) }}%</span>
            </div>
            <div class="progress-bar">
              <div class="progress-fill purple" :style="{ width: coverage.search_pct + '%' }"></div>
            </div>
            <div class="coverage-detail">
              {{ coverage.blobs_searchable.toLocaleString() }} of {{ coverage.blobs_stored.toLocaleString() }} content blobs in FTS5
            </div>
          </div>

          <div class="coverage-card">
            <div class="coverage-header">
              <span class="coverage-label">Messages with Content</span>
              <span class="coverage-pct">{{ msgPct.toFixed(1) }}%</span>
            </div>
            <div class="progress-bar">
              <div class="progress-fill green" :style="{ width: msgPct + '%' }"></div>
            </div>
            <div class="coverage-detail">
              {{ coverage.messages_with_content.toLocaleString() }} of {{ coverage.total_messages.toLocaleString() }} messages have content blocks
            </div>
          </div>
        </div>

        <div class="coverage-kinds" v-if="coverage.by_kind.length">
          <h4>Indexed File Types</h4>
          <div class="kind-list">
            <div v-for="k in coverage.by_kind" :key="k.kind" class="kind-row">
              <span class="kind-name">{{ k.kind }}</span>
              <span class="kind-count">{{ k.file_count.toLocaleString() }} files</span>
              <span class="kind-bytes">{{ formatBytes(k.total_bytes) }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="section" v-if="dailyStats.length">
        <h3>Daily Activity</h3>
        <DailyChart :data="dailyStats" />
      </div>

      <div class="section" v-if="recentSessions.length">
        <h3>Recent Sessions</h3>
        <div class="session-list">
          <SessionCard
            v-for="session in recentSessions"
            :key="session.id"
            :session="session"
          />
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api } from '@/api/client'
import type { AnalyticsOverview, DailyStats, SessionSummary, IndexCoverage } from '@/types'
import DailyChart from '@/components/DailyChart.vue'
import SessionCard from '@/components/SessionCard.vue'

const loading = ref(true)
const error = ref('')
const overview = ref<AnalyticsOverview | null>(null)
const coverage = ref<IndexCoverage | null>(null)
const dailyStats = ref<DailyStats[]>([])
const recentSessions = ref<SessionSummary[]>([])

const msgPct = computed(() => {
  if (!coverage.value || coverage.value.total_messages === 0) return 0
  return (coverage.value.messages_with_content / coverage.value.total_messages) * 100
})

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

onMounted(async () => {
  try {
    const [ov, cov, daily, sessions] = await Promise.all([
      api.analytics.overview(),
      api.analytics.coverage(),
      api.analytics.daily(),
      api.sessions.list({ limit: 10 }),
    ])
    overview.value = ov
    coverage.value = cov
    dailyStats.value = daily
    recentSessions.value = sessions.items

  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
})

</script>

<style scoped>
.dashboard h2 { margin-bottom: 1.5rem; }
.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 1rem;
  margin-bottom: 2rem;
}
.stat-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 1.25rem;
}
.stat-value {
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--accent);
}
.stat-label {
  color: var(--text-secondary);
  font-size: 0.875rem;
  margin-top: 0.25rem;
}
.section { margin-bottom: 2rem; }
.section h3 { margin-bottom: 1rem; color: var(--text-secondary); }
.session-list { display: flex; flex-direction: column; gap: 0.75rem; }

/* Coverage */
.coverage-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 1rem;
  margin-bottom: 1.25rem;
}
.coverage-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 1rem;
}
.coverage-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}
.coverage-label { font-size: 0.875rem; font-weight: 500; }
.coverage-pct { font-size: 1.25rem; font-weight: 700; color: var(--accent); }
.progress-bar {
  height: 8px;
  background: var(--bg-tertiary);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 0.5rem;
}
.progress-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 4px;
  transition: width 0.5s ease;
}
.progress-fill.green { background: var(--success); }
.progress-fill.purple { background: var(--purple); }
.coverage-detail { font-size: 0.75rem; color: var(--text-secondary); }

.coverage-kinds { margin-top: 0.5rem; }
.coverage-kinds h4 { font-size: 0.8125rem; color: var(--text-secondary); margin-bottom: 0.5rem; font-weight: 500; }
.kind-list { display: flex; flex-direction: column; gap: 0.25rem; }
.kind-row {
  display: flex;
  align-items: center;
  gap: 1rem;
  font-size: 0.8125rem;
  padding: 0.25rem 0;
}
.kind-name { color: var(--text); min-width: 120px; }
.kind-count { color: var(--text-secondary); min-width: 80px; }
.kind-bytes { color: var(--text-secondary); }

.loading, .error { padding: 2rem; text-align: center; }
.error { color: var(--danger); }

</style>
