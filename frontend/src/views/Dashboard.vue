<template>
  <div class="dashboard">
    <div v-if="loading && !overview" class="loading">Loading dashboard...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <template v-else>
      <!-- Top: Recent Sessions -->
      <section class="section recent-section" v-if="recentSessions.length">
        <div class="section-header">
          <h3>Recent Sessions</h3>
          <router-link to="/sessions" class="nav-hint">View All →</router-link>
        </div>
        <div class="session-list">
          <SessionCard
            v-for="session in recentSessions"
            :key="session.id"
            :session="session"
          />
        </div>
      </section>

      <!-- Middle Row: Overview Heatmap -->
      <section class="section activity-section">
        <div class="section-header">
          <h3>Daily Activity</h3>
          <span class="nav-hint">Sessions per day</span>
        </div>
        <ActivityHeatmap :data="dailyStats" :project-data="dailyProjects" />
      </section>

      <!-- Middle: Time Window Controller -->
      <section class="section controls-section">
        <TimeSlider @change="onTimeWindowChange" />
      </section>

      <!-- Grid: Analytics Breakdown -->
      <div class="analytics-grid">
        <DashboardBarChart
          v-if="projectChartData.length"
          title="Sessions per Project"
          :data="projectChartData"
          color="var(--bl-accent)"
        />
        <DashboardBarChart
          v-if="llmSessionData.length"
          title="Sessions per LLM"
          :data="llmSessionData"
          color="var(--bl-success)"
        />
        <DashboardBarChart
          v-if="llmMessageData.length"
          title="Messages per LLM"
          :data="llmMessageData"
          color="var(--bl-purple)"
        />
        <DashboardBarChart
          v-if="llmToolData.length"
          title="Tools per LLM"
          :data="llmToolData"
          color="var(--bl-warning)"
        />
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { api } from '@/api/client'
import type { AnalyticsOverview, SessionSummary, DailyStats, ProjectBreakdown, LlmBreakdown, DailyProjectStats } from '@/types'
import SessionCard from '@/components/SessionCard.vue'
import ActivityHeatmap from '@/components/ActivityHeatmap.vue'
import DashboardBarChart from '@/components/DashboardBarChart.vue'
import TimeSlider, { type TimeOption } from '@/components/TimeSlider.vue'

const loading = ref(true)
const error = ref('')
const overview = ref<AnalyticsOverview | null>(null)
const recentSessions = ref<SessionSummary[]>([])
const projects = ref<ProjectBreakdown[]>([])
const llmStats = ref<LlmBreakdown[]>([])
const dailyStats = ref<DailyStats[]>([])
const dailyProjects = ref<DailyProjectStats[]>([])

const projectChartData = computed(() => 
  projects.value
    .filter(p => p.session_count > 0)
    .map(p => ({ label: p.project_slug, value: p.session_count }))
)

const llmSessionData = computed(() => 
  llmStats.value
    .filter(l => l.session_count > 0)
    .map(l => ({ label: l.source_kind, value: l.session_count }))
)

const llmMessageData = computed(() => 
  llmStats.value
    .filter(l => l.message_count > 0)
    .map(l => ({ label: l.source_kind, value: l.message_count }))
)

const llmToolData = computed(() => 
  llmStats.value
    .filter(l => l.tool_call_count > 0)
    .map(l => ({ label: l.source_kind, value: l.tool_call_count }))
)

async function fetchHeatmap(from?: string) {
  let effectiveFrom = from
  if (!effectiveFrom) {
    // Default to last 6 months if 'All Time' is selected
    const sixMonthsAgo = new Date()
    sixMonthsAgo.setMonth(sixMonthsAgo.getMonth() - 6)
    effectiveFrom = sixMonthsAgo.toISOString().split('T')[0]
  }
  
  const [ds, dp] = await Promise.all([
    api.analytics.daily({ from: effectiveFrom }),
    api.analytics.dailyProjects({ from: effectiveFrom })
  ])
  dailyStats.value = ds
  dailyProjects.value = dp
}

async function fetchData(from?: string) {
  loading.value = true
  try {
    const [ov, proj, ls, sessions] = await Promise.all([
      api.analytics.overview(),
      api.analytics.projects({ from }),
      api.analytics.llms({ from }),
      api.sessions.list({ limit: 5 }),
    ])
    overview.value = ov
    projects.value = proj
    llmStats.value = ls
    recentSessions.value = sessions.items
    
    // Also update heatmap
    await fetchHeatmap(from)
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

function onTimeWindowChange(option: TimeOption) {
  let from: string | undefined
  if (option.days) {
    const d = new Date()
    d.setDate(d.getDate() - option.days)
    from = d.toISOString().split('T')[0]
  }
  fetchData(from)
}

onMounted(() => {
  // TimeSlider will trigger fetchData(7d) on mount due to immediate: true watch
  // but we add a safety check here to ensure we don't stay in loading state
  setTimeout(() => {
    if (loading.value && !overview.value) {
      const d = new Date()
      d.setDate(d.getDate() - 7)
      fetchData(d.toISOString().split('T')[0])
    }
  }, 1000)
})
</script>

<style scoped>
.dashboard {
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

.analytics-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
  gap: 1.5rem;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 1rem;
}

.section h3 {
  color: var(--bl-text-2);
  font-family: var(--bl-font-mono);
  text-transform: uppercase;
  font-size: var(--bl-text-sm);
  letter-spacing: 0.05em;
}

.nav-hint {
  font-family: var(--bl-font-mono);
  font-size: var(--bl-text-2xs);
  opacity: 0.6;
  text-decoration: none;
  color: var(--bl-accent);
}

.session-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.loading, .error {
  padding: 4rem;
  text-align: center;
  font-family: var(--bl-font-mono);
  font-size: var(--bl-text-md);
}

.error { color: var(--bl-danger); }

@media (max-width: 900px) {
  .analytics-grid {
    grid-template-columns: 1fr;
  }
}
</style>
