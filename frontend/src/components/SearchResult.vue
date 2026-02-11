<template>
  <div class="search-result">
    <div class="sr-header">
      <span class="sr-kind">{{ hit.kind }}</span>
      <router-link
        v-if="hit.session_id"
        :to="`/sessions/${hit.session_id}`"
        class="sr-session"
      >
        {{ hit.session_summary?.slice(0, 60) || hit.session_id.slice(0, 12) }}
      </router-link>
      <span v-if="hit.message_type" class="sr-type">{{ hit.message_type }}</span>
    </div>
    <div class="sr-snippet" v-html="hit.snippet"></div>
  </div>
</template>

<script setup lang="ts">
import type { SearchHit } from '@/types'

defineProps<{ hit: SearchHit }>()
</script>

<style scoped>
.search-result {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 1rem;
}
.sr-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.5rem;
  font-size: 0.8125rem;
}
.sr-kind {
  background: var(--bg-tertiary);
  padding: 0.125rem 0.5rem;
  border-radius: 4px;
  font-size: 0.75rem;
  color: var(--text-secondary);
}
.sr-session { font-size: 0.8125rem; }
.sr-type { color: var(--text-secondary); font-size: 0.75rem; }
.sr-snippet {
  font-size: 0.875rem;
  line-height: 1.5;
  color: var(--text-secondary);
}
.sr-snippet :deep(mark) {
  background: rgba(88, 166, 255, 0.25);
  color: var(--accent);
  padding: 0.05rem 0.2rem;
  border-radius: 2px;
}
</style>
