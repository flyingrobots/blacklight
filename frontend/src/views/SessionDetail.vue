<template>
  <div class="session-detail">
    <div v-if="loading" class="loading-state"><div class="spinner"></div></div>
    <div v-else-if="error" class="error-state">{{ error }}</div>

    <template v-else-if="session">
      <!-- Header -->
      <header class="detail-header">
        <router-link to="/sessions" class="back-link">
          <svg viewBox="0 0 16 16" fill="currentColor" width="12" height="12"><path d="M7.78 12.53a.75.75 0 0 1-1.06 0L2.47 8.28a.75.75 0 0 1 0-1.06l4.25-4.25a.751.751 0 0 1 1.042.018.751.751 0 0 1 .018 1.042L4.81 7h7.44a.75.75 0 0 1 0 1.5H4.81l2.97 2.97a.75.75 0 0 1 0 1.06z"/></svg>
          Sessions
        </router-link>

        <h1>{{ session.enrichment_title || session.first_prompt?.slice(0, 120) || 'Untitled Session' }}</h1>

        <div class="header-meta">
          <span class="meta-pill project">{{ session.project_slug }}</span>
          <span v-if="session.git_branch" class="meta-pill branch">{{ session.git_branch }}</span>
          <span class="meta-text">{{ new Date(session.created_at).toLocaleString() }}</span>
          <span v-if="session.message_count" class="meta-text">{{ session.message_count }} messages</span>
          <span v-if="session.source_kind" class="meta-text">{{ session.source_kind }}</span>
        </div>

        <div v-if="session.tags && session.tags.length" class="tag-row">
          <span v-for="t in session.tags" :key="t.tag" class="tag-pill">#{{ t.tag }}</span>
        </div>

        <p v-if="session.enrichment_summary" class="summary-text">{{ session.enrichment_summary }}</p>

        <div class="outcome-section">
          <div v-if="session.outcome && !isEditingOutcome" class="outcome-row">
            <span :class="['outcome-badge', session.outcome.outcome?.toLowerCase()]" v-if="session.outcome.outcome">{{ session.outcome.outcome }}</span>
            <span v-if="session.outcome.reason_code" class="reason-pill">{{ session.outcome.reason_code }}</span>
            <span v-if="session.outcome.underlying_goal" class="outcome-goal">{{ session.outcome.underlying_goal }}</span>
            <button class="btn-text" @click="startEditOutcome">Edit</button>
          </div>
          <div v-else-if="!isEditingOutcome" class="outcome-empty">
            <button class="btn-text" @click="startEditOutcome">+ Add Outcome</button>
          </div>
          
          <div v-else class="outcome-editor">
            <select v-model="editOutcome.outcome" class="select-sm">
              <option v-for="o in OUTCOMES" :key="o" :value="o">{{ o }}</option>
            </select>
            <select v-if="editOutcome.outcome !== 'success'" v-model="editOutcome.reason_code" class="select-sm">
              <option :value="null">-- Select Reason --</option>
              <option v-for="r in REASONS" :key="r" :value="r">{{ r }}</option>
            </select>
            <button class="btn btn-primary btn-xs" @click="saveOutcome" :disabled="saving">
              {{ saving ? 'Saving...' : 'Save' }}
            </button>
            <button class="btn-text" @click="isEditingOutcome = false">Cancel</button>
          </div>
        </div>
      </header>

      <!-- Tabs -->
      <div class="detail-tabs">
        <button
          v-for="tab in tabs"
          :key="tab"
          :class="['tab-btn', { active: activeTab === tab }]"
          @click="activeTab = tab"
        >{{ tab }}</button>
      </div>

      <!-- Conversation tab -->
      <div v-if="activeTab === 'Conversation'" class="tab-pane">
        <MessageThread :messages="messages" />
      </div>

      <!-- Files tab -->
      <div v-if="activeTab === 'Files'" class="tab-pane">
        <div v-if="files.length" class="files-list">
          <div v-for="file in files" :key="file.file_path + file.message_id" class="file-row">
            <span :class="['op-badge', file.operation.toLowerCase()]">{{ file.operation }}</span>
            <code class="file-path">{{ file.file_path }}</code>
          </div>
        </div>
        <div v-else class="empty-tab">No file references in this session.</div>
      </div>

      <!-- Raw tab -->
      <div v-if="activeTab === 'Raw'" class="tab-pane">
        <div v-if="rawLoading" class="loading-inline">Loading raw data...</div>
        <div v-else class="raw-feed">
          <div v-for="(line, i) in rawLines" :key="i" class="raw-entry">
            <div class="raw-header" @click="line.expanded = !line.expanded">
              <span class="raw-type">{{ line.type }}</span>
              <span v-if="line.role" class="raw-role">{{ line.role }}</span>
              <span class="raw-preview">{{ line.preview }}</span>
              <span class="raw-toggle">{{ line.expanded ? '\u25BC' : '\u25B6' }}</span>
            </div>
            <pre v-if="line.expanded" class="raw-code">{{ line.json }}</pre>
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
import type { SessionDetail, MessageDetail, FileReference } from '@/types'
import MessageThread from '@/components/MessageThread.vue'

