<template>
  <router-link :to="`/sessions/${session.id}`" class="session-card">
    <div class="card-header">
      <span class="project">{{ session.project_slug }}</span>
      <span class="date">{{ new Date(session.modified_at).toLocaleDateString() }}</span>
    </div>
    <div class="prompt" :class="{ faded: !displayText.primary }">{{ displayText.text }}</div>
    <div v-if="session.enrichment_summary" class="enrichment-summary">{{ session.enrichment_summary }}</div>
    <div class="card-footer">
      <span v-if="session.message_count != null && session.message_count > 0" class="meta">{{ session.message_count }} msgs</span>
      <span v-if="session.git_branch" class="meta branch">{{ session.git_branch }}</span>
      <span v-if="session.outcome" class="meta outcome">{{ session.outcome }}</span>
      <span v-for="t in session.tags" :key="t.tag" class="tag-chip">{{ t.tag }}</span>
    </div>
  </router-link>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { SessionSummary } from '@/types'

const props = defineProps<{ session: SessionSummary }>()

const displayText = computed(() => {
  const s = props.session
  const truncate = (t: string) => t.length > 120 ? t.slice(0, 120) + '...' : t
  if (s.enrichment_title) return { text: s.enrichment_title, primary: true }
  if (s.first_prompt) return { text: truncate(s.first_prompt), primary: true }
  if (s.brief_summary) return { text: truncate(s.brief_summary), primary: true }
  if (s.summary) return { text: truncate(s.summary), primary: true }
  return { text: `${s.message_count ?? 0} messages in ${s.project_slug}`, primary: false }
})
</script>

<style scoped>
.session-card {
  display: block;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 1rem;
  transition: border-color 0.15s;
  text-decoration: none;
  color: var(--text);
}
.session-card:hover { border-color: var(--accent); text-decoration: none; }
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}
.project {
  font-size: 0.75rem;
  color: var(--accent);
  background: #1a2a3a;
  padding: 0.125rem 0.5rem;
  border-radius: 4px;
}
.date { font-size: 0.75rem; color: var(--text-secondary); }
.prompt { font-size: 0.875rem; line-height: 1.4; margin-bottom: 0.5rem; }
.prompt.faded { color: var(--text-secondary); font-style: italic; }
.enrichment-summary {
  font-size: 0.8rem;
  color: var(--text-secondary);
  line-height: 1.3;
  margin-bottom: 0.5rem;
}
.card-footer { display: flex; gap: 0.75rem; flex-wrap: wrap; align-items: center; }
.meta {
  font-size: 0.75rem;
  color: var(--text-secondary);
}
.meta.branch { color: var(--purple); }
.meta.outcome { color: var(--success); }
.tag-chip {
  font-size: 0.675rem;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 0.0625rem 0.4375rem;
  color: var(--text-secondary);
}
</style>
