<template>
  <div
    ref="hudRef"
    :class="['hud', expanded ? 'hud-expanded' : 'hud-collapsed', `hud-status-${status.status}`]"
    @click="onHudClick"
  >
    <!-- Collapsed pill -->
    <div v-if="!expanded" class="hud-pill">
      <span v-if="isActive" class="hud-spinner"></span>
      <span v-else-if="status.status === 'paused'" class="hud-icon hud-icon-paused">&#9646;&#9646;</span>
      <span v-else class="hud-icon">&#9707;</span>
      <span v-if="isActive || status.status === 'paused'" class="hud-pill-text">
        {{ pillLabel }}
      </span>
    </div>

    <!-- Expanded card -->
    <div v-if="expanded" class="hud-card" @click.stop>
      <div class="hud-card-header" @click="collapse">
        <span :class="['status-badge', `status-${status.status}`]">{{ status.status }}</span>
        <button class="hud-close" @click.stop="collapse">&times;</button>
      </div>

      <!-- Progress section -->
      <div v-if="isActive || status.status === 'paused'" class="hud-progress">
        <div class="progress-phase">{{ status.progress.phase }}</div>
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: progressPct + '%' }"></div>
        </div>
        <div class="progress-stats">
          <span>{{ status.progress.files_done }}/{{ status.progress.files_total }} files</span>
          <span>{{ status.progress.messages_processed.toLocaleString() }} msgs</span>
          <span>{{ status.progress.blobs_inserted.toLocaleString() }} blobs</span>
        </div>
      </div>

      <!-- Controls -->
      <div class="hud-controls">
        <template v-if="isActive">
          <button class="btn btn-warning" @click="doPause">Pause</button>
          <button class="btn btn-danger" @click="doStop">Stop</button>
        </template>
        <template v-else-if="status.status === 'paused'">
          <button class="btn btn-primary" @click="doResume">Resume</button>
          <button class="btn btn-danger" @click="doStop">Stop</button>
        </template>
        <template v-else>
          <button class="btn btn-primary" @click="doStart(false)">Re-index</button>
          <button class="btn btn-secondary" @click="doStart(true)">Full Re-index</button>
        </template>
      </div>

      <!-- Last run report -->
      <div v-if="status.latest_report" class="hud-report">
        <div class="report-title">Last run: {{ status.latest_report.elapsed_secs.toFixed(1) }}s</div>
        <div class="report-stats">
          <span>{{ status.latest_report.sessions_parsed }} sessions</span>
          <span>{{ status.latest_report.messages_processed.toLocaleString() }} messages</span>
          <span>{{ status.latest_report.blobs_inserted.toLocaleString() }} blobs</span>
          <span>{{ status.latest_report.files_processed }} files</span>
          <span>{{ status.latest_report.tool_calls_inserted.toLocaleString() }} tool calls</span>
        </div>
      </div>

      <!-- Error -->
      <div v-if="status.error_message" class="hud-error">
        {{ status.error_message }}
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
import type { IndexerStatusResponse } from '@/types'

gsap.registerPlugin(Flip)

const hudRef = ref<HTMLElement>()
const expanded = ref(false)

const status = ref<IndexerStatusResponse>({
  status: 'idle',
  progress: { phase: '', files_total: 0, files_done: 0, messages_processed: 0, blobs_inserted: 0 },
  latest_report: null,
  error_message: null,
})

let pollTimer: ReturnType<typeof setInterval> | null = null

const isActive = computed(() => status.value.status === 'running')

const progressPct = computed(() => {
  const p = status.value.progress
  if (p.files_total === 0) return 0
  return Math.min(100, (p.files_done / p.files_total) * 100)
})

const pillLabel = computed(() => {
  const p = status.value.progress
  const pct = progressPct.value
  if (status.value.status === 'paused') return `Paused ${pct.toFixed(0)}%`
  if (p.phase) return `${p.phase} ${pct.toFixed(0)}%`
  return 'Running...'
})

async function pollIndexerStatus() {
  try {
    status.value = await api.indexer.status()
  } catch {
    // Ignore poll errors
  }
}

function startPolling() {
  if (pollTimer) return
  pollTimer = setInterval(pollIndexerStatus, 1000)
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

async function doStart(full: boolean) {
  try {
    await api.indexer.start(full)
    status.value.status = 'running'
    status.value.progress = { phase: 'Starting...', files_total: 0, files_done: 0, messages_processed: 0, blobs_inserted: 0 }
    status.value.latest_report = null
    status.value.error_message = null
    startPolling()
  } catch (e: any) {
    status.value.error_message = e.message
  }
}

async function doStop() {
  try {
    await api.indexer.stop()
  } catch (e: any) {
    status.value.error_message = e.message
  }
}

async function doPause() {
  try {
    await api.indexer.pause()
    status.value.status = 'paused'
  } catch (e: any) {
    status.value.error_message = e.message
  }
}

async function doResume() {
  try {
    await api.indexer.resume()
    status.value.status = 'running'
  } catch (e: any) {
    status.value.error_message = e.message
  }
}

onMounted(async () => {
  try {
    status.value = await api.indexer.status()
  } catch {
    // Indexer endpoint may not be available
  }
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
  width: 340px;
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
</style>
