<template>
  <div class="operations-view">
    <h1>Operations</h1>

    <!-- Status cards -->
    <div class="status-grid">
      <!-- Indexer -->
      <div class="status-card">
        <div class="sc-header">
          <h2>Indexer</h2>
          <span :class="['status-badge', `badge-${indexerStatus.status}`]">{{ indexerStatus.status }}</span>
        </div>

        <div v-if="indexerIsActive || indexerStatus.status === 'paused'" class="progress-section">
          <div class="progress-label">{{ indexerStatus.progress.phase || 'Processing...' }}</div>
          <div class="progress-bar"><div class="progress-fill" :style="{ width: indexerPct + '%' }"></div></div>
          <div class="progress-detail">
            {{ indexerStatus.progress.files_done }}/{{ indexerStatus.progress.files_total }} files
            &middot; {{ indexerStatus.progress.messages_processed.toLocaleString() }} msgs
          </div>
        </div>

        <div v-if="indexerStatus.latest_report" class="report-section">
          <div class="report-title">Last run: {{ indexerStatus.latest_report.elapsed_secs.toFixed(1) }}s</div>
          <div class="report-stats">
            {{ indexerStatus.latest_report.sessions_parsed }} sessions,
            {{ indexerStatus.latest_report.messages_processed.toLocaleString() }} messages,
            {{ indexerStatus.latest_report.blobs_inserted.toLocaleString() }} blobs
          </div>
        </div>

        <div v-if="indexerStatus.outdated_count > 0" class="outdated-note">
          {{ indexerStatus.outdated_count }} sessions need re-indexing
        </div>

        <div v-if="indexerStatus.error_message" class="error-msg">{{ indexerStatus.error_message }}</div>

        <div class="sc-controls">
          <template v-if="indexerIsActive">
            <button class="btn btn-warn" @click="doPause">Pause</button>
            <button class="btn btn-danger" @click="doStop">Stop</button>
          </template>
          <template v-else-if="indexerStatus.status === 'paused'">
            <button class="btn btn-primary" @click="doResume">Resume</button>
            <button class="btn btn-danger" @click="doStop">Stop</button>
          </template>
          <template v-else>
            <button class="btn btn-primary" @click="doStart(false)">Index</button>
            <button class="btn btn-secondary" @click="doStart(true)">Full Re-index</button>
          </template>
        </div>
      </div>

      <!-- Enrichment -->
      <div class="status-card">
        <div class="sc-header">
          <h2>Enrichment</h2>
          <span :class="['status-badge', `badge-${enricherStatus.status}`]">{{ enricherStatus.status }}</span>
        </div>

        <div v-if="enricherIsActive" class="progress-section">
          <div class="progress-label">Enriching sessions...</div>
          <div class="progress-bar"><div class="progress-fill" :style="{ width: enricherPct + '%' }"></div></div>
          <div class="progress-detail">
            {{ enricherStatus.sessions_done }}/{{ enricherStatus.sessions_total }} done
            <template v-if="enricherStatus.sessions_failed"> &middot; {{ enricherStatus.sessions_failed }} failed</template>
          </div>
        </div>

        <div v-if="enricherStatus.latest_report" class="report-section">
          <div class="report-stats">
            {{ enricherStatus.latest_report.enriched }} enriched,
            {{ enricherStatus.latest_report.skipped }} skipped,
            {{ enricherStatus.latest_report.failed }} failed
          </div>
        </div>

        <div v-if="enricherStatus.outdated_count > 0" class="outdated-note">
          {{ enricherStatus.outdated_count }} sessions need enrichment
        </div>

        <div v-if="enricherStatus.error_message" class="error-msg">{{ enricherStatus.error_message }}</div>

        <div class="sc-controls">
          <template v-if="enricherIsActive">
            <button class="btn btn-danger" @click="doEnrichStop">Stop</button>
          </template>
          <template v-else>
            <button class="btn btn-primary" @click="doEnrichStart(false)">Enrich</button>
            <button class="btn btn-secondary" @click="doEnrichStart(true)">Force All</button>
          </template>
        </div>
      </div>
    </div>

    <!-- Schedule -->
    <section class="section-card">
      <div class="sc-header">
        <h2>Schedule</h2>
        <span v-if="scheduleConfig?.enabled" class="status-badge badge-running">Active</span>
        <span v-else class="status-badge badge-idle">Off</span>
      </div>
      <div class="schedule-grid">
        <label class="schedule-field">
          <span>Enabled</span>
          <input type="checkbox" v-model="scheduleForm.enabled" />
        </label>
        <label class="schedule-field">
          <span>Interval (min)</span>
          <input type="number" v-model.number="scheduleForm.interval_minutes" min="1" class="num-input" />
        </label>
        <label class="schedule-field">
          <span>Run enrichment</span>
          <input type="checkbox" v-model="scheduleForm.run_enrichment" />
        </label>
        <label class="schedule-field">
          <span>Concurrency</span>
          <input type="number" v-model.number="scheduleForm.enrichment_concurrency" min="1" max="20" class="num-input" />
        </label>
      </div>
      <div class="schedule-timing" v-if="scheduleConfig">
        <span>Last run: {{ scheduleConfig.last_run_at ? fmtTime(scheduleConfig.last_run_at) : 'Never' }}</span>
        <span v-if="scheduleConfig.enabled">Next: {{ scheduleConfig.next_run_at ? fmtTime(scheduleConfig.next_run_at) : '...' }}</span>
      </div>
      <div class="sc-controls">
        <button class="btn btn-primary" @click="saveSchedule">Save</button>
        <span v-if="scheduleSaved" class="saved-msg">Saved</span>
      </div>
    </section>

    <!-- Ingestion Trust -->
    <section class="section-card">
      <div class="sc-header">
        <h2>Ingestion Trust</h2>
        <span class="review-count" v-if="indexRuns.length > 0">{{ indexRuns.length }} runs recorded</span>
      </div>

      <div v-if="runsLoading" class="loading-inline">Loading runs...</div>
      <div v-else-if="indexRuns.length === 0" class="empty-review">No indexing runs recorded yet.</div>
      <div v-else class="runs-table-container">
        <table class="runs-table">
          <thead>
            <tr>
              <th>Started</th>
              <th>Status</th>
              <th>Type</th>
              <th>Files</th>
              <th>Msgs</th>
              <th>Blobs</th>
              <th>Errors</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="run in indexRuns" :key="run.id">
              <td class="run-date">{{ fmtDate(run.started_at) }}</td>
              <td><span :class="['status-badge', `badge-${run.status}`]">{{ run.status }}</span></td>
              <td class="run-type">{{ run.is_full ? 'Full' : 'Incr' }}</td>
              <td class="run-stats">{{ run.files_processed }}/{{ run.files_scanned }}</td>
              <td class="run-stats">{{ run.messages_processed.toLocaleString() }}</td>
              <td class="run-stats">{{ run.blobs_inserted.toLocaleString() }}</td>
              <td :class="['run-stats', { 'has-errors': run.errors > 0 }]" :title="run.error_message || ''">
                {{ run.errors }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- Review queue -->
    <section class="section-card">
      <div class="sc-header">
        <h2>Review Queue</h2>
        <span class="review-count" v-if="reviewTotal > 0">{{ reviewTotal }} pending</span>
        <button v-if="reviewItems.length > 0" class="btn btn-primary btn-sm" @click="approveAll">Approve All</button>
      </div>

      <div v-if="reviewLoading" class="loading-inline">Loading...</div>
      <div v-else-if="reviewItems.length === 0" class="empty-review">No enrichments pending review.</div>
      <div v-else class="review-list">
        <div v-for="item in reviewItems" :key="item.session_id" class="review-item">
          <div class="ri-main">
            <router-link :to="`/sessions/${item.session_id}`" class="ri-title">{{ item.title }}</router-link>
            <div class="ri-meta">
              <span class="ri-project">{{ item.project_slug }}</span>
              <span class="ri-date">{{ fmtDate(item.enriched_at) }}</span>
            </div>
            <p class="ri-summary">{{ item.summary }}</p>
            <div class="ri-tags">
              <span
                v-for="tag in item.tags"
                :key="tag.tag"
                :class="['ri-tag', { 'low-conf': tag.confidence < 0.8 }]"
              >
                {{ tag.tag }}
                <span class="tag-pct">{{ (tag.confidence * 100).toFixed(0) }}%</span>
              </span>
            </div>
          </div>
          <div class="ri-actions">
            <button class="btn btn-approve" @click="approve(item.session_id)">Approve</button>
            <button class="btn btn-reject" @click="reject(item.session_id)">Reject</button>
          </div>
        </div>
      </div>
    </section>

    <!-- Logs -->
    <section class="section-card">
      <div class="sc-header clickable" @click="logsExpanded = !logsExpanded">
        <h2>Logs</h2>
        <span class="expand-icon">{{ logsExpanded ? '&#9660;' : '&#9654;' }}</span>
      </div>
      <div v-if="logsExpanded" class="logs-container">
        <div v-if="indexerLogs.length" class="logs-section">
          <div class="logs-title">Indexer</div>
          <div class="log-viewer">
            <div v-for="(line, i) in indexerLogs" :key="'i'+i" :class="['log-line', line.includes('Failed') ? 'log-err' : '']">{{ line }}</div>
          </div>
        </div>
        <div v-if="enricherLogs.length" class="logs-section">
          <div class="logs-title">Enrichment</div>
          <div class="log-viewer">
            <div v-for="(line, i) in enricherLogs" :key="'e'+i" :class="['log-line', line.includes('Failed') ? 'log-err' : '']">{{ line }}</div>
          </div>
        </div>
        <div v-if="!indexerLogs.length && !enricherLogs.length" class="empty-review">
          No log output yet.
        </div>
      </div>
    </section>

    <!-- Migration (only if pending) -->
    <section v-if="migrationStatus.pending_count > 0 || migrationIsActive" class="section-card">
      <div class="sc-header">
        <h2>Migration</h2>
        <span :class="['status-badge', `badge-${migrationStatus.status}`]">{{ migrationStatus.status }}</span>
      </div>
      <div v-if="migrationIsActive" class="progress-section">
        <div class="progress-bar"><div class="progress-fill" :style="{ width: migrationPct + '%' }"></div></div>
        <div class="progress-detail">
          {{ migrationStatus.progress.fingerprints_updated }}/{{ migrationStatus.progress.total_sessions }} sessions
        </div>
      </div>
      <div v-if="migrationStatus.pending_count > 0 && !migrationIsActive" class="outdated-note">
        {{ migrationStatus.pending_count }} sessions need migration
      </div>
      <div v-if="migrationStatus.error_message" class="error-msg">{{ migrationStatus.error_message }}</div>
      <div class="sc-controls">
        <button v-if="!migrationIsActive" class="btn btn-primary" @click="doMigrationStart">Start Migration</button>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { api } from '@/api/client'
import type { IndexerStatusResponse, EnricherStatusResponse, ScheduleConfig, ReviewItem, MigrationStatusResponse, IndexRun } from '@/types'

const indexerStatus = ref<IndexerStatusResponse>({
  status: 'idle',
  progress: { phase: '', files_total: 0, files_done: 0, messages_processed: 0, blobs_inserted: 0 },
  latest_report: null, error_message: null, required_version: 0, outdated_count: 0,
})
const indexRuns = ref<IndexRun[]>([])
const runsLoading = ref(true)
const enricherStatus = ref<EnricherStatusResponse>({
  status: 'idle', sessions_total: 0, sessions_done: 0, sessions_failed: 0,
  latest_report: null, error_message: null, required_version: 0, outdated_count: 0,
})
const migrationStatus = ref<MigrationStatusResponse>({
  status: 'idle', progress: { total_sessions: 0, backed_up: 0, fingerprints_updated: 0 },
  error_message: null, pending_count: 0,
})
const scheduleConfig = ref<ScheduleConfig | null>(null)
const scheduleForm = ref({ enabled: true, interval_minutes: 60, run_enrichment: true, enrichment_concurrency: 5 })
const scheduleSaved = ref(false)
const reviewItems = ref<ReviewItem[]>([])
const reviewTotal = ref(0)
const reviewLoading = ref(true)
const indexerLogs = ref<string[]>([])
const enricherLogs = ref<string[]>([])
const logsExpanded = ref(false)

let pollTimer: ReturnType<typeof setInterval>

const indexerIsActive = computed(() => indexerStatus.value.status === 'running')
const enricherIsActive = computed(() => enricherStatus.value.status === 'running')
const migrationIsActive = computed(() => migrationStatus.value.status === 'running')

const indexerPct = computed(() => {
  const p = indexerStatus.value.progress
  return p.files_total === 0 ? 0 : Math.min(100, (p.files_done / p.files_total) * 100)
})
const enricherPct = computed(() => {
  const s = enricherStatus.value
  return s.sessions_total === 0 ? 0 : Math.min(100, ((s.sessions_done + s.sessions_failed) / s.sessions_total) * 100)
})
const migrationPct = computed(() => {
  const s = migrationStatus.value.progress
  return s.total_sessions === 0 ? 0 : Math.min(100, (s.fingerprints_updated / s.total_sessions) * 100)
})

function fmtTime(iso: string): string {
  return new Date(iso).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
}

function fmtDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}

