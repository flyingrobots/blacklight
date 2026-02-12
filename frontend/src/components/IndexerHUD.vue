<template>
  <div
    ref="hudRef"
    :class="['hud', expanded ? 'hud-expanded' : 'hud-collapsed', `hud-status-${activeStatus}`]"
    @click="onHudClick"
  >
    <!-- Collapsed pill -->
    <div v-if="!expanded" class="hud-pill">
      <span v-if="anyActive" class="hud-spinner"></span>
      <span v-else-if="indexerStatus.status === 'paused'" class="hud-icon hud-icon-paused">&#9646;&#9646;</span>
      <span v-else class="hud-icon">&#9707;</span>
      <span v-if="anyActive || indexerStatus.status === 'paused'" class="hud-pill-text">
        {{ pillLabel }}
      </span>
    </div>

    <!-- Expanded card -->
    <div v-if="expanded" class="hud-card" @click.stop>
      <div class="hud-card-header" @click="collapse">
        <div class="hud-tabs">
          <button
            v-for="tab in tabs"
            :key="tab"
            :class="['hud-tab', { active: activeTab === tab }]"
            @click.stop="activeTab = tab"
          >{{ tab }}</button>
        </div>
        <button class="hud-close" @click.stop="collapse">&times;</button>
      </div>

      <!-- INDEXER TAB -->
      <div v-if="activeTab === 'Indexer'">
        <div class="hud-status-row">
          <span :class="['status-badge', `status-${indexerStatus.status}`]">{{ indexerStatus.status }}</span>
        </div>

        <div v-if="indexerIsActive || indexerStatus.status === 'paused'" class="hud-progress">
          <div class="progress-phase">{{ indexerStatus.progress.phase }}</div>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: indexerPct + '%' }"></div>
          </div>
          <div class="progress-stats">
            <span>{{ indexerStatus.progress.files_done }}/{{ indexerStatus.progress.files_total }} files</span>
            <span>{{ indexerStatus.progress.messages_processed.toLocaleString() }} msgs</span>
            <span>{{ indexerStatus.progress.blobs_inserted.toLocaleString() }} blobs</span>
          </div>
        </div>

        <div class="hud-controls">
          <template v-if="indexerIsActive">
            <button class="btn btn-warning" @click="doPause">Pause</button>
            <button class="btn btn-danger" @click="doStop">Stop</button>
          </template>
          <template v-else-if="indexerStatus.status === 'paused'">
            <button class="btn btn-primary" @click="doResume">Resume</button>
            <button class="btn btn-danger" @click="doStop">Stop</button>
          </template>
          <template v-else>
            <button class="btn btn-primary" @click="doStart(false)">Re-index</button>
            <button class="btn btn-secondary" @click="doStart(true)">Full Re-index</button>
          </template>
        </div>

        <div v-if="indexerStatus.latest_report" class="hud-report">
          <div class="report-title">Last run: {{ indexerStatus.latest_report.elapsed_secs.toFixed(1) }}s</div>
          <div class="report-stats">
            <span>{{ indexerStatus.latest_report.sessions_parsed }} sessions</span>
            <span>{{ indexerStatus.latest_report.messages_processed.toLocaleString() }} messages</span>
            <span>{{ indexerStatus.latest_report.blobs_inserted.toLocaleString() }} blobs</span>
            <span>{{ indexerStatus.latest_report.files_processed }} files</span>
            <span>{{ indexerStatus.latest_report.tool_calls_inserted.toLocaleString() }} tool calls</span>
          </div>
        </div>

        <div v-if="indexerStatus.error_message" class="hud-error">
          {{ indexerStatus.error_message }}
        </div>
      </div>

      <!-- ENRICHMENT TAB -->
      <div v-if="activeTab === 'Enrichment'">
        <div class="hud-status-row">
          <span :class="['status-badge', `status-${enricherStatus.status}`]">{{ enricherStatus.status }}</span>
        </div>

        <div v-if="enricherIsActive" class="hud-progress">
          <div class="progress-phase">Enriching sessions...</div>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: enricherPct + '%' }"></div>
          </div>
          <div class="progress-stats">
            <span>{{ enricherStatus.sessions_done }}/{{ enricherStatus.sessions_total }} done</span>
            <span v-if="enricherStatus.sessions_failed">{{ enricherStatus.sessions_failed }} failed</span>
          </div>
        </div>

        <div class="hud-controls">
          <template v-if="enricherIsActive">
            <button class="btn btn-danger" @click="doEnrichStop">Stop</button>
          </template>
          <template v-else>
            <button class="btn btn-primary" @click="doEnrichStart(false)">Enrich</button>
            <button class="btn btn-secondary" @click="doEnrichStart(true)">Force All</button>
          </template>
        </div>

        <div v-if="enricherStatus.latest_report" class="hud-report">
          <div class="report-title">Last run</div>
          <div class="report-stats">
            <span>{{ enricherStatus.latest_report.enriched }} enriched</span>
            <span>{{ enricherStatus.latest_report.skipped }} skipped</span>
            <span>{{ enricherStatus.latest_report.failed }} failed</span>
            <span>{{ enricherStatus.latest_report.total_candidates }} candidates</span>
          </div>
        </div>

        <div v-if="enricherStatus.error_message" class="hud-error">
          {{ enricherStatus.error_message }}
        </div>
      </div>

      <!-- SCHEDULE TAB -->
      <div v-if="activeTab === 'Schedule'">
        <div class="schedule-form">
          <label class="schedule-row">
            <span>Enabled</span>
            <input type="checkbox" v-model="scheduleForm.enabled" />
          </label>
          <label class="schedule-row">
            <span>Interval (min)</span>
            <input type="number" v-model.number="scheduleForm.interval_minutes" min="1" class="schedule-input" />
          </label>
          <label class="schedule-row">
            <span>Run enrichment</span>
            <input type="checkbox" v-model="scheduleForm.run_enrichment" />
          </label>
          <label class="schedule-row">
            <span>Concurrency</span>
            <input type="number" v-model.number="scheduleForm.enrichment_concurrency" min="1" max="20" class="schedule-input" />
          </label>
          <div class="hud-controls">
            <button class="btn btn-primary" @click="saveSchedule">Save</button>
          </div>
          <div v-if="scheduleSaved" class="schedule-saved">Saved</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { gsap } from 'gsap'
