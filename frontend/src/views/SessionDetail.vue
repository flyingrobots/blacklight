<template>
  <div class="session-detail">
    <div v-if="loading" class="loading">Loading...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <template v-else-if="session">
      <div class="header">
        <router-link to="/sessions" class="back">&larr; Sessions</router-link>
        <h2>{{ session.first_prompt?.slice(0, 100) || 'Session' }}</h2>
        <div class="meta-row">
          <span class="badge">{{ session.project_slug }}</span>
          <span v-if="session.git_branch" class="badge branch">{{ session.git_branch }}</span>
          <span class="date">{{ new Date(session.created_at).toLocaleString() }}</span>
          <span v-if="session.message_count" class="count">{{ session.message_count }} messages</span>
        </div>
        <p v-if="session.summary" class="summary">{{ session.summary }}</p>

        <div v-if="session.outcome" class="outcome-box">
          <div class="outcome-header">Outcome</div>
          <div v-if="session.outcome.outcome" class="outcome-field">
            <span class="label">Result:</span> {{ session.outcome.outcome }}
          </div>
          <div v-if="session.outcome.underlying_goal" class="outcome-field">
            <span class="label">Goal:</span> {{ session.outcome.underlying_goal }}
          </div>
          <div v-if="session.outcome.brief_summary" class="outcome-field">
            <span class="label">Summary:</span> {{ session.outcome.brief_summary }}
          </div>
        </div>
      </div>

      <div class="tabs">
        <button
          v-for="tab in tabs"
          :key="tab"
          :class="['tab', { active: activeTab === tab }]"
          @click="activeTab = tab"
        >{{ tab }}</button>
      </div>

      <div class="tab-content">
        <MessageThread v-if="activeTab === 'Messages'" :messages="messages" />

        <div v-if="activeTab === 'Tools'" class="tools-list">
          <ToolCallCard v-for="tool in tools" :key="tool.id" :tool="tool" />
          <div v-if="!tools.length" class="empty">No tool calls in this session</div>
        </div>

        <div v-if="activeTab === 'Files'" class="files-list">
          <div v-for="file in files" :key="file.file_path + file.message_id" class="file-row">
            <span class="op-badge" :class="file.operation.toLowerCase()">{{ file.operation }}</span>
            <code>{{ file.file_path }}</code>
          </div>
          <div v-if="!files.length" class="empty">No file references in this session</div>
        </div>

        <div v-if="activeTab === 'Raw'" class="raw-view">
          <div v-if="rawLoading" class="loading">Loading raw JSONL...</div>
          <div v-else-if="rawError" class="error">{{ rawError }}</div>
          <div v-else class="raw-lines">
            <div
              v-for="(line, i) in rawLines"
              :key="i"
              :class="['raw-line', line.type]"
            >
              <div class="raw-line-header" @click="line.expanded = !line.expanded">
                <span class="raw-line-num">{{ i + 1 }}</span>
                <span class="raw-line-type">{{ line.type }}</span>
                <span class="raw-line-role" v-if="line.role">{{ line.role }}</span>
                <span class="raw-line-preview" v-if="!line.expanded">{{ line.preview }}</span>
                <span class="raw-toggle">{{ line.expanded ? '\u25BC' : '\u25B6' }}</span>
              </div>
              <pre v-if="line.expanded" class="raw-json">{{ line.json }}</pre>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '@/api/client'
import type { SessionDetail, MessageDetail, ToolCallDetail, FileReference } from '@/types'
import MessageThread from '@/components/MessageThread.vue'
import ToolCallCard from '@/components/ToolCallCard.vue'

const route = useRoute()
const loading = ref(true)
const error = ref('')
const session = ref<SessionDetail | null>(null)
const messages = ref<MessageDetail[]>([])
const tools = ref<ToolCallDetail[]>([])
const files = ref<FileReference[]>([])
const activeTab = ref('Messages')
const tabs = ['Messages', 'Tools', 'Files', 'Raw']
const rawLoading = ref(false)
const rawError = ref('')

interface RawLine {
  type: string
  role: string
  preview: string
  json: string
  expanded: boolean
}

const rawLines = ref<RawLine[]>([])

