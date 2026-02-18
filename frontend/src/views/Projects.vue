<template>
  <div class="projects">
    <h2>Projects</h2>
    <div v-if="loading" class="loading">Loading...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <template v-else>
      <div class="treemap">
        <router-link
          v-for="p in projects"
          :key="p.project_slug"
          :to="{ path: '/sessions', query: { project: p.project_slug } }"
          class="project-card"
          :style="cardStyle(p)"
        >
          <div class="card-header">
            <span class="project-name">{{ p.project_slug }}</span>
            <span class="session-count">{{ p.session_count }} sessions</span>
          </div>
          <div class="card-stats">
            <span>{{ fmtNum(p.message_count) }} msgs</span>
            <span>{{ fmtNum(p.tool_call_count) }} tools</span>
            <span>{{ fmtNum(p.files_touched) }} files</span>
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
            <span
              v-for="t in p.top_tools"
              :key="t.tool_name"
              class="legend-item"
            >
              <span class="legend-dot" :style="{ background: toolColor(t.tool_name) }"></span>
              {{ shortName(t.tool_name) }}
            </span>
          </div>
        </router-link>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api } from '@/api/client'
import type { ProjectDetail, ToolFrequency } from '@/types'

const loading = ref(true)
const error = ref('')
const projects = ref<ProjectDetail[]>([])

const TOOL_COLORS: Record<string, string> = {
  Read: '#58a6ff',
  Bash: '#f85149',
  Edit: '#3fb950',
  Write: '#d29922',
  Grep: '#bc8cff',
  Glob: '#f0883e',
  Search: '#a5d6ff',
  Task: '#8b949e',
}

function toolColor(name: string): string {
  // Match known tool names (may appear as e.g. "Read" or "read")
  for (const [key, color] of Object.entries(TOOL_COLORS)) {
    if (name.toLowerCase().includes(key.toLowerCase())) return color
  }
  return '#484f58'
}

function shortName(name: string): string {
  // Strip common prefixes
  return name.replace(/^mcp__\w+__/, '').replace(/Tool$/, '')
}

const maxMessages = computed(() =>
  Math.max(1, ...projects.value.map(p => p.message_count))
)

function cardStyle(p: ProjectDetail) {
  // Scale card size: min 1fr, larger projects get more columns
  const ratio = p.message_count / maxMessages.value
  const span = ratio > 0.5 ? 2 : 1
  return {
    gridColumn: `span ${span}`,
  }
}

function segmentStyle(t: ToolFrequency, p: ProjectDetail) {
  const total = p.top_tools.reduce((s, x) => s + x.call_count, 0)
  const pct = total > 0 ? (t.call_count / total) * 100 : 0
  return {
    width: `${pct}%`,
    background: toolColor(t.tool_name),
  }
}

function fmtNum(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`
  return String(n)
}

function fmtDate(d: string | null): string {
  if (!d) return '?'
  return d.slice(0, 10)
}

async function fetchProjects() {
  loading.value = true
  error.value = ''
  try {
    projects.value = await api.projects()
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

onMounted(fetchProjects)
</script>

<style scoped>
.projects {
  max-width: 1100px;
}
.projects h2 {
  margin-bottom: 1.5rem;
}
.treemap {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 1rem;
}
.project-card {
  display: block;
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  padding: 1rem;
  text-decoration: none;
  color: var(--bl-text);
  transition: border-color 0.15s, background 0.15s;
}
.project-card:hover {
  border-color: var(--bl-accent);
  background: var(--bl-bg-3);
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 0.5rem;
}
.project-name {
  font-weight: 600;
  font-size: var(--bl-text-base);
  color: var(--bl-accent);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 70%;
}
.session-count {
  font-size: 0.8rem;
  color: var(--bl-text-2);
  white-space: nowrap;
}
.card-stats {
  display: flex;
  gap: 1rem;
  font-size: 0.8rem;
  color: var(--bl-text-2);
  margin-bottom: 0.25rem;
}
.card-dates {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
  margin-bottom: 0.75rem;
}
.tool-bar {
  display: flex;
  height: 6px;
  border-radius: 3px;
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
  font-size: 0.7rem;
  color: var(--bl-text-2);
}
.legend-item {
  display: flex;
  align-items: center;
  gap: 3px;
}
.legend-dot {
  width: 8px;
  height: 8px;
  border-radius: var(--bl-radius-round);
  flex-shrink: 0;
}
.loading {
  color: var(--bl-text-2);
  padding: 2rem 0;
}
.error {
  color: var(--bl-danger);
  padding: 1rem 0;
}
</style>