// @ts-ignore - macOS case-insensitive FS conflict between flip.d.ts and gsap/Flip module declaration
import { Flip } from 'gsap/Flip'
import { api } from '@/api/client'
import type { IndexerStatusResponse, EnricherStatusResponse, ScheduleConfig } from '@/types'

gsap.registerPlugin(Flip)

const hudRef = ref<HTMLElement>()
const expanded = ref(false)
const tabs = ['Indexer', 'Enrichment', 'Schedule'] as const
type Tab = typeof tabs[number]
const activeTab = ref<Tab>('Indexer')

const indexerStatus = ref<IndexerStatusResponse>({
  status: 'idle',
  progress: { phase: '', files_total: 0, files_done: 0, messages_processed: 0, blobs_inserted: 0 },
  latest_report: null,
  error_message: null,
})

const enricherStatus = ref<EnricherStatusResponse>({
  status: 'idle',
  sessions_total: 0,
  sessions_done: 0,
  sessions_failed: 0,
  latest_report: null,
  error_message: null,
})

const scheduleForm = ref({
  enabled: true,
  interval_minutes: 60,
  run_enrichment: true,
  enrichment_concurrency: 5,
})
const scheduleSaved = ref(false)

let pollTimer: ReturnType<typeof setInterval> | null = null

const indexerIsActive = computed(() => indexerStatus.value.status === 'running')
const enricherIsActive = computed(() => enricherStatus.value.status === 'running')
const anyActive = computed(() => indexerIsActive.value || enricherIsActive.value)

const activeStatus = computed(() => {
  if (enricherIsActive.value) return enricherStatus.value.status
  return indexerStatus.value.status
})

const indexerPct = computed(() => {
  const p = indexerStatus.value.progress
  if (p.files_total === 0) return 0
  return Math.min(100, (p.files_done / p.files_total) * 100)
})

const enricherPct = computed(() => {
  const s = enricherStatus.value
  if (s.sessions_total === 0) return 0
  return Math.min(100, ((s.sessions_done + s.sessions_failed) / s.sessions_total) * 100)
})