const route = useRoute()
const loading = ref(true)
const error = ref('')
const session = ref<SessionDetail | null>(null)
const messages = ref<MessageDetail[]>([])
const files = ref<FileReference[]>([])
const activeTab = ref('Conversation')

const OUTCOMES = ['success', 'partial', 'failed', 'abandoned']
const REASONS = ['repro_missing', 'context_drift', 'tool_misuse', 'dependency_trap', 'unknown']

const isEditingOutcome = ref(false)
const saving = ref(false)
const editOutcome = ref({
  outcome: 'success',
  reason_code: null as string | null
})

function startEditOutcome() {
  if (session.value?.outcome) {
    editOutcome.value = {
      outcome: session.value.outcome.outcome || 'success',
      reason_code: session.value.outcome.reason_code
    }
  }
  isEditingOutcome.value = true
}

async function saveOutcome() {
  if (!session.value) return
  saving.value = true
  try {
    await api.sessions.updateOutcome(session.value.id, editOutcome.value)
    // Refresh session to get updated outcome
    const s = await api.sessions.get(session.value.id)
    session.value = s
    isEditingOutcome.value = false
  } catch (e: any) {
    alert('Failed to save outcome: ' + e.message)
  } finally {
    saving.value = false
  }
}

const tabs = ['Conversation', 'Files', 'Raw']
const rawLoading = ref(false)

interface RawLine {
  type: string; role: string; preview: string; json: string; expanded: boolean
}
const rawLines = ref<RawLine[]>([])

async function fetchSession(id: string) {
  loading.value = true
  error.value = ''
  try {
    const [s, m, f] = await Promise.all([
      api.sessions.get(id),
      api.sessions.messages(id, { limit: 500 }),
      api.sessions.files(id),
    ])
    session.value = s
    messages.value = m.items
    files.value = f
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

async function fetchRaw(id: string) {
  if (rawLines.value.length) return
  rawLoading.value = true
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
          if (typeof content === 'string') preview = content.slice(0, 100)
          else if (Array.isArray(content) && content.length > 0) {
            const first = content[0]
            preview = first.type === 'text' ? (first.text || '').slice(0, 100) : `[${first.type}]`
          }
        }
        return { type, role, preview, json: JSON.stringify(obj, null, 2), expanded: false }
      } catch {
        return { type: 'error', role: '', preview: line.slice(0, 50), json: line, expanded: false }
      }
    })
  } catch { /* */ } finally { rawLoading.value = false }
}

onMounted(() => fetchSession(route.params.id as string))
watch(() => route.params.id, (id) => {
  if (id) { rawLines.value = []; fetchSession(id as string) }
})
watch(activeTab, (tab) => {
  if (tab === 'Raw') fetchRaw(route.params.id as string)
})
</script>

<style scoped>
.session-detail {
  max-width: 1000px;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

/* Header */
.detail-header {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.back-link {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  font-size: var(--bl-text-sm);
  color: var(--bl-text-2);
  margin-bottom: 0.25rem;
}

.back-link:hover { color: var(--bl-accent); opacity: 1; }

.detail-header h1 {
  font-size: var(--bl-text-2xl);
  font-weight: 700;
  line-height: 1.3;
}

.header-meta {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  flex-wrap: wrap;
}

.meta-pill {
  padding: 2px 8px;
  border-radius: var(--bl-radius-pill);
  font-size: var(--bl-text-2xs);
  font-weight: 600;
}

.meta-pill.project {
  background: var(--bl-purple-dim);
  color: var(--bl-c3);
}

.meta-pill.branch {
  background: var(--bl-accent-dim);
  color: var(--bl-accent);
}

.meta-text {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-3);
}

.tag-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
}

