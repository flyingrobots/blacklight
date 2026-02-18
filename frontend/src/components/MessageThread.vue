<template>
  <div class="message-thread">
    <div
      v-for="msg in messages"
      :key="msg.id"
      :class="['message', effectiveType(msg)]"
    >
      <div class="message-header">
        <span class="role">{{ roleLabel(msg) }}</span>
        <span v-if="msg.model" class="model">{{ msg.model }}</span>
        <span class="time">{{ new Date(msg.timestamp).toLocaleTimeString() }}</span>
        <span v-if="msg.duration_ms" class="duration">{{ (msg.duration_ms / 1000).toFixed(1) }}s</span>
      </div>
      <div class="message-body">
        <ContentBlock
          v-for="block in msg.content_blocks"
          :key="block.block_index"
          :block="block"
        />
        <div v-if="!msg.content_blocks.length" class="empty-msg">(no content blocks)</div>
      </div>
    </div>
    <div v-if="!messages.length" class="empty">No messages</div>
  </div>
</template>

<script setup lang="ts">
import type { MessageDetail } from '@/types'
import ContentBlock from './ContentBlock.vue'

defineProps<{ messages: MessageDetail[] }>()

/** Is this "user" message actually just tool results (not human input)? */
function isToolResultMessage(msg: MessageDetail): boolean {
  if (msg.type !== 'user') return false
  if (msg.content_blocks.length === 0) return false
  return msg.content_blocks.every(b => b.block_type === 'tool_result')
}

/** CSS class — tool-result user messages get their own style */
function effectiveType(msg: MessageDetail): string {
  if (isToolResultMessage(msg)) return 'tool-result'
  return msg.type
}

/** Display label for the message role */
function roleLabel(msg: MessageDetail): string {
  if (isToolResultMessage(msg)) return 'Tool Result'
  if (msg.type === 'user') return 'You'
  if (msg.type === 'assistant') return 'Assistant'
  return msg.type.charAt(0).toUpperCase() + msg.type.slice(1)
}
</script>

<style scoped>
.message-thread { display: flex; flex-direction: column; gap: 1rem; }
.message {
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  overflow: hidden;
}
.message.user { border-left: 3px solid var(--bl-accent); }
.message.assistant { border-left: 3px solid var(--bl-success); }
.message.tool-result { border-left: 3px solid var(--bl-border); opacity: 0.85; }
.message.system { border-left: 3px solid var(--bl-warning); }
.message.summary { border-left: 3px solid var(--bl-purple); }
.message-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 1rem;
  background: var(--bl-bg-2);
  border-bottom: 1px solid var(--bl-border);
  font-size: var(--bl-text-xs);
}
.tool-result .message-header { background: var(--bl-bg); }
.role { font-weight: 600; }
.model { color: var(--bl-text-2); }
.time { color: var(--bl-text-2); }
.duration { color: var(--bl-text-2); }
.message-body { padding: 0.75rem 1rem; }
.empty-msg { color: var(--bl-text-2); font-size: var(--bl-text-sm); font-style: italic; }
.empty { color: var(--bl-text-2); padding: 2rem; text-align: center; }
</style>
