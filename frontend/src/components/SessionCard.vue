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
  border-radius: var(--bl-radius-sm);
  padding: 0.75rem 1rem;
  transition: all 0.15s;
  text-decoration: none;
  color: var(--bl-text);
  position: relative;
  font-family: var(--bl-font-mono);
}
.session-card:hover { 
  border-color: var(--bl-accent); 
  background: var(--bl-bg-3);
  text-decoration: none; 
}

/* TUI Selection Indicator */
.session-card.selected::before {
  content: '>';
  position: absolute;
  left: 0.35rem;
  top: 50%;
  transform: translateY(-50%);
  color: var(--bl-accent);
  font-weight: bold;
}
.session-card.selected {
  padding-left: 1.75rem;
  border-color: var(--bl-accent);
  background: var(--bl-bg-3);
  box-shadow: none;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.25rem;
  opacity: 0.8;
}
.header-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.source-badge {
  font-size: 0.625rem;
  color: var(--bl-text-2);
  border: 1px solid var(--bl-border);
  padding: 0 0.375rem;
  border-radius: var(--bl-radius-sm);
  text-transform: uppercase;
}
.project {
  font-size: var(--bl-text-xs);
  color: var(--bl-accent);
  font-weight: bold;
  text-transform: uppercase;
}
.date { font-size: var(--bl-text-xs); color: var(--bl-text-2); }
.prompt { 
  font-size: var(--bl-text-md); 
  line-height: 1.2; 
  margin-bottom: 0.35rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.prompt.faded { color: var(--bl-text-2); opacity: 0.6; }
.enrichment-summary {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
  line-height: 1.2;
  margin-bottom: 0.35rem;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.card-footer { display: flex; gap: 0.75rem; flex-wrap: wrap; align-items: center; opacity: 0.7; }
.meta {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
}
.meta.branch { color: var(--bl-purple); }
.meta.outcome { color: var(--bl-success); border: 1px solid var(--bl-success); padding: 0 4px; border-radius: 2px; }
.tag-chip {
  font-size: 0.625rem;
  background: var(--bl-bg-3);
  border: 1px solid var(--bl-border);
  border-radius: 2px;
  padding: 0 0.4375rem;
  color: var(--bl-text-2);
}
</style>
