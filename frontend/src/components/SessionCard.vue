<template>
  <router-link :to="`/sessions/${session.id}`" class="session-card">
    <div class="card-header">
      <div class="header-left">
        <span class="project">{{ session.project_slug }}</span>
        <span v-if="session.source_name" class="source-badge">{{ session.source_kind || 'source' }}: {{ session.source_name }}</span>
      </div>
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
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  padding: 1rem;
  transition: border-color 0.15s;
  text-decoration: none;
  color: var(--bl-text);
}
.session-card:hover { border-color: var(--bl-accent); text-decoration: none; }
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}
.header-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.source-badge {
  font-family: var(--bl-font-mono);
  font-size: 0.625rem;
  color: var(--bl-text-2);
  opacity: 0.7;
  border: 1px solid var(--bl-border);
  padding: 0.0625rem 0.375rem;
  border-radius: var(--bl-radius-sm);
}
.project {
  font-size: var(--bl-text-xs);
  color: var(--bl-accent);
  background: #1a2a3a;
  padding: 0.125rem 0.5rem;
  border-radius: var(--bl-radius-sm);
}
.date { font-size: var(--bl-text-xs); color: var(--bl-text-2); }
.prompt { font-size: var(--bl-text-md); line-height: 1.4; margin-bottom: 0.5rem; }
.prompt.faded { color: var(--bl-text-2); font-style: italic; }
.enrichment-summary {
  font-size: 0.8rem;
  color: var(--bl-text-2);
  line-height: 1.3;
  margin-bottom: 0.5rem;
}
.card-footer { display: flex; gap: 0.75rem; flex-wrap: wrap; align-items: center; }
.meta {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
}
.meta.branch { color: var(--bl-purple); }
.meta.outcome { color: var(--bl-success); }
.tag-chip {
  font-size: 0.675rem;
  background: var(--bl-bg-3);
  border: 1px solid var(--bl-border);
  border-radius: 10px;
  padding: 0.0625rem 0.4375rem;
  color: var(--bl-text-2);
}
</style>
