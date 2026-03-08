<template>
  <div class="insights-view">
    <h1>Insights</h1>

    <!-- Tab bar -->
    <div class="tab-bar">
      <button
        v-for="tab in tabs"
        :key="tab"
        :class="['tab-btn', { active: activeTab === tab }]"
        @click="activeTab = tab"
      >{{ tab }}</button>
    </div>

    <div v-if="loading" class="loading-state"><div class="spinner"></div></div>

    <template v-else>
      <!-- Activity -->
      <div v-if="activeTab === 'Activity'" class="tab-content">
        <div class="date-filters">
          <button
            v-for="opt in timeOptions"
            :key="opt.label"
            :class="['time-btn', { active: activeTime === opt.label }]"
            @click="selectTime(opt)"
          >{{ opt.label }}</button>
        </div>
        <div class="chart-card" v-if="dailyStats.length">
          <DailyChart :data="dailyStats" />
        </div>
        <div v-else class="empty-section">No activity data available.</div>
      </div>

      <!-- Models -->
      <div v-if="activeTab === 'Models'" class="tab-content">
        <div v-if="modelUsage.length" class="table-wrap">
          <table class="data-table">
            <thead>
              <tr>
                <th>Model</th>
                <th class="r">Input Tokens</th>
                <th class="r">Output Tokens</th>
                <th class="r">Cache Read</th>
                <th class="r">Cache Create</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="m in modelUsage" :key="m.model">
                <td><code>{{ m.model }}</code></td>
                <td class="r">{{ (m.input_tokens ?? 0).toLocaleString() }}</td>
                <td class="r">{{ (m.output_tokens ?? 0).toLocaleString() }}</td>
                <td class="r">{{ (m.cache_read_tokens ?? 0).toLocaleString() }}</td>
                <td class="r">{{ (m.cache_creation_tokens ?? 0).toLocaleString() }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-else class="empty-section">No model usage data.</div>
      </div>

      <!-- Tools -->
      <div v-if="activeTab === 'Tools'" class="tab-content">
        <div v-if="toolFrequency.length" class="bar-list">
          <div v-for="t in toolFrequency" :key="t.tool_name" class="bar-row">
            <span class="bar-label">{{ t.tool_name }}</span>
            <div class="bar-track">
              <div class="bar-fill" :style="{ width: barWidth(t.call_count) }"></div>
            </div>
            <span class="bar-value">{{ t.call_count.toLocaleString() }}</span>
          </div>
        </div>
        <div v-else class="empty-section">No tool usage data.</div>
      </div>

      <!-- Outcomes -->
      <div v-if="activeTab === 'Outcomes'" class="tab-content">
        <div v-if="outcomes.length" class="bar-list">
          <div v-for="o in outcomes" :key="o.outcome" class="bar-row">
            <span class="bar-label">{{ o.outcome }}</span>
            <div class="bar-track">
              <div class="bar-fill bar-fill-green" :style="{ width: outcomeWidth(o.count) }"></div>
            </div>
            <span class="bar-value">{{ o.count }}</span>
          </div>
        </div>
        <div v-else class="empty-section">No outcome data.</div>
      </div>

      <!-- Storage -->
      <div v-if="activeTab === 'Storage'" class="tab-content">
        <template v-if="storage">
          <div class="storage-stats">
            <div class="storage-stat">
              <span class="ss-val">{{ storage.unique_blobs.toLocaleString() }}</span>
              <span class="ss-lbl">Unique Blobs</span>
            </div>
            <div class="storage-stat">
              <span class="ss-val">{{ formatBytes(storage.total_bytes) }}</span>
              <span class="ss-lbl">Total Content</span>
            </div>
            <div class="storage-stat">
              <span class="ss-val">{{ storage.total_references.toLocaleString() }}</span>
              <span class="ss-lbl">References</span>
            </div>
            <div class="storage-stat">
              <span class="ss-val">{{ (storage.dedup_ratio * 100).toFixed(1) }}%</span>
              <span class="ss-lbl">Dedup Ratio</span>
            </div>
          </div>
          <div v-if="storage.by_kind.length" class="table-wrap">
            <table class="data-table">
              <thead>
                <tr>
                  <th>Kind</th>
                  <th class="r">Blobs</th>
                  <th class="r">Size</th>
                  <th class="r">%</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="k in storage.by_kind" :key="k.kind">
                  <td><code>{{ k.kind }}</code></td>
                  <td class="r">{{ k.blob_count.toLocaleString() }}</td>
                  <td class="r">{{ formatBytes(k.total_bytes) }}</td>
                  <td class="r">{{ ((k.total_bytes / storage.total_bytes) * 100).toFixed(1) }}%</td>
                </tr>
              </tbody>
            </table>
          </div>
        </template>
        <div v-else class="empty-section">No storage data.</div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api } from '@/api/client'