const pillLabel = computed(() => {
  if (enricherIsActive.value) {
    const pct = enricherPct.value
    return `Enriching ${pct.toFixed(0)}%`
  }
  const p = indexerStatus.value.progress
  const pct = indexerPct.value
  if (indexerStatus.value.status === 'paused') return `Paused ${pct.toFixed(0)}%`
  if (p.phase) return `${p.phase} ${pct.toFixed(0)}%`
  return 'Running...'
})

async function pollStatus() {
  try {
    indexerStatus.value = await api.indexer.status()
  } catch { /* ignore */ }
  try {
    enricherStatus.value = await api.enrichment.status()
  } catch { /* ignore */ }
}

function startPolling() {
  if (pollTimer) return
  pollTimer = setInterval(pollStatus, 1000)
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

async function flipTo(newExpanded: boolean) {
  if (!hudRef.value) {
    expanded.value = newExpanded
    return
  }
  const state = Flip.getState(hudRef.value)
  expanded.value = newExpanded
  await nextTick()
  Flip.from(state, {
    duration: 0.35,
    ease: 'power2.inOut',
    absolute: true,
  })
}

function onHudClick() {
  if (!expanded.value) flipTo(true)
}

function collapse() {
  flipTo(false)
}

// Indexer actions
async function doStart(full: boolean) {
  try {
    await api.indexer.start(full)
    indexerStatus.value.status = 'running'
    indexerStatus.value.progress = { phase: 'Starting...', files_total: 0, files_done: 0, messages_processed: 0, blobs_inserted: 0 }
    indexerStatus.value.latest_report = null
    indexerStatus.value.error_message = null
    startPolling()
  } catch (e: any) {
    indexerStatus.value.error_message = e.message
  }
}

async function doStop() {
  try {
    await api.indexer.stop()
  } catch (e: any) {
    indexerStatus.value.error_message = e.message
  }
}

async function doPause() {
  try {
    await api.indexer.pause()
    indexerStatus.value.status = 'paused'
  } catch (e: any) {
    indexerStatus.value.error_message = e.message
  }
}

async function doResume() {
  try {
    await api.indexer.resume()
    indexerStatus.value.status = 'running'
  } catch (e: any) {
    indexerStatus.value.error_message = e.message
  }
}

// Enricher actions
async function doEnrichStart(force: boolean) {
  try {
    await api.enrichment.start({ force })
    enricherStatus.value.status = 'running'
    enricherStatus.value.sessions_total = 0
    enricherStatus.value.sessions_done = 0
    enricherStatus.value.sessions_failed = 0
    enricherStatus.value.latest_report = null
    enricherStatus.value.error_message = null
    startPolling()
  } catch (e: any) {
    enricherStatus.value.error_message = e.message
  }
}

async function doEnrichStop() {
  try {
    await api.enrichment.stop()
  } catch (e: any) {
    enricherStatus.value.error_message = e.message
  }
}

// Schedule actions
async function loadSchedule() {
  try {
    const config = await api.schedule.get()
    scheduleForm.value = {
      enabled: config.enabled,
      interval_minutes: config.interval_minutes,
      run_enrichment: config.run_enrichment,
      enrichment_concurrency: config.enrichment_concurrency,
    }
  } catch { /* ignore */ }
}

async function saveSchedule() {
  try {
    await api.schedule.update(scheduleForm.value)
    scheduleSaved.value = true
    setTimeout(() => { scheduleSaved.value = false }, 2000)
  } catch (e: any) {
    enricherStatus.value.error_message = e.message
  }
}

onMounted(async () => {
  await pollStatus()
  loadSchedule()
  startPolling()
})

onUnmounted(() => {
  stopPolling()
})
</script>

<style scoped>
.hud {
  position: fixed;
  bottom: 1.5rem;
  right: 1.5rem;
  z-index: 9000;
  font-size: 0.8125rem;
}

/* Collapsed pill */
.hud-collapsed {
  cursor: pointer;
}
.hud-pill {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 20px;
  padding: 0.4rem 0.75rem;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.3);
  transition: border-color 0.2s;
  white-space: nowrap;
}
.hud-collapsed:hover .hud-pill {
  border-color: var(--accent);
}
.hud-status-running .hud-pill,
.hud-status-paused .hud-pill {
  border-color: var(--accent);
}
.hud-icon {
  font-size: 1rem;
  color: var(--text-secondary);
  line-height: 1;
}
.hud-icon-paused {
  color: #d97706;
  font-size: 0.7rem;
  letter-spacing: -2px;
}
.hud-pill-text {
  color: var(--text-secondary);
  font-size: 0.75rem;
  font-weight: 500;
}

