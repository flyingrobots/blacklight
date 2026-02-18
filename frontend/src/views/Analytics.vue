<template>
  <div class="analytics-view">
    <h2>Analytics</h2>

    <div v-if="loading" class="loading">Loading...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <template v-else>
      <div class="section" v-if="dailyStats.length">
        <h3>Daily Activity</h3>
        <DailyChart :data="dailyStats" />
      </div>

      <div class="section" v-if="modelUsage.length">
        <h3>Model Usage</h3>
        <table class="data-table">
          <thead>
            <tr>
              <th>Model</th>
              <th class="right">Input Tokens</th>
              <th class="right">Output Tokens</th>
              <th class="right">Cache Read</th>
              <th class="right">Cache Create</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="m in modelUsage" :key="m.model">
              <td><code>{{ m.model }}</code></td>
              <td class="right">{{ (m.input_tokens ?? 0).toLocaleString() }}</td>
              <td class="right">{{ (m.output_tokens ?? 0).toLocaleString() }}</td>
              <td class="right">{{ (m.cache_read_tokens ?? 0).toLocaleString() }}</td>
              <td class="right">{{ (m.cache_creation_tokens ?? 0).toLocaleString() }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="two-col">
        <div class="section" v-if="toolFrequency.length">
          <h3>Top Tools</h3>
          <div class="bar-chart">
            <div v-for="t in toolFrequency" :key="t.tool_name" class="bar-row">
              <span class="bar-label">{{ t.tool_name }}</span>
              <div class="bar-track">
                <div class="bar-fill" :style="{ width: barWidth(t.call_count) }"></div>
              </div>
              <span class="bar-value">{{ t.call_count.toLocaleString() }}</span>
            </div>
          </div>
        </div>

        <div class="section" v-if="outcomes.length">
          <h3>Outcomes</h3>
          <div class="bar-chart">
            <div v-for="o in outcomes" :key="o.outcome" class="bar-row">
              <span class="bar-label">{{ o.outcome }}</span>
              <div class="bar-track">
                <div class="bar-fill outcome" :style="{ width: outcomeWidth(o.count) }"></div>
              </div>
              <span class="bar-value">{{ o.count }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="section" v-if="projects.length">
        <h3>Projects</h3>
        <table class="data-table">
          <thead>
            <tr>
              <th>Project</th>
              <th class="right">Sessions</th>
              <th class="right">Messages</th>
              <th class="right">Tool Calls</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="p in projects" :key="p.project_slug">
              <td>{{ p.project_slug }}</td>
              <td class="right">{{ p.session_count.toLocaleString() }}</td>
              <td class="right">{{ p.message_count.toLocaleString() }}</td>
              <td class="right">{{ p.tool_call_count.toLocaleString() }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api } from '@/api/client'
import type { DailyStats, ModelUsage, ToolFrequency, ProjectBreakdown, OutcomeStats } from '@/types'
import DailyChart from '@/components/DailyChart.vue'

const loading = ref(true)
const error = ref('')
const dailyStats = ref<DailyStats[]>([])
const modelUsage = ref<ModelUsage[]>([])
const toolFrequency = ref<ToolFrequency[]>([])
const projects = ref<ProjectBreakdown[]>([])
const outcomes = ref<OutcomeStats[]>([])

const maxToolCount = computed(() => Math.max(...toolFrequency.value.map(t => t.call_count), 1))
const maxOutcomeCount = computed(() => Math.max(...outcomes.value.map(o => o.count), 1))

function barWidth(count: number): string {
  return `${(count / maxToolCount.value) * 100}%`
}

function outcomeWidth(count: number): string {
  return `${(count / maxOutcomeCount.value) * 100}%`
}

onMounted(async () => {
  try {
    const [d, m, t, p, o] = await Promise.all([
      api.analytics.daily(),
      api.analytics.models(),
      api.analytics.tools({ limit: 15 }),
      api.analytics.projects(),
      api.analytics.outcomes(),
    ])
    dailyStats.value = d
    modelUsage.value = m
    toolFrequency.value = t
    projects.value = p
    outcomes.value = o
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.analytics-view h2 { margin-bottom: 1.5rem; }
.section { margin-bottom: 2rem; }
.section h3 { margin-bottom: 1rem; color: var(--bl-text-2); }
.two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 2rem; }
@media (max-width: 800px) { .two-col { grid-template-columns: 1fr; } }
.data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--bl-text-md);
}
.data-table th, .data-table td {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--bl-border);
  text-align: left;
}
.data-table th { color: var(--bl-text-2); font-weight: 500; }
.data-table .right { text-align: right; }
.bar-chart { display: flex; flex-direction: column; gap: 0.5rem; }
.bar-row { display: flex; align-items: center; gap: 0.75rem; font-size: var(--bl-text-sm); }
.bar-label { width: 140px; flex-shrink: 0; color: var(--bl-text-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.bar-track { flex: 1; height: 20px; background: var(--bl-bg-3); border-radius: var(--bl-radius-sm); overflow: hidden; }
.bar-fill { height: 100%; background: var(--bl-accent); border-radius: var(--bl-radius-sm); transition: width 0.3s; }
.bar-fill.outcome { background: var(--bl-success); }
.bar-value { width: 60px; text-align: right; flex-shrink: 0; }
.loading, .error { padding: 2rem; text-align: center; }
.error { color: var(--bl-danger); }
</style>