.tag-pill {
  font-size: var(--bl-text-2xs);
  color: var(--bl-c3);
  background: var(--bl-purple-dim);
  padding: 1px 6px;
  border-radius: var(--bl-radius-pill);
}

.summary-text {
  font-size: var(--bl-text-sm);
  color: var(--bl-text-2);
  line-height: 1.5;
}

.outcome-row {
  display: flex;
  align-items: center;
  gap: 0.625rem;
}

.outcome-badge {
  font-size: var(--bl-text-2xs);
  font-weight: 600;
  padding: 2px 8px;
  border-radius: var(--bl-radius-pill);
  text-transform: uppercase;
}

.outcome-badge.success { background: rgba(63, 185, 80, 0.15); color: var(--bl-success); }
.outcome-badge.partial { background: rgba(210, 153, 34, 0.15); color: var(--bl-warning); }
.outcome-badge.failed { background: rgba(248, 81, 73, 0.15); color: var(--bl-danger); }
.outcome-badge.abandoned { background: var(--bl-surface-2); color: var(--bl-text-3); }

.reason-pill {
  font-size: var(--bl-text-2xs);
  font-family: var(--bl-font-mono);
  color: var(--bl-text-3);
  background: var(--bl-surface-2);
  padding: 2px 6px;
  border-radius: var(--bl-radius-sm);
}

.outcome-section {
  margin-top: 0.5rem;
}

.outcome-editor {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.btn-text {
  background: none;
  border: none;
  padding: 0;
  color: var(--bl-accent);
  font-size: var(--bl-text-xs);
  cursor: pointer;
  opacity: 0.8;
}

.btn-text:hover { opacity: 1; text-decoration: underline; }

.outcome-goal {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
}

/* Tabs */
.detail-tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--bl-border);
}

.tab-btn {
  background: none;
  border: none;
  padding: 0.625rem 1rem;
  font-size: var(--bl-text-sm);
  font-weight: 500;
  color: var(--bl-text-2);
  cursor: pointer;
  position: relative;
  transition: color 0.15s;
}

.tab-btn:hover { color: var(--bl-text); }

.tab-btn.active {
  color: var(--bl-accent);
}

.tab-btn.active::after {
  content: '';
  position: absolute;
  bottom: -1px;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--bl-accent);
}

.tab-pane {
  padding-top: 0.5rem;
}

/* Files */
.files-list {
  display: flex;
  flex-direction: column;
}

.file-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.375rem 0;
  border-bottom: 1px solid var(--bl-border);
}

.op-badge {
  font-size: var(--bl-text-2xs);
  font-weight: 600;
  padding: 1px 6px;
  border-radius: var(--bl-radius-sm);
  min-width: 40px;
  text-align: center;
  text-transform: uppercase;
}

.op-badge.read { background: rgba(63, 185, 80, 0.1); color: var(--bl-success); }
.op-badge.write { background: rgba(210, 153, 34, 0.1); color: var(--bl-warning); }
.op-badge.edit { background: rgba(93, 204, 203, 0.1); color: var(--bl-accent); }

.file-path {
  font-size: var(--bl-text-xs);
  color: var(--bl-text);
  word-break: break-all;
}

/* Raw */
.raw-feed {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.raw-entry {
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  overflow: hidden;
}

.raw-header {
  padding: 0.375rem 0.75rem;
  background: var(--bl-surface);
  cursor: pointer;
  display: flex;
  gap: 0.625rem;
  align-items: center;
  font-size: var(--bl-text-xs);
}

.raw-header:hover { background: var(--bl-surface-2); }

.raw-type { font-weight: 600; color: var(--bl-accent); min-width: 70px; }
.raw-role { color: var(--bl-text-2); min-width: 50px; }
.raw-preview { flex: 1; color: var(--bl-text-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.raw-toggle { color: var(--bl-text-3); font-size: 0.6rem; }

.raw-code {
  padding: 0.75rem;
  background: var(--bl-surface-2);
  font-size: var(--bl-text-xs);
  margin: 0;
  border: none;
  border-radius: 0;
  border-top: 1px solid var(--bl-border);
}

/* States */
.loading-state, .error-state {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4rem;
  color: var(--bl-text-2);
}

.spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--bl-border);
  border-top-color: var(--bl-accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

.empty-tab {
  text-align: center;
  padding: 2rem;
  color: var(--bl-text-2);
  font-size: var(--bl-text-sm);
}

.loading-inline {
  padding: 2rem;
  text-align: center;
  color: var(--bl-text-2);
}
</style>
