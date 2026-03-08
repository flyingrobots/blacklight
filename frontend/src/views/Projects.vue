<template>
  <div class="projects-view">
    <h1>Projects</h1>

    <div v-if="loading" class="loading-state"><div class="spinner"></div></div>

    <div v-else-if="projects.length" class="project-grid">
      <router-link
        v-for="p in projects"
        :key="p.project_slug"
        :to="{ path: '/sessions', query: { project: p.project_slug } }"
        class="project-card"
      >
        <div class="card-top">
          <span class="project-name">{{ p.project_slug }}</span>
          <span class="session-count">{{ p.session_count }} sessions</span>
        </div>

        <div class="card-stats">
          <div class="stat">
            <span class="stat-num">{{ fmtNum(p.message_count) }}</span>
            <span class="stat-lbl">messages</span>
          </div>
          <div class="stat">
            <span class="stat-num">{{ fmtNum(p.tool_call_count) }}</span>
            <span class="stat-lbl">tool calls</span>
          </div>
          <div class="stat">
            <span class="stat-num">{{ fmtNum(p.files_touched) }}</span>
            <span class="stat-lbl">files</span>
          </div>
        </div>

        <div class="card-dates">
          {{ fmtDate(p.first_session) }} &ndash; {{ fmtDate(p.last_session) }}
        </div>

        <div class="tool-bar" v-if="p.top_tools.length">
          <div
            v-for="t in p.top_tools"
            :key="t.tool_name"
            class="tool-segment"
            :style="segmentStyle(t, p)"
            :title="`${t.tool_name}: ${t.call_count}`"
          ></div>
        </div>
        <div class="tool-legend" v-if="p.top_tools.length">
          <span v-for="t in p.top_tools" :key="t.tool_name" class="legend-item">
            <span class="legend-dot" :style="{ background: toolColor(t.tool_name) }"></span>
            {{ shortName(t.tool_name) }}
          </span>
        </div>
      </router-link>
    </div>

    <div v-else class="empty-state">
      <p>No projects found. Run the indexer to discover projects.</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/api/client'
import type { ProjectDetail, ToolFrequency } from '@/types'

const loading = ref(true)
const projects = ref<ProjectDetail[]>([])

const TOOL_COLORS: Record<string, string> = {
  Read: '#58a6ff', Bash: '#f85149', Edit: '#3fb950', Write: '#d29922',
  Grep: '#bc8cff', Glob: '#f0883e', Search: '#a5d6ff', Task: '#8b949e',
}

function toolColor(name: string): string {
  for (const [key, color] of Object.entries(TOOL_COLORS)) {
    if (name.toLowerCase().includes(key.toLowerCase())) return color
  }
  return '#505860'
}

function shortName(name: string): string {
  return name.replace(/^mcp__\w+__/, '').replace(/Tool$/, '')
}

function segmentStyle(t: ToolFrequency, p: ProjectDetail) {
  const total = p.top_tools.reduce((s, x) => s + x.call_count, 0)
  const pct = total > 0 ? (t.call_count / total) * 100 : 0
  return { width: `${pct}%`, background: toolColor(t.tool_name) }
}

function fmtNum(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`
  return String(n)
}

function fmtDate(d: string | null): string {
  if (!d) return '?'
  return new Date(d).toLocaleDateString(undefined, { month: 'short', year: '2-digit' })
}

onMounted(async () => {
  try {
    projects.value = await api.projects()
  } catch (e: any) {
    console.error(e)
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.projects-view h1 {
  font-size: var(--bl-text-xl);
  margin-bottom: 1.25rem;
}

.project-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 0.75rem;
}

.project-card {
  display: block;
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  padding: 1rem 1.25rem;
  text-decoration: none;
  color: var(--bl-text);
  transition: border-color 0.15s;
}

.project-card:hover {
  border-color: var(--bl-accent);
  opacity: 1;
}

.card-top {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 0.75rem;
}

.project-name {
  font-weight: 600;
  font-size: var(--bl-text-base);
  color: var(--bl-accent);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 65%;
}

.session-count {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
}

.card-stats {
  display: flex;
  gap: 1.25rem;
  margin-bottom: 0.5rem;
}

.stat {
  display: flex;
  flex-direction: column;
}

.stat-num {
  font-size: var(--bl-text-lg);
  font-weight: 600;
  color: var(--bl-text);
  line-height: 1.2;
}

.stat-lbl {
  font-size: var(--bl-text-2xs);
  color: var(--bl-text-2);
}

.card-dates {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-3);
  margin-bottom: 0.75rem;
}

.tool-bar {
  display: flex;
  height: 4px;
  border-radius: 2px;
  overflow: hidden;
  margin-bottom: 0.5rem;
}

.tool-segment {
  height: 100%;
  min-width: 2px;
}

.tool-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  font-size: var(--bl-text-2xs);
  color: var(--bl-text-2);
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 3px;
}

.legend-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
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
</style>