/* Spinner */
.hud-spinner {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: hud-spin 0.8s linear infinite;
}
@keyframes hud-spin {
  to { transform: rotate(360deg); }
}

/* Expanded card */
.hud-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 1rem;
  width: 360px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.4);
}
.hud-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
  cursor: pointer;
}
.hud-close {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 1.25rem;
  cursor: pointer;
  padding: 0 0.25rem;
  line-height: 1;
}
.hud-close:hover {
  color: var(--text);
}

/* Tabs */
.hud-tabs {
  display: flex;
  gap: 0.25rem;
}
.hud-tab {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 0.75rem;
  font-weight: 500;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  cursor: pointer;
  transition: color 0.15s, background 0.15s;
}
.hud-tab:hover {
  color: var(--text);
  background: var(--bg-tertiary);
}
.hud-tab.active {
  color: var(--accent);
  background: var(--bg-tertiary);
}

/* Status row */
.hud-status-row {
  margin-bottom: 0.75rem;
}

/* Status badges */
.status-badge {
  display: inline-block;
  padding: 0.2rem 0.6rem;
  border-radius: 12px;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
}
.status-idle { background: var(--bg-tertiary); color: var(--text-secondary); }
.status-running { background: #1d4ed8; color: #fff; }
.status-paused { background: #d97706; color: #fff; }
.status-completed { background: var(--success); color: #fff; }
.status-failed { background: var(--danger); color: #fff; }
.status-cancelled { background: #d97706; color: #fff; }

/* Progress */
.hud-progress {
  margin-bottom: 0.75rem;
}
.progress-phase {
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-bottom: 0.375rem;
}
.progress-bar {
  height: 6px;
  background: var(--bg-tertiary);
  border-radius: 3px;
  overflow: hidden;
  margin-bottom: 0.375rem;
}
.progress-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 3px;
  transition: width 0.5s ease;
}
.progress-stats {
  display: flex;
  gap: 0.75rem;
  font-size: 0.6875rem;
  color: var(--text-secondary);
}

/* Controls */
.hud-controls {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
}
.btn {
  padding: 0.3rem 0.65rem;
  border: none;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
}
.btn-primary { background: var(--accent); color: #fff; }
.btn-primary:hover { opacity: 0.9; }
.btn-secondary { background: var(--bg-tertiary); color: var(--text); }
.btn-secondary:hover { opacity: 0.85; }
.btn-warning { background: #d97706; color: #fff; }
.btn-warning:hover { opacity: 0.9; }
.btn-danger { background: var(--danger); color: #fff; }
.btn-danger:hover { opacity: 0.9; }

/* Report */
.hud-report {
  border-top: 1px solid var(--border);
  padding-top: 0.5rem;
  margin-bottom: 0.25rem;
}
.report-title {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text);
  margin-bottom: 0.25rem;
}
.report-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  font-size: 0.6875rem;
  color: var(--text-secondary);
}

/* Error */
.hud-error {
  margin-top: 0.5rem;
  padding: 0.375rem 0.5rem;
  background: rgba(239, 68, 68, 0.1);
  border-radius: 4px;
  font-size: 0.75rem;
  color: var(--danger);
}

/* Schedule form */
.schedule-form {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.schedule-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.8125rem;
  color: var(--text);
}
.schedule-row input[type="checkbox"] {
  accent-color: var(--accent);
}
.schedule-input {
  width: 80px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text);
  font-size: 0.8125rem;
  padding: 0.25rem 0.4rem;
  text-align: right;
}
.schedule-saved {
  font-size: 0.75rem;
  color: var(--success);
  text-align: center;
}
</style>
