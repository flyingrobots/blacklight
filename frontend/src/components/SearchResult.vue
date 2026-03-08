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
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  padding: 0.75rem 1rem;
  transition: border-color 0.15s;
}

.search-result:hover {
  border-color: var(--bl-border-2);
}

.sr-header {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  margin-bottom: 0.375rem;
  font-size: var(--bl-text-xs);
}

.sr-kind {
  background: var(--bl-surface-3);
  padding: 1px 6px;
  border-radius: var(--bl-radius-sm);
  font-size: var(--bl-text-2xs);
  color: var(--bl-text-2);
}

.sr-session {
  font-size: var(--bl-text-xs);
  color: var(--bl-accent);
}

.sr-type {
  color: var(--bl-text-3);
  font-size: var(--bl-text-2xs);
}

.sr-snippet {
  font-size: var(--bl-text-sm);
  line-height: 1.5;
  color: var(--bl-text-2);
}

.sr-snippet :deep(mark) {
  background: rgba(93, 204, 203, 0.2);
  color: var(--bl-accent);
  padding: 0 2px;
  border-radius: 2px;
}
</style>
