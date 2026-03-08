<template>
  <div class="home">
    <!-- Stats strip -->
    <div class="stats-strip" v-if="overview">
      <div class="stat-item">
        <span class="stat-value">{{ overview.total_sessions.toLocaleString() }}</span>
        <span class="stat-label">Sessions</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">{{ overview.total_messages.toLocaleString() }}</span>
        <span class="stat-label">Messages</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">{{ formatSize(overview.total_blob_bytes) }}</span>
        <span class="stat-label">Indexed</span>
      </div>
      <div class="stat-item" v-if="coverage">
        <span class="stat-value">{{ coverage.file_pct.toFixed(0) }}%</span>
        <span class="stat-label">Coverage</span>
      </div>
    </div>

    <!-- Heatmap -->
    <section class="section" v-if="dailyStats.length">
      <div class="section-header">
        <h2>Activity</h2>
        <div class="time-filters">
          <button
            v-for="opt in timeOptions"
            :key="opt.label"
            :class="['time-btn', { active: activeTime === opt.label }]"
            @click="selectTime(opt)"
          >{{ opt.label }}</button>
        </div>
      </div>
      <div class="heatmap-card">
        <ActivityHeatmap :data="dailyStats" :project-data="dailyProjects" />
      </div>
    </section>

    <!-- Recent sessions -->
    <section class="section">
      <div class="section-header">
        <h2>Recent Sessions</h2>
        <router-link to="/sessions" class="view-all">View all</router-link>
      </div>
      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
      </div>
      <div v-else-if="recentSessions.length" class="session-feed">
        <SessionCard
          v-for="session in recentSessions"
          :key="session.id"
          :session="session"
        />
      </div>
      <div v-else class="empty-state">
        <p>No sessions indexed yet.</p>
        <router-link to="/operations" class="empty-action">Run the indexer</router-link>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/api/client'
import type { AnalyticsOverview, SessionSummary, DailyStats, DailyProjectStats, IndexCoverage } from '@/types'
import SessionCard from '@/components/SessionCard.vue'
import ActivityHeatmap from '@/components/ActivityHeatmap.vue'

const loading = ref(true)
const overview = ref<AnalyticsOverview | null>(null)
const coverage = ref<IndexCoverage | null>(null)
const recentSessions = ref<SessionSummary[]>([])
const dailyStats = ref<DailyStats[]>([])
const dailyProjects = ref<DailyProjectStats[]>([])
const activeTime = ref('6M')

const timeOptions = [
  { label: '7D', days: 7 },
  { label: '30D', days: 30 },
  { label: '90D', days: 90 },
  { label: '6M', days: 180 },
  { label: '1Y', days: 365 },
  { label: 'All', days: undefined as number | undefined },
]

function formatSize(bytes: number): string {
  if (!bytes) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

function getFromDate(days?: number): string | undefined {
  if (!days) return undefined
  const d = new Date()
  d.setDate(d.getDate() - days)
  return d.toISOString().split('T')[0]
}

async function selectTime(opt: { label: string; days?: number }) {
  activeTime.value = opt.label
  const from = getFromDate(opt.days)
  const [ds, dp] = await Promise.all([
    api.analytics.daily({ from }),
    api.analytics.dailyProjects({ from }),
  ])
  dailyStats.value = ds
  dailyProjects.value = dp
}

onMounted(async () => {
  try {
    const [ov, cov, sessions] = await Promise.all([
      api.analytics.overview(),
      api.analytics.coverage(),
      api.sessions.list({ limit: 20 }),
    ])
    overview.value = ov
    coverage.value = cov
    recentSessions.value = sessions.items

    const from = getFromDate(180)
    const [ds, dp] = await Promise.all([
      api.analytics.daily({ from }),
      api.analytics.dailyProjects({ from }),
    ])
    dailyStats.value = ds
    dailyProjects.value = dp
  } catch (e: any) {
    console.error(e)
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.home {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.stats-strip {
  display: flex;
  gap: 2rem;
  padding: 1rem 0;
  border-bottom: 1px solid var(--bl-border);
}

.stat-item {
  display: flex;
  flex-direction: column;
}

.stat-value {
  font-size: var(--bl-text-2xl);
  font-weight: 700;
  color: var(--bl-text);
  line-height: 1.2;
}

.stat-label {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-top: 2px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
}

.section-header h2 {
  font-size: var(--bl-text-lg);
  font-weight: 600;
}

.view-all {
  font-size: var(--bl-text-sm);
  color: var(--bl-accent);
}

.time-filters {
  display: flex;
  gap: 2px;
  background: var(--bl-surface-2);
  border-radius: var(--bl-radius-md);
  padding: 2px;
}

.time-btn {
  background: none;
  border: none;
  padding: 0.25rem 0.625rem;
  font-size: var(--bl-text-xs);
  font-weight: 500;
  color: var(--bl-text-2);
  border-radius: var(--bl-radius-sm);
  cursor: pointer;
  transition: all 0.15s;
}

.time-btn:hover {
  color: var(--bl-text);
}

.time-btn.active {
  background: var(--bl-surface-3);
  color: var(--bl-text);
}

.heatmap-card {
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  padding: 1rem;
  overflow-x: auto;
}

.session-feed {
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: var(--bl-border);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  overflow: hidden;
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 3rem;
}

.spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--bl-border);
  border-top-color: var(--bl-accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

.empty-state {
  text-align: center;
  padding: 3rem;
  color: var(--bl-text-2);
}

.empty-action {
  display: inline-block;
  margin-top: 0.75rem;
  padding: 0.5rem 1rem;
  background: var(--bl-accent-dim);
  color: var(--bl-accent);
  border-radius: var(--bl-radius-md);
  font-size: var(--bl-text-sm);
  font-weight: 500;
}
</style>
