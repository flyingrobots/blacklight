<template>
  <router-link :to="`/sessions/${session.id}`" class="session-card">
    <div class="card-header">
      <span class="project">{{ session.project_slug }}</span>
      <span class="date">{{ new Date(session.modified_at).toLocaleDateString() }}</span>
    </div>
    <div class="prompt">{{ session.first_prompt?.slice(0, 120) || 'No prompt' }}{{ (session.first_prompt?.length ?? 0) > 120 ? '...' : '' }}</div>
    <div class="card-footer">
      <span v-if="session.message_count" class="meta">{{ session.message_count }} msgs</span>
      <span v-if="session.git_branch" class="meta branch">{{ session.git_branch }}</span>
      <span v-if="session.outcome" class="meta outcome">{{ session.outcome }}</span>
    </div>
  </router-link>
</template>

<script setup lang="ts">
import type { SessionSummary } from '@/types'

defineProps<{ session: SessionSummary }>()
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
.card-footer { display: flex; gap: 0.75rem; flex-wrap: wrap; }
.meta {
  font-size: 0.75rem;
  color: var(--text-secondary);
}
.meta.branch { color: var(--purple); }
.meta.outcome { color: var(--success); }
</style>
