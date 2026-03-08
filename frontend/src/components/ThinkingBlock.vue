<template>
  <div class="thinking-block">
    <button class="thinking-toggle" @click="expanded = !expanded">
      <span class="toggle-icon">{{ expanded ? '\u25BC' : '\u25B6' }}</span>
      Thinking
      <span class="char-count">({{ content.length.toLocaleString() }} chars)</span>
    </button>
    <pre v-if="expanded" class="thinking-content">{{ content }}</pre>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
defineProps<{ content: string }>()
const expanded = ref(false)
</script>

<style scoped>
.thinking-block {
  background: var(--bl-surface-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  overflow: hidden;
}

.thinking-toggle {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  width: 100%;
  padding: 0.3rem 0.625rem;
  background: var(--bl-surface);
  border: none;
  color: var(--bl-purple);
  cursor: pointer;
  font-size: var(--bl-text-xs);
  font-weight: 600;
  text-align: left;
}

.thinking-toggle:hover { background: var(--bl-surface-2); }
.toggle-icon { font-size: 0.5rem; color: var(--bl-text-3); }
.char-count { color: var(--bl-text-3); font-weight: 400; }

.thinking-content {
  padding: 0.625rem;
  font-size: var(--bl-text-xs);
  max-height: 400px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-wrap: break-word;
  color: var(--bl-text-2);
  background: none;
  border: none;
  border-radius: 0;
  border-top: 1px solid var(--bl-border);
}
</style>
