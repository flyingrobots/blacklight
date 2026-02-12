import { ref, computed } from 'vue'

export interface Toast {
  id: number
  level: 'info' | 'warn' | 'error'
  message: string
  timestamp: number
}

const MAX_VISIBLE = 4

const TTL: Record<Toast['level'], number> = {
  info: 5000,
  warn: 8000,
  error: 15000,
}

// Module-level singleton state
const queue = ref<Toast[]>([])
let nextId = 1
let ws: WebSocket | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null

const visible = computed(() => queue.value.slice(0, MAX_VISIBLE))

function push(level: Toast['level'], message: string) {
  const id = nextId++
  const toast: Toast = { id, level, message, timestamp: Date.now() }
  queue.value.push(toast)

  setTimeout(() => dismiss(id), TTL[level])
}

function dismiss(id: number) {
  const idx = queue.value.findIndex((t) => t.id === id)
  if (idx !== -1) queue.value.splice(idx, 1)
}

function connectWs() {
  if (ws) return

  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  const url = `${protocol}//${location.host}/api/ws`

  ws = new WebSocket(url)

  ws.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data)
      if (data.level && data.message) {
        push(data.level, data.message)
      }
    } catch {
      // Ignore malformed messages
    }
  }

  ws.onclose = () => {
    ws = null
    reconnectTimer = setTimeout(connectWs, 3000)
  }

  ws.onerror = () => {
    ws?.close()
  }
}

function disconnectWs() {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
  if (ws) {
    ws.onclose = null // Prevent reconnect
    ws.close()
    ws = null
  }
}

export function useNotifications() {
  return { queue, visible, push, dismiss, connectWs, disconnectWs }
}
