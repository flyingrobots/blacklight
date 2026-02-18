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
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  overflow: hidden;
}
.thinking-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.375rem 0.75rem;
  background: var(--bl-bg-3);
  border: none;
  color: var(--bl-purple);
  cursor: pointer;
  font-size: var(--bl-text-sm);
  font-weight: 600;
  text-align: left;
}
.thinking-toggle:hover { background: var(--bl-border); }
.icon { font-size: var(--bl-text-2xs); }
.length { color: var(--bl-text-2); font-weight: 400; }
.thinking-content {
  padding: 0.75rem;
  font-size: var(--bl-text-sm);
  max-height: 500px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-wrap: break-word;
  color: var(--bl-text-2);
  background: none;
  border: none;
  border-radius: 0;
}
</style>