// Indexer actions
async function doStart(full: boolean) {
  try {
    await api.indexer.start(full)
    indexerStatus.value.status = 'running'
    indexerStatus.value.progress = { phase: 'Starting...', files_total: 0, files_done: 0, messages_processed: 0, blobs_inserted: 0 }
    indexerStatus.value.latest_report = null
    indexerStatus.value.error_message = null
  } catch (e: any) { indexerStatus.value.error_message = e.message }
}
async function doStop() { try { await api.indexer.stop() } catch { /* */ } }
async function doPause() { try { await api.indexer.pause(); indexerStatus.value.status = 'paused' } catch { /* */ } }
async function doResume() { try { await api.indexer.resume(); indexerStatus.value.status = 'running' } catch { /* */ } }

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
  } catch (e: any) { enricherStatus.value.error_message = e.message }
}
async function doEnrichStop() { try { await api.enrichment.stop() } catch { /* */ } }

// Migration
async function doMigrationStart() {
  try {
    await api.migration.start()
    migrationStatus.value.status = 'running'
    migrationStatus.value.progress = { total_sessions: 0, backed_up: 0, fingerprints_updated: 0 }
    migrationStatus.value.error_message = null
  } catch (e: any) { migrationStatus.value.error_message = e.message }
}

// Index Runs
async function loadRuns() {
  try {
    const result = await api.indexer.runs({ limit: 10 })
    indexRuns.value = result.items
  } catch { /* */ } finally { runsLoading.value = false }
}

