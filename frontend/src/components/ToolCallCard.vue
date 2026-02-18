<template>
  <div class="tool-call-card">
    <div class="tc-header">
      <span class="tc-name">{{ tool.tool_name }}</span>
      <span class="tc-time">{{ new Date(tool.timestamp).toLocaleTimeString() }}</span>
    </div>
    <div v-if="tool.input" class="tc-section">
      <div class="tc-label">Input</div>
      <pre class="tc-content">{{ formatJson(tool.input) }}</pre>
    </div>
    <div v-if="tool.output" class="tc-section">
      <div class="tc-label">Output</div>
      <pre class="tc-content">{{ truncate(tool.output, 1000) }}</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ToolCallDetail } from '@/types'

defineProps<{ tool: ToolCallDetail }>()

function formatJson(input: string): string {
  try {
    return JSON.stringify(JSON.parse(input), null, 2)
  } catch {
    return input
  }
}

function truncate(text: string, maxLen: number): string {
  if (text.length <= maxLen) return text
  return text.slice(0, maxLen) + '\n... (truncated)'
}
</script>

<style scoped>
.tool-call-card {
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  overflow: hidden;
}
.tc-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem 0.75rem;
  background: var(--bl-bg-3);
  border-bottom: 1px solid var(--bl-border);
}
.tc-name { font-weight: 600; color: var(--bl-accent); font-size: var(--bl-text-md); }
.tc-time { font-size: var(--bl-text-xs); color: var(--bl-text-2); }
.tc-section { border-top: 1px solid var(--bl-border); }
.tc-section:first-of-type { border-top: none; }
.tc-label {
  padding: 0.25rem 0.75rem;
  font-size: var(--bl-text-xs);
  font-weight: 600;
  color: var(--bl-text-2);
  text-transform: uppercase;
}
.tc-content {
  padding: 0.25rem 0.75rem 0.5rem;
  font-size: var(--bl-text-sm);
  max-height: 300px;
  overflow-y: auto;
  background: none;
  border: none;
  border-radius: 0;
}
</style>
