<template>
  <div :class="['content-block', block.block_type]">
    <div v-if="block.block_type === 'text' && block.content" class="text-content">
      <pre>{{ block.content }}</pre>
    </div>

    <div v-else-if="block.block_type === 'tool_use'" class="tool-use">
      <div class="tool-header">
        <span class="tool-icon">&#9881;</span>
        <span class="tool-name">{{ block.tool_name }}</span>
        <span v-if="block.tool_use_id" class="tool-id">{{ block.tool_use_id.slice(0, 8) }}</span>
      </div>
      <pre v-if="block.tool_input" class="tool-content">{{ formatJson(block.tool_input) }}</pre>
    </div>

    <div v-else-if="block.block_type === 'tool_result'" class="tool-result">
      <div class="tool-header">
        <span class="tool-icon">&#10003;</span>
        <span class="tool-label">Result</span>
      </div>
      <pre v-if="block.content" class="tool-content">{{ truncate(block.content, 2000) }}</pre>
    </div>

    <ThinkingBlock
      v-else-if="block.block_type === 'thinking'"
      :content="block.content ?? ''"
    />
  </div>
</template>

<script setup lang="ts">
import type { ContentBlockDetail } from '@/types'
import ThinkingBlock from './ThinkingBlock.vue'

defineProps<{ block: ContentBlockDetail }>()

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
.content-block { margin-bottom: 0.5rem; }
.content-block:last-child { margin-bottom: 0; }
.text-content pre {
  white-space: pre-wrap;
  word-wrap: break-word;
  background: none;
  border: none;
  padding: 0;
  font-size: var(--bl-text-md);
  line-height: 1.5;
}
.tool-use, .tool-result {
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  overflow: hidden;
}
.tool-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.75rem;
  background: var(--bl-bg-3);
  font-size: var(--bl-text-sm);
}
.tool-icon { opacity: 0.6; }
.tool-name { font-weight: 600; color: var(--bl-accent); }
.tool-label { font-weight: 600; color: var(--bl-success); }
.tool-id { color: var(--bl-text-2); font-size: var(--bl-text-xs); }
.tool-content {
  padding: 0.5rem 0.75rem;
  font-size: var(--bl-text-sm);
  max-height: 400px;
  overflow-y: auto;
  background: none;
  border: none;
  border-radius: 0;
}
</style>