// Schedule
async function loadSchedule() {
  try {
    const config = await api.schedule.get()
    scheduleConfig.value = config
    scheduleForm.value = {
      enabled: config.enabled, interval_minutes: config.interval_minutes,
      run_enrichment: config.run_enrichment, enrichment_concurrency: config.enrichment_concurrency,
    }
  } catch { /* */ }
}

async function saveSchedule() {
  try {
    await api.schedule.update(scheduleForm.value)
    scheduleSaved.value = true
    setTimeout(() => { scheduleSaved.value = false }, 2000)
    loadSchedule()
  } catch { /* */ }
}

// Review
async function loadReview() {
  reviewLoading.value = true
  try {
    const result = await api.review.list({ limit: 50 })
    reviewItems.value = result.items
    reviewTotal.value = result.total
  } catch { /* */ } finally { reviewLoading.value = false }
}

async function approve(sessionId: string) {
  try {
    await api.review.approve(sessionId)
    reviewItems.value = reviewItems.value.filter(i => i.session_id !== sessionId)
    reviewTotal.value = Math.max(0, reviewTotal.value - 1)
  } catch { /* */ }
}

async function reject(sessionId: string) {
  try {
    await api.review.reject(sessionId)
    reviewItems.value = reviewItems.value.filter(i => i.session_id !== sessionId)
    reviewTotal.value = Math.max(0, reviewTotal.value - 1)
  } catch { /* */ }
}

