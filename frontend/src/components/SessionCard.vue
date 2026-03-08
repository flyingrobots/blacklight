<template>
  <router-link :to="`/sessions/${session.id}`" class="session-card">
    <div class="sc-main">
      <div class="sc-title">{{ displayTitle }}</div>
      <p v-if="session.enrichment_summary" class="sc-summary">{{ session.enrichment_summary }}</p>
    </div>
    <div class="sc-meta">
      <span v-if="session.outcome" :class="['outcome-badge', session.outcome.toLowerCase()]">{{ session.outcome }}</span>
      <span v-if="session.reason_code" class="reason-pill">{{ session.reason_code }}</span>
      <span class="sc-project" :style="{ color: projectColor }">{{ session.project_slug }}</span>
      <span class="sc-date">{{ formatDate(session.modified_at) }}</span>
      <span v-if="session.message_count" class="sc-stat">{{ session.message_count }} msgs</span>
      <span v-if="session.git_branch" class="sc-stat">{{ session.git_branch }}</span>
      <span v-if="session.tags && session.tags.length" class="sc-tag">#{{ session.tags[0].tag }}</span>
    </div>
  </router-link>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { SessionSummary } from '@/types'

const props = defineProps<{ session: SessionSummary }>()

const PROJECT_COLORS = ['#5dcccb', '#bc8cff', '#58a6ff', '#f0883e', '#3fb950', '#d29922', '#f85149']

const projectColor = computed(() => {
  const hash = props.session.project_slug.split('').reduce((a, b) => a + b.charCodeAt(0), 0)
  return PROJECT_COLORS[hash % PROJECT_COLORS.length]
})

const displayTitle = computed(() => {
  if (props.session.enrichment_title) return props.session.enrichment_title
  if (props.session.first_prompt) return props.session.first_prompt.slice(0, 100)
  return 'Untitled session'
})

function formatDate(dateStr: string) {
  const d = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - d.getTime()
  const hours = diff / (1000 * 60 * 60)
  if (hours < 1) return 'just now'
  if (hours < 24) return `${Math.floor(hours)}h ago`
  if (hours < 48) return 'yesterday'
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
}
</script>

<style scoped>
.session-card {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
  padding: 0.75rem 1rem;
  background: var(--bl-surface);
  text-decoration: none;
  color: var(--bl-text);
  transition: background 0.1s;
}

.session-card:hover {
  background: var(--bl-surface-2);
  opacity: 1;
}

.sc-main {
  flex: 1;
  min-width: 0;
}

.sc-title {
  font-size: var(--bl-text-sm);
  font-weight: 500;
  line-height: 1.4;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.sc-summary {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
  margin-top: 0.25rem;
  line-height: 1.4;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
}

.sc-meta {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  flex-shrink: 0;
  font-size: var(--bl-text-xs);
}

.sc-project {
  font-weight: 600;
}

.sc-date {
  color: var(--bl-text-3);
}

.sc-stat {
  color: var(--bl-text-3);
}

.sc-tag {
  color: var(--bl-c3);
  font-size: var(--bl-text-2xs);
}

.outcome-badge {
  font-size: 10px;
  font-weight: 700;
  padding: 1px 6px;
  border-radius: var(--bl-radius-pill);
  text-transform: uppercase;
}

.outcome-badge.success { background: rgba(63, 185, 80, 0.15); color: var(--bl-success); }
.outcome-badge.partial { background: rgba(210, 153, 34, 0.15); color: var(--bl-warning); }
.outcome-badge.failed { background: rgba(248, 81, 73, 0.15); color: var(--bl-danger); }
.outcome-badge.abandoned { background: var(--bl-surface-2); color: var(--bl-text-3); }

.reason-pill {
  font-size: 10px;
  font-family: var(--bl-font-mono);
  color: var(--bl-text-3);
  background: var(--bl-surface-2);
  padding: 1px 4px;
  border-radius: var(--bl-radius-sm);
}
</style>