async function fetchSession(id: string) {
  loading.value = true
  error.value = ''
  try {
    const [s, m, t, f] = await Promise.all([
      api.sessions.get(id),
      api.sessions.messages(id, { limit: 500 }),
      api.sessions.tools(id),
      api.sessions.files(id),
    ])
    session.value = s
    messages.value = m.items
    tools.value = t
    files.value = f
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

async function fetchRaw(id: string) {
  if (rawLines.value.length) return // already loaded
  rawLoading.value = true
  rawError.value = ''
  try {
    const text = await api.sessions.raw(id)
    rawLines.value = text.split('\n').filter(l => l.trim()).map(line => {
      try {
        const obj = JSON.parse(line)
        const type = obj.type || 'unknown'
        const role = obj.message?.role || obj.role || ''
        let preview = ''
        if (obj.message?.content) {
          const content = obj.message.content
          if (typeof content === 'string') {
            preview = content.slice(0, 120)
          } else if (Array.isArray(content) && content.length > 0) {
            const first = content[0]
            if (first.type === 'text') preview = (first.text || '').slice(0, 120)
            else if (first.type === 'tool_use') preview = `tool_use: ${first.name}`
            else if (first.type === 'tool_result') preview = `tool_result (${content.length} blocks)`
            else preview = `${first.type} (${content.length} blocks)`
          }
        }
        return {
          type,
          role,
          preview,
          json: JSON.stringify(obj, null, 2),
          expanded: false,
        }
      } catch {
        return { type: 'parse_error', role: '', preview: line.slice(0, 100), json: line, expanded: false }
      }
    })
  } catch (e: any) {
    rawError.value = e.message
  } finally {
    rawLoading.value = false
  }
}

onMounted(() => fetchSession(route.params.id as string))
watch(() => route.params.id, (id) => {
  if (id) {
    rawLines.value = []
    fetchSession(id as string)
  }
})
watch(activeTab, (tab) => {
  if (tab === 'Raw') fetchRaw(route.params.id as string)
})
</script>

<style scoped>
.back {
  font-size: 0.875rem;
  color: var(--text-secondary);
  display: inline-block;
  margin-bottom: 0.5rem;
}
.header { margin-bottom: 1.5rem; }
.header h2 { margin-bottom: 0.5rem; }
.meta-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.5rem;
  flex-wrap: wrap;
}
.badge {
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 0.125rem 0.625rem;
  font-size: 0.75rem;
  color: var(--text-secondary);
}
.badge.branch { color: var(--purple); border-color: var(--purple); }
.date, .count { font-size: 0.875rem; color: var(--text-secondary); }
.summary { color: var(--text-secondary); margin-top: 0.5rem; }
.outcome-box {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 1rem;
  margin-top: 1rem;
}
.outcome-header { font-weight: 600; margin-bottom: 0.5rem; color: var(--accent); }
.outcome-field { font-size: 0.875rem; margin-bottom: 0.25rem; }
.outcome-field .label { color: var(--text-secondary); }
.tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border);
  margin-bottom: 1.5rem;
}
.tab {
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 0.5rem 1rem;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 0.875rem;
}
.tab.active { color: var(--accent); border-bottom-color: var(--accent); }
.tab:hover { color: var(--text); }
.tools-list { display: flex; flex-direction: column; gap: 0.75rem; }
.files-list { display: flex; flex-direction: column; gap: 0.375rem; }
.file-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.375rem 0;
  font-size: 0.875rem;
}
.op-badge {
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  padding: 0.125rem 0.5rem;
  border-radius: 4px;
  min-width: 50px;
  text-align: center;
}
.op-badge.read { background: #1a3a2a; color: var(--success); }
.op-badge.write { background: #3a2a1a; color: var(--warning); }
.op-badge.edit { background: #1a2a3a; color: var(--accent); }
.empty { color: var(--text-secondary); padding: 1rem 0; }
.loading, .error { padding: 2rem; text-align: center; }
.error { color: var(--danger); }

/* Raw JSONL view */
.raw-view { max-width: 100%; }
.raw-lines { display: flex; flex-direction: column; gap: 1px; }
.raw-line {
  border: 1px solid var(--border);
  border-radius: 4px;
  overflow: hidden;
}
.raw-line-header {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  padding: 0.375rem 0.75rem;
  background: var(--bg-secondary);
  cursor: pointer;
  font-size: 0.8125rem;
  user-select: none;
}
.raw-line-header:hover { background: var(--bg-tertiary); }
.raw-line-num {
  color: var(--text-secondary);
  min-width: 2rem;
  text-align: right;
  font-size: 0.75rem;
}
.raw-line-type {
  font-weight: 600;
  min-width: 8rem;
  color: var(--accent);
}
.raw-line.human .raw-line-type,
.raw-line.user .raw-line-type { color: var(--accent); }
.raw-line.assistant .raw-line-type { color: var(--success); }
.raw-line.summary .raw-line-type { color: var(--purple); }
.raw-line.system .raw-line-type { color: var(--warning); }
.raw-line-role {
  color: var(--text-secondary);
  font-size: 0.75rem;
}
.raw-line-preview {
  color: var(--text-secondary);
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.75rem;
}
.raw-toggle {
  color: var(--text-secondary);
  font-size: 0.625rem;
  flex-shrink: 0;
}
.raw-json {
  padding: 0.75rem;
  font-size: 0.75rem;
  line-height: 1.4;
  background: var(--bg);
  border: none;
  border-radius: 0;
  max-height: 600px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
