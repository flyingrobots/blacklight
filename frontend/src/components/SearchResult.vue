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
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  padding: 1rem;
}
.sr-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.5rem;
  font-size: var(--bl-text-sm);
}
.sr-kind {
  background: var(--bl-bg-3);
  padding: 0.125rem 0.5rem;
  border-radius: var(--bl-radius-sm);
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
}
.sr-session { font-size: var(--bl-text-sm); }
.sr-type { color: var(--bl-text-2); font-size: var(--bl-text-xs); }
.sr-snippet {
  font-size: var(--bl-text-md);
  line-height: 1.5;
  color: var(--bl-text-2);
}
.sr-snippet :deep(mark) {
  background: rgba(88, 166, 255, 0.25);
  color: var(--bl-accent);
  padding: 0.05rem 0.2rem;
  border-radius: 2px;
}
</style>