async function approveAll() {
  try {
    await api.review.approveAll()
    reviewItems.value = []
    reviewTotal.value = 0
  } catch { /* */ }
}

// Polling
async function pollStatus() {
  try { indexerStatus.value = await api.indexer.status() } catch { /* */ }
  try { enricherStatus.value = await api.enrichment.status() } catch { /* */ }
  try { migrationStatus.value = await api.migration.status() } catch { /* */ }
  if (indexerIsActive.value) {
    loadRuns()
  }
}

watch(logsExpanded, async (open) => {
  if (open) {
    try { indexerLogs.value = await api.indexer.logs() } catch { /* */ }
    try { enricherLogs.value = await api.enrichment.logs() } catch { /* */ }
  }
})

onMounted(async () => {
  await Promise.all([pollStatus(), loadSchedule(), loadReview(), loadRuns()])
  pollTimer = setInterval(pollStatus, 2000)
})

onUnmounted(() => clearInterval(pollTimer))
</script>

<style scoped>
.operations-view {
  max-width: 900px;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.operations-view h1 {
  font-size: var(--bl-text-xl);
  margin-bottom: 0.5rem;
}

.status-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.75rem;
}

@media (max-width: 768px) {
  .status-grid { grid-template-columns: 1fr; }
}

.status-card, .section-card {
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  padding: 1rem 1.25rem;
}

