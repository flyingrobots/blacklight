<template>
  <div class="insights-view">
    <h1>Insights & Autopsy</h1>

    <div v-if="loading" class="loading-state"><div class="spinner"></div></div>

    <template v-else>
      <div class="insights-grid">
        <!-- Outcome Summary -->
        <section class="insight-card full-width">
          <div class="card-header">
            <h2>Session Outcome Distribution</h2>
            <span class="meta-text" v-if="coverage">{{ coverage.outcome_pct.toFixed(1) }}% labeled</span>
          </div>
          
          <div class="outcome-bar-container">
            <div class="outcome-bar">
              <div 
                v-for="o in outcomes.outcomes" 
                :key="o.outcome"
                :class="['bar-segment', o.outcome.toLowerCase()]"
                :style="{ width: getOutcomePct(o.count) + '%' }"
                :title="`${o.outcome}: ${o.count}`"
              ></div>
            </div>
            <div class="outcome-legend">
              <div v-for="o in outcomes.outcomes" :key="o.outcome" class="legend-item">
                <span :class="['dot', o.outcome.toLowerCase()]"></span>
                <span class="label">{{ o.outcome }}</span>
                <span class="count">{{ o.count }}</span>
              </div>
            </div>
          </div>
        </section>

        <!-- Failure Reasons -->
        <section class="insight-card">
          <div class="card-header">
            <h2>Failure Autopsy</h2>
            <span class="meta-text">Recurring reason codes</span>
          </div>
          <div class="reasons-list">
            <div v-for="r in outcomes.reasons" :key="r.reason_code" class="reason-row">
              <div class="reason-info">
                <span class="reason-code">{{ r.reason_code }}</span>
                <div class="reason-bar-bg">
                  <div class="reason-bar-fill" :style="{ width: getReasonPct(r.count) + '%' }"></div>
                </div>
              </div>
              <span class="reason-count">{{ r.count }}</span>
            </div>
            <div v-if="outcomes.reasons.length === 0" class="empty-text">
              No failure reasons recorded yet.
            </div>
          </div>
        </section>

        <!-- Coverage Stats -->
        <section class="insight-card" v-if="coverage">
          <div class="card-header">
            <h2>Ingestion Coverage</h2>
          </div>
          <div class="stats-list">
            <div class="stat-row">
              <span class="stat-label">Total Sessions</span>
              <span class="stat-value">{{ coverage.total_sessions.toLocaleString() }}</span>
            </div>
            <div class="stat-row">
              <span class="stat-label">Labeled Sessions</span>
              <span class="stat-value">{{ coverage.sessions_with_outcomes.toLocaleString() }}</span>
            </div>
            <div class="stat-row">
              <span class="stat-label">Total Messages</span>
              <span class="stat-value">{{ coverage.total_messages.toLocaleString() }}</span>
            </div>
            <div class="stat-row">
              <span class="stat-label">Messages w/ Content</span>
              <span class="stat-value">{{ coverage.messages_with_content.toLocaleString() }}</span>
            </div>
          </div>
        </section>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/api/client'
import type { OutcomeBreakdown, IndexCoverage } from '@/types'

const loading = ref(true)
const outcomes = ref<OutcomeBreakdown>({ outcomes: [], reasons: [] })
const coverage = ref<IndexCoverage | null>(null)

async function fetchData() {
  loading.value = true
  try {
    const [o, c] = await Promise.all([
      api.analytics.outcomes(),
      api.analytics.coverage()
    ])
    outcomes.value = o
    coverage.value = c
  } catch (e) {
    console.error('Failed to fetch insights:', e)
  } finally {
    loading.value = false
  }
}

function getOutcomeTotal() {
  return outcomes.value.outcomes.reduce((a, b) => a + b.count, 0)
}

function getOutcomePct(count: number) {
  const total = getOutcomeTotal()
  return total > 0 ? (count / total) * 100 : 0
}

function getReasonMax() {
  return Math.max(...outcomes.value.reasons.map(r => r.count), 1)
}

function getReasonPct(count: number) {
  return (count / getReasonMax()) * 100
}

onMounted(fetchData)
</script>

<style scoped>
.insights-view {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.insights-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 1.25rem;
}

.full-width {
  grid-column: span 2;
}

.insight-card {
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  padding: 1.25rem;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 1.25rem;
}

.card-header h2 {
  font-size: var(--bl-text-sm);
  font-weight: 600;
  color: var(--bl-text);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.meta-text {
  font-size: var(--bl-text-2xs);
  color: var(--bl-text-3);
}

/* Outcome Bar */
.outcome-bar-container {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.outcome-bar {
  height: 12px;
  display: flex;
  background: var(--bl-surface-2);
  border-radius: 6px;
  overflow: hidden;
}

.bar-segment {
  height: 100%;
  transition: width 0.3s ease;
}

.bar-segment.success { background: var(--bl-success); }
.bar-segment.partial { background: var(--bl-warning); }
.bar-segment.failed { background: var(--bl-danger); }
.bar-segment.abandoned { background: var(--bl-text-3); }
.bar-segment.unknown { background: var(--bl-border); }

.outcome-legend {
  display: flex;
  gap: 1.5rem;
  flex-wrap: wrap;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.dot.success { background: var(--bl-success); }
.dot.partial { background: var(--bl-warning); }
.dot.failed { background: var(--bl-danger); }
.dot.abandoned { background: var(--bl-text-3); }
.dot.unknown { background: var(--bl-border); }

.legend-item .label {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
  text-transform: capitalize;
}

.legend-item .count {
  font-size: var(--bl-text-xs);
  font-weight: 600;
  color: var(--bl-text);
}

/* Reasons List */
.reasons-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.reason-row {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.reason-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.reason-code {
  font-size: var(--bl-text-xs);
  font-family: var(--bl-font-mono);
  color: var(--bl-text-2);
}

.reason-bar-bg {
  height: 4px;
  background: var(--bl-surface-2);
  border-radius: 2px;
  overflow: hidden;
}

.reason-bar-fill {
  height: 100%;
  background: var(--bl-accent);
  border-radius: 2px;
}

.reason-count {
  font-size: var(--bl-text-xs);
  font-weight: 600;
  color: var(--bl-text);
  min-width: 2rem;
  text-align: right;
}

/* Stats List */
.stats-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.stat-row {
  display: flex;
  justify-content: space-between;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--bl-border-2);
}

.stat-row:last-child { border-bottom: none; }

.stat-label { font-size: var(--bl-text-xs); color: var(--bl-text-2); }
.stat-value { font-size: var(--bl-text-xs); font-weight: 600; font-family: var(--bl-font-mono); }

/* States */
.loading-state {
  display: flex;
  justify-content: center;
  padding: 4rem;
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

.empty-text {
  text-align: center;
  padding: 2rem;
  color: var(--bl-text-3);
  font-size: var(--bl-text-sm);
}
</style>
