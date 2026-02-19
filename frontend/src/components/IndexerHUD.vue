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

        <div class="hud-summary-row">
          <span>{{ coverageData ? coverageData.indexed_files.toLocaleString() + '/' + coverageData.source_files.toLocaleString() + ' files indexed' : '...' }}</span>
          <span class="hud-timestamp" v-if="indexerLastUpdated">Last updated {{ indexerLastUpdated }}</span>
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

        <div class="hud-summary-row">
          <span>{{ enrichmentSummary }}</span>
          <span class="hud-timestamp" v-if="enricherLastUpdated">Last updated {{ enricherLastUpdated }}</span>
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

      <!-- MIGRATION TAB -->
      <div v-if="activeTab === 'Migration'">
        <div class="hud-status-row">
          <span :class="['status-badge', `status-${migrationStatus.status}`]">{{ migrationStatus.status }}</span>
        </div>

        <div class="hud-summary-row">
          <span>V3 to V4 Migration</span>
        </div>

        <div v-if="migrationIsActive" class="hud-progress">
          <div class="progress-phase">Backing up and fingerprinting...</div>
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: migrationPct + '%' }"></div>
          </div>
          <div class="progress-stats">
            <span>{{ migrationStatus.progress.fingerprints_updated }}/{{ migrationStatus.progress.total_sessions }} sessions</span>
            <span>{{ migrationStatus.progress.backed_up }} backed up</span>
          </div>
        </div>

        <div class="hud-controls">
          <button v-if="!migrationIsActive" class="btn btn-primary" @click="doMigrationStart">Start Migration</button>
          <span v-else class="migration-running-msg">Migration in progress...</span>
        </div>

        <div v-if="migrationStatus.error_message" class="hud-error">
          {{ migrationStatus.error_message }}
        </div>
        
        <div class="hud-info-box">
          Migration ensures all sessions are backed up to CAS and cryptographically fingerprinted.
        </div>
      </div>

      <!-- LOGS TAB -->
      <div v-if="activeTab === 'Logs'" class="hud-logs-tab">
        <div class="logs-section" v-if="enricherLogs.length">
          <div class="logs-section-title">Enrichment</div>
          <div class="hud-log-viewer" ref="enricherLogEl">
            <div v-for="(line, i) in enricherLogs" :key="'e'+i" :class="['log-line', line.includes('Failed') ? 'log-error' : '']">{{ line }}</div>
          </div>
        </div>
        <div class="logs-section" v-if="indexerLogs.length">
          <div class="logs-section-title">Indexer</div>
          <div class="hud-log-viewer" ref="indexerLogEl">
            <div v-for="(line, i) in indexerLogs" :key="'i'+i" class="log-line">{{ line }}</div>
          </div>
        </div>
        <div v-if="!enricherLogs.length && !indexerLogs.length" class="logs-empty">
          No log output yet. Run the indexer or enricher to see output here.
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
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { gsap } from 'gsap'
// @ts-ignore - macOS case-insensitive FS conflict between flip.d.ts and gsap/Flip module declaration
import { Flip } from 'gsap/Flip'
import { api } from '@/api/client'
import type { IndexerStatusResponse, EnricherStatusResponse, IndexCoverage, AnalyticsOverview, ScheduleConfig } from '@/types'

gsap.registerPlugin(Flip)

const hudRef = ref<HTMLElement>()
const expanded = ref(false)
const tabs = ['Indexer', 'Enrichment', 'Migration', 'Logs', 'Schedule'] as const
type Tab = typeof tabs[number]
const activeTab = ref<Tab>('Indexer')

const coverageData = ref<IndexCoverage | null>(null)
const overviewData = ref<AnalyticsOverview | null>(null)
const indexerLastUpdated = ref('')
const enricherLastUpdated = ref('')
const enrichedCount = ref(0)
const totalSessionCount = ref(0)

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