.sc-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}

.sc-header.clickable { cursor: pointer; }

.sc-header h2 {
  font-size: var(--bl-text-sm);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--bl-text-2);
}

.expand-icon {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-3);
  margin-left: auto;
}

.status-badge {
  padding: 2px 8px;
  border-radius: var(--bl-radius-pill);
  font-size: var(--bl-text-2xs);
  font-weight: 600;
  text-transform: uppercase;
}

.badge-idle { background: var(--bl-surface-3); color: var(--bl-text-2); }
.badge-running { background: rgba(63, 185, 80, 0.15); color: var(--bl-success); }
.badge-paused { background: rgba(210, 153, 34, 0.15); color: var(--bl-warning); }
.badge-completed { background: rgba(63, 185, 80, 0.15); color: var(--bl-success); }
.badge-failed { background: rgba(248, 81, 73, 0.15); color: var(--bl-danger); }
.badge-cancelled { background: rgba(210, 153, 34, 0.15); color: var(--bl-warning); }

/* Progress */
.progress-section { margin-bottom: 0.75rem; }
.progress-label { font-size: var(--bl-text-xs); color: var(--bl-text-2); margin-bottom: 0.375rem; }
.progress-bar { height: 4px; background: var(--bl-surface-3); border-radius: 2px; overflow: hidden; margin-bottom: 0.375rem; }
.progress-fill { height: 100%; background: var(--bl-accent); border-radius: 2px; transition: width 0.5s ease; }
.progress-detail { font-size: var(--bl-text-xs); color: var(--bl-text-3); }

.report-section { margin-bottom: 0.75rem; }
.report-title { font-size: var(--bl-text-xs); font-weight: 500; color: var(--bl-text-2); margin-bottom: 2px; }
.report-stats { font-size: var(--bl-text-xs); color: var(--bl-text-3); }

.outdated-note {
  font-size: var(--bl-text-xs);
  color: var(--bl-accent);
  margin-bottom: 0.75rem;
}

.error-msg {
  padding: 0.375rem 0.5rem;
  background: rgba(248, 81, 73, 0.1);
  border-radius: var(--bl-radius-sm);
  font-size: var(--bl-text-xs);
  color: var(--bl-danger);
  margin-bottom: 0.75rem;
}

