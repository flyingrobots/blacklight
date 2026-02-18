<template>
  <div class="dashboard">
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
          <div class="stat-value">{{ formatBytes(overview.db_size_bytes) }}</div>
          <div class="stat-label">DB Size</div>
        </div>
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
import { ref, onMounted } from 'vue'
import { api } from '@/api/client'
import type { AnalyticsOverview, SessionSummary } from '@/types'
import SessionCard from '@/components/SessionCard.vue'

const loading = ref(true)
const error = ref('')
const overview = ref<AnalyticsOverview | null>(null)
const recentSessions = ref<SessionSummary[]>([])

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

onMounted(async () => {
  try {
    const [ov, sessions] = await Promise.all([
      api.analytics.overview(),
      api.sessions.list({ limit: 3 }),
    ])
    overview.value = ov
    recentSessions.value = sessions.items
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
  margin-bottom: 2rem;
}
.stat-card {
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  padding: 1.25rem;
}
.stat-value {
  font-size: var(--bl-text-xl);
  font-weight: 600;
  color: var(--bl-accent);
}
.stat-label {
  color: var(--bl-text-2);
  font-size: var(--bl-text-md);
  margin-top: 0.25rem;
}
.section { margin-bottom: 2rem; }
.section h3 { margin-bottom: 1rem; color: var(--bl-text-2); }
.session-list { display: flex; flex-direction: column; gap: 0.75rem; }
.loading, .error { padding: 2rem; text-align: center; }
.error { color: var(--bl-danger); }
</style>