const migrationStatus = ref<import('@/types').MigrationStatusResponse>({
  status: 'idle',
  progress: { total_sessions: 0, backed_up: 0, fingerprints_updated: 0 },
  error_message: null,
})

const scheduleForm = ref({
  enabled: true,
  interval_minutes: 60,
  run_enrichment: true,
  enrichment_concurrency: 5,
})
const scheduleSaved = ref(false)

const indexerLogs = ref<string[]>([])
const enricherLogs = ref<string[]>([])

let pollTimer: ReturnType<typeof setInterval> | null = null

const indexerIsActive = computed(() => indexerStatus.value.status === 'running')
const enricherIsActive = computed(() => enricherStatus.value.status === 'running')
const migrationIsActive = computed(() => migrationStatus.value.status === 'running')
const anyActive = computed(() => indexerIsActive.value || enricherIsActive.value || migrationIsActive.value)

const activeStatus = computed(() => {
  if (migrationIsActive.value) return migrationStatus.value.status
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

const migrationPct = computed(() => {
  const s = migrationStatus.value.progress
  if (s.total_sessions === 0) return 0
  return Math.min(100, (s.fingerprints_updated / s.total_sessions) * 100)
})

const enrichmentSummary = computed(() => {
  if (totalSessionCount.value === 0) return '...'
  return `${enrichedCount.value.toLocaleString()}/${totalSessionCount.value.toLocaleString()} sessions enriched`
})

function timeAgo(date: Date): string {
  const secs = Math.floor((Date.now() - date.getTime()) / 1000)
  if (secs < 60) return 'just now'
  const mins = Math.floor(secs / 60)
  if (mins < 60) return `${mins}m ago`
  const hrs = Math.floor(mins / 60)
  if (hrs < 24) return `${hrs}h ago`
  const days = Math.floor(hrs / 24)
  return `${days}d ago`
}

const pillLabel = computed(() => {
  if (migrationIsActive.value) {
    return `Migrating ${migrationPct.value.toFixed(0)}%`
  }
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

async function fetchCoverageAndOverview() {
  try {
    coverageData.value = await api.analytics.coverage()
    totalSessionCount.value = coverageData.value.total_sessions
    enrichedCount.value = coverageData.value.sessions_with_outcomes
  } catch { /* ignore */ }
  try {
    overviewData.value = await api.analytics.overview()
    totalSessionCount.value = overviewData.value.total_sessions
    if (overviewData.value.last_session) {
      indexerLastUpdated.value = timeAgo(new Date(overviewData.value.last_session))
    }
  } catch { /* ignore */ }
  // Use enrichment count from a lightweight query
  try {
    const pending = await api.enrichment.pendingCount()
    // pending.count is pending_review; enrichedCount is sessions_with_outcomes from coverage
    // We'll refine enricherLastUpdated from the enricher status
  } catch { /* ignore */ }
}

async function pollStatus() {
  try {
    const prev = indexerStatus.value.status
    indexerStatus.value = await api.indexer.status()
    // If indexer just finished, refresh coverage data
    if (prev === 'running' && indexerStatus.value.status !== 'running') {
      indexerLastUpdated.value = timeAgo(new Date())
      fetchCoverageAndOverview()
    }
  } catch { /* ignore */ }
  try {
    const prev = enricherStatus.value.status
    enricherStatus.value = await api.enrichment.status()
    if (prev === 'running' && enricherStatus.value.status !== 'running') {
      enricherLastUpdated.value = timeAgo(new Date())
      fetchCoverageAndOverview()
    }
  } catch { /* ignore */ }
  try {
    migrationStatus.value = await api.migration.status()
  } catch { /* ignore */ }
  // Fetch logs when the Logs tab is visible
  if (expanded.value && activeTab.value === 'Logs') {
    try { indexerLogs.value = await api.indexer.logs() } catch { /* ignore */ }
    try { enricherLogs.value = await api.enrichment.logs() } catch { /* ignore */ }
  }
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

// Migration actions
async function doMigrationStart() {
  try {
    await api.migration.start()
    migrationStatus.value.status = 'running'
    migrationStatus.value.progress = { total_sessions: 0, backed_up: 0, fingerprints_updated: 0 }
    migrationStatus.value.error_message = null
    startPolling()
  } catch (e: any) {
    migrationStatus.value.error_message = e.message
  }
}

// Fetch logs immediately when switching to Logs tab
watch(activeTab, async (tab) => {
  if (tab === 'Logs') {
    try { indexerLogs.value = await api.indexer.logs() } catch { /* ignore */ }
    try { enricherLogs.value = await api.enrichment.logs() } catch { /* ignore */ }
  }
})

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
  // Small delay to let backend wake up
  setTimeout(async () => {
    await pollStatus()
    await fetchCoverageAndOverview()
    loadSchedule()
    startPolling()
  }, 1000)
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
  font-size: var(--bl-text-sm);
}

/* Collapsed pill */
.hud-collapsed {
  cursor: pointer;
}
.hud-pill {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-pill);
  padding: 0.4rem 0.75rem;
  box-shadow: var(--bl-shadow-sm);
  transition: border-color 0.2s;
  white-space: nowrap;
}
.hud-collapsed:hover .hud-pill {
  border-color: var(--bl-accent);
}
.hud-status-running .hud-pill,
.hud-status-paused .hud-pill {
  border-color: var(--bl-accent);
}
.hud-icon {
  font-size: var(--bl-text-base);
  color: var(--bl-text-2);
  line-height: 1;
}
.hud-icon-paused {
  color: #d97706;
  font-size: 0.7rem;
  letter-spacing: -2px;
}
.hud-pill-text {
  color: var(--bl-text-2);
  font-size: var(--bl-text-xs);
  font-weight: 500;
}

/* Spinner */
.hud-spinner {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid var(--bl-border);
  border-top-color: var(--bl-accent);
  border-radius: var(--bl-radius-round);
  animation: hud-spin 0.8s linear infinite;
}
@keyframes hud-spin {
  to { transform: rotate(360deg); }
}

/* Expanded card */
.hud-card {
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-xl);
  padding: 1rem;
  width: 360px;
  box-shadow: var(--bl-shadow-lg);
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
  color: var(--bl-text-2);
  font-size: var(--bl-text-lg);
  cursor: pointer;
  padding: 0 0.25rem;
  line-height: 1;
}
.hud-close:hover {
  color: var(--bl-text);
}

/* Tabs */
.hud-tabs {
  display: flex;
  gap: 0.25rem;
}
.hud-tab {
  background: none;
  border: none;
  color: var(--bl-text-2);
  font-size: var(--bl-text-xs);
  font-weight: 500;
  padding: 0.25rem 0.5rem;
  border-radius: var(--bl-radius-sm);
  cursor: pointer;
  transition: color 0.15s, background 0.15s;
}
.hud-tab:hover {
  color: var(--bl-text);
  background: var(--bl-bg-3);
}
.hud-tab.active {
  color: var(--bl-accent);
  background: var(--bl-bg-3);
}

/* Status row */
.hud-status-row {
  margin-bottom: 0.75rem;
}

/* Summary row */
.hud-summary-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
  margin-bottom: 0.75rem;
  padding: 0.375rem 0.5rem;
  background: var(--bl-bg-3);
  border-radius: var(--bl-radius-md);
}
.hud-timestamp {
  font-size: 0.6875rem;
  color: var(--bl-text-2);
  opacity: 0.7;
}

/* Status badges */
.status-badge {
  display: inline-block;
  padding: 0.2rem 0.6rem;
  border-radius: var(--bl-radius-xl);
  font-size: var(--bl-text-xs);
  font-weight: 600;
  text-transform: uppercase;
}
.status-idle { background: var(--bl-bg-3); color: var(--bl-text-2); }
.status-running { background: #1d4ed8; color: #fff; }
.status-paused { background: #d97706; color: #fff; }
.status-completed { background: var(--bl-success); color: #fff; }
.status-failed { background: var(--bl-danger); color: #fff; }
.status-cancelled { background: #d97706; color: #fff; }

/* Progress */
.hud-progress {
  margin-bottom: 0.75rem;
}
.progress-phase {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
  margin-bottom: 0.375rem;
}
.progress-bar {
  height: 6px;
  background: var(--bl-bg-3);
  border-radius: 3px;
  overflow: hidden;
  margin-bottom: 0.375rem;
}
.progress-fill {
  height: 100%;
  background: var(--bl-accent);
  border-radius: 3px;
  transition: width 0.5s ease;
}
.progress-stats {
  display: flex;
  gap: 0.75rem;
  font-size: 0.6875rem;
  color: var(--bl-text-2);
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
  border-radius: var(--bl-radius-md);
  font-size: var(--bl-text-xs);
  font-weight: 500;
  cursor: pointer;
}
.btn-primary { background: var(--bl-accent); color: #fff; }
.btn-primary:hover { opacity: 0.9; }
.btn-secondary { background: var(--bl-bg-3); color: var(--bl-text); }
.btn-secondary:hover { opacity: 0.85; }
.btn-warning { background: #d97706; color: #fff; }
.btn-warning:hover { opacity: 0.9; }
.btn-danger { background: var(--bl-danger); color: #fff; }
.btn-danger:hover { opacity: 0.9; }

/* Report */
.hud-report {
  border-top: 1px solid var(--bl-border);
  padding-top: 0.5rem;
  margin-bottom: 0.25rem;
}
.report-title {
  font-size: var(--bl-text-xs);
  font-weight: 500;
  color: var(--bl-text);
  margin-bottom: 0.25rem;
}
.report-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  font-size: 0.6875rem;
  color: var(--bl-text-2);
}

/* Error */
.hud-error {
  margin-top: 0.5rem;
  padding: 0.375rem 0.5rem;
  background: rgba(239, 68, 68, 0.1);
  border-radius: var(--bl-radius-sm);
  font-size: var(--bl-text-xs);
  color: var(--bl-danger);
}

.hud-info-box {
  margin-top: 0.75rem;
  padding: 0.5rem;
  background: var(--bl-bg-3);
  border-left: 3px solid var(--bl-accent);
  border-radius: var(--bl-radius-sm);
  font-size: 0.6875rem;
  color: var(--bl-text-2);
  line-height: 1.4;
}

.migration-running-msg {
  font-size: var(--bl-text-xs);
  color: var(--bl-accent);
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

/* Logs */
.hud-logs-tab {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.logs-section-title {
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--bl-text-2);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 0.25rem;
}
.hud-log-viewer {
  max-height: 200px;
  overflow-y: auto;
  background: var(--bl-bg-3);
  border-radius: var(--bl-radius-md);
  padding: 0.375rem 0.5rem;
  font-family: monospace;
  font-size: 0.6875rem;
  line-height: 1.5;
}
.log-line {
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--bl-text-2);
}
.log-error {
  color: var(--bl-danger);
}
.logs-empty {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
  text-align: center;
  padding: 1rem 0;
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
  font-size: var(--bl-text-sm);
  color: var(--bl-text);
}
.schedule-row input[type="checkbox"] {
  accent-color: var(--bl-accent);
}
.schedule-input {
  width: 80px;
  background: var(--bl-bg-3);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-sm);
  color: var(--bl-text);
  font-size: var(--bl-text-sm);
  padding: 0.25rem 0.4rem;
  text-align: right;
}
.schedule-saved {
  font-size: var(--bl-text-xs);
  color: var(--bl-success);
  text-align: center;
}
</style>