/* Controls */
.sc-controls {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.btn {
  padding: 0.375rem 0.75rem;
  border: none;
  border-radius: var(--bl-radius-md);
  font-size: var(--bl-text-xs);
  font-weight: 500;
  cursor: pointer;
  transition: opacity 0.15s;
}

.btn:hover { opacity: 0.85; }
.btn-sm { padding: 0.25rem 0.5rem; margin-left: auto; }
.btn-primary { background: var(--bl-accent); color: var(--bl-c1); }
.btn-secondary { background: var(--bl-surface-3); color: var(--bl-text); }
.btn-warn { background: var(--bl-warning); color: #fff; }
.btn-danger { background: var(--bl-danger); color: #fff; }
.btn-approve { background: rgba(63, 185, 80, 0.15); color: var(--bl-success); }
.btn-reject { background: rgba(248, 81, 73, 0.1); color: var(--bl-danger); }

.saved-msg { font-size: var(--bl-text-xs); color: var(--bl-success); }

/* Schedule */
.schedule-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

.schedule-field {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: var(--bl-text-sm);
  color: var(--bl-text);
}

.schedule-field input[type="checkbox"] { accent-color: var(--bl-accent); }

.num-input {
  width: 72px;
  background: var(--bl-surface-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-sm);
  padding: 0.25rem 0.4rem;
  text-align: right;
  font-size: var(--bl-text-sm);
  color: var(--bl-text);
}

.schedule-timing {
  display: flex;
  gap: 1.5rem;
  font-size: var(--bl-text-xs);
  color: var(--bl-text-3);
  margin-bottom: 0.75rem;
  font-family: var(--bl-font-mono);
}

/* Review */
.review-count {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
}

.review-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.review-item {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
  padding: 0.75rem;
  background: var(--bl-surface-2);
  border-radius: var(--bl-radius-md);
}

.ri-title {
  font-weight: 600;
  font-size: var(--bl-text-sm);
  color: var(--bl-accent);
  display: block;
  margin-bottom: 0.25rem;
}

.ri-meta {
  display: flex;
  gap: 0.75rem;
  font-size: var(--bl-text-xs);
  color: var(--bl-text-3);
  margin-bottom: 0.375rem;
}

.ri-summary {
  font-size: var(--bl-text-sm);
  color: var(--bl-text-2);
  line-height: 1.4;
  margin-bottom: 0.375rem;
}

.ri-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
}

.ri-tag {
  font-size: var(--bl-text-2xs);
  padding: 1px 6px;
  border-radius: var(--bl-radius-pill);
  background: var(--bl-surface-3);
  color: var(--bl-text-2);
}

.ri-tag.low-conf {
  border: 1px solid var(--bl-warning);
  color: var(--bl-warning);
}

.tag-pct {
  font-size: 0.6rem;
  opacity: 0.6;
  margin-left: 2px;
}

.ri-actions {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  flex-shrink: 0;
}

.empty-review, .loading-inline {
  text-align: center;
  padding: 1.5rem;
  font-size: var(--bl-text-sm);
  color: var(--bl-text-2);
}

/* Logs */
.logs-container {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.logs-title {
  font-size: var(--bl-text-2xs);
  font-weight: 600;
  text-transform: uppercase;
  color: var(--bl-text-3);
  margin-bottom: 0.25rem;
}

.log-viewer {
  max-height: 200px;
  overflow-y: auto;
  background: var(--bl-surface-2);
  border-radius: var(--bl-radius-md);
  padding: 0.5rem;
  font-family: var(--bl-font-mono);
  font-size: var(--bl-text-2xs);
  line-height: 1.5;
}

.log-line { color: var(--bl-text-2); white-space: pre-wrap; word-break: break-all; }
.log-err { color: var(--bl-danger); }

/* Ingestion Trust */
.runs-table-container {
  overflow-x: auto;
  margin: -0.5rem -0.75rem;
}

.runs-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--bl-text-xs);
  color: var(--bl-text);
}

.runs-table th {
  text-align: left;
  padding: 0.75rem;
  border-bottom: 1px solid var(--bl-border);
  color: var(--bl-text-3);
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.runs-table td {
  padding: 0.75rem;
  border-bottom: 1px solid var(--bl-border-2);
  white-space: nowrap;
}

.runs-table tr:last-child td {
  border-bottom: none;
}

.run-date {
  color: var(--bl-text-2);
}

.run-type {
  color: var(--bl-text-3);
}

.run-stats {
  font-family: var(--bl-font-mono);
  text-align: right;
}

.has-errors {
  color: var(--bl-danger);
  font-weight: 600;
}
</style>