import type { DailyStats, ModelUsage, ToolFrequency, OutcomeStats, StorageOverview } from '@/types'
import DailyChart from '@/components/DailyChart.vue'

const tabs = ['Activity', 'Models', 'Tools', 'Outcomes', 'Storage'] as const
type Tab = typeof tabs[number]
const activeTab = ref<Tab>('Activity')
const loading = ref(true)
const activeTime = ref('All')

const dailyStats = ref<DailyStats[]>([])
const modelUsage = ref<ModelUsage[]>([])
const toolFrequency = ref<ToolFrequency[]>([])
const outcomes = ref<OutcomeStats[]>([])
const storage = ref<StorageOverview | null>(null)

const timeOptions = [
  { label: '30D', days: 30 },
  { label: '90D', days: 90 },
  { label: '6M', days: 180 },
  { label: '1Y', days: 365 },
  { label: 'All', days: undefined as number | undefined },
]

const maxToolCount = computed(() => Math.max(...toolFrequency.value.map(t => t.call_count), 1))
const maxOutcomeCount = computed(() => Math.max(...outcomes.value.map(o => o.count), 1))

function barWidth(count: number): string {
  return `${(count / maxToolCount.value) * 100}%`
}

function outcomeWidth(count: number): string {
  return `${(count / maxOutcomeCount.value) * 100}%`
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
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
  try {
    dailyStats.value = await api.analytics.daily({ from })
  } catch { /* ignore */ }
}

onMounted(async () => {
  try {
    const [d, m, t, o, s] = await Promise.all([
      api.analytics.daily(),
      api.analytics.models(),
      api.analytics.tools({ limit: 15 }),
      api.analytics.outcomes(),
      api.storage(),
    ])
    dailyStats.value = d
    modelUsage.value = m
    toolFrequency.value = t
    outcomes.value = o
    storage.value = s
  } catch (e: any) {
    console.error(e)
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.insights-view h1 {
  font-size: var(--bl-text-xl);
  margin-bottom: 1rem;
}

.tab-bar {
  display: flex;
  gap: 2px;
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  padding: 3px;
  margin-bottom: 1.5rem;
  width: fit-content;
}

.tab-btn {
  padding: 0.375rem 1rem;
  background: none;
  border: none;
  border-radius: var(--bl-radius-sm);
  font-size: var(--bl-text-sm);
  font-weight: 500;
  color: var(--bl-text-2);
  cursor: pointer;
  transition: all 0.15s;
}

.tab-btn:hover { color: var(--bl-text); }
.tab-btn.active { background: var(--bl-surface-3); color: var(--bl-text); }

.tab-content {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.date-filters {
  display: flex;
  gap: 2px;
  background: var(--bl-surface-2);
  border-radius: var(--bl-radius-md);
  padding: 2px;
  width: fit-content;
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
}

.time-btn:hover { color: var(--bl-text); }
.time-btn.active { background: var(--bl-surface-3); color: var(--bl-text); }

.chart-card {
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  padding: 1rem;
}

/* Tables */
.table-wrap {
  overflow-x: auto;
}

.data-table {
  width: 100%;
  font-size: var(--bl-text-sm);
}

.data-table th, .data-table td {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--bl-border);
  text-align: left;
}

.data-table th {
  color: var(--bl-text-2);
  font-weight: 500;
  font-size: var(--bl-text-xs);
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.data-table .r { text-align: right; }
.data-table code { font-size: var(--bl-text-xs); color: var(--bl-accent); }

/* Bar charts */
.bar-list {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  max-width: 700px;
}

.bar-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  font-size: var(--bl-text-sm);
}

.bar-label {
  width: 140px;
  flex-shrink: 0;
  color: var(--bl-text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--bl-text-xs);
}

.bar-track {
  flex: 1;
  height: 16px;
  background: var(--bl-surface-2);
  border-radius: 3px;
  overflow: hidden;
}

.bar-fill {
  height: 100%;
  background: var(--bl-accent);
  border-radius: 3px;
  transition: width 0.3s;
  opacity: 0.8;
}

.bar-fill-green {
  background: var(--bl-success);
}

.bar-value {
  width: 60px;
  text-align: right;
  flex-shrink: 0;
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
  font-variant-numeric: tabular-nums;
}

/* Storage */
.storage-stats {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 0.75rem;
}

.storage-stat {
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  padding: 1rem;
}

.ss-val {
  display: block;
  font-size: var(--bl-text-xl);
  font-weight: 600;
  color: var(--bl-accent);
}

.ss-lbl {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
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

.empty-section {
  text-align: center;
  padding: 2rem;
  color: var(--bl-text-2);
}
</style>
