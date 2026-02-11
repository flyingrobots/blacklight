<template>
  <div class="thinking-block">
    <button class="thinking-toggle" @click="expanded = !expanded">
      <span class="icon">{{ expanded ? '&#9660;' : '&#9654;' }}</span>
      Thinking
      <span class="length">({{ content.length.toLocaleString() }} chars)</span>
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
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
}
.thinking-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.375rem 0.75rem;
  background: var(--bg-tertiary);
  border: none;
  color: var(--purple);
  cursor: pointer;
  font-size: 0.8125rem;
  font-weight: 600;
  text-align: left;
}
.thinking-toggle:hover { background: var(--border); }
.icon { font-size: 0.625rem; }
.length { color: var(--text-secondary); font-weight: 400; }
.thinking-content {
  padding: 0.75rem;
  font-size: 0.8125rem;
  max-height: 500px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-wrap: break-word;
  color: var(--text-secondary);
  background: none;
  border: none;
  border-radius: 0;
}
</style>
