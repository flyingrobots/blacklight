<template>
  <div class="toast" :class="`toast--${toast.level}`">
    <span class="toast-message">{{ toast.message }}</span>
    <button class="toast-dismiss" @click="$emit('dismiss')">&times;</button>
  </div>
</template>

<script setup lang="ts">
import type { Toast } from '@/composables/useNotifications'

defineProps<{ toast: Toast }>()
defineEmits<{ dismiss: [] }>()
</script>

<style scoped>
.toast {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  border-radius: 6px;
  border-left: 4px solid;
  background: var(--bg-secondary, #1a1a2e);
  color: var(--text, #e0e0e0);
  font-size: 0.8125rem;
  line-height: 1.4;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  max-width: 380px;
  word-break: break-word;
}

.toast--info {
  border-left-color: #3b82f6;
}

.toast--warn {
  border-left-color: #f59e0b;
}

.toast--error {
  border-left-color: #ef4444;
}

.toast-message {
  flex: 1;
}

.toast-dismiss {
  flex-shrink: 0;
  background: none;
  border: none;
  color: var(--text-secondary, #888);
  font-size: 1.125rem;
  cursor: pointer;
  padding: 0;
  line-height: 1;
  transition: color 0.15s;
}

.toast-dismiss:hover {
  color: var(--text, #e0e0e0);
}
</style>
