/**
 * WebSocket 客户端 — 接收实时测试进度
 */
import { useAuthStore } from '@/stores/auth'

let wsClient: WsClient | null = null

if (typeof window !== 'undefined') {
  window.addEventListener('netpulse:logout', () => wsClient?.close())
}

export type ProgressMessage =
  | { type: 'task_started'; task_id: string; total_urls: number }
  | { type: 'url_testing'; task_id: string; url: string; current: number; total: number }
  | { type: 'url_completed'; task_id: string; url: string; result: unknown }
  | { type: 'progress_update'; task_id: string; progress: number }
  | { type: 'log'; task_id: string; level: string; message: string }
  | { type: 'task_completed'; task_id: string; success_count: number; fail_count: number }
  | { type: 'task_failed'; task_id: string; error: string }
  | { type: 'subscribed'; task_id: string }

type MessageHandler = (msg: ProgressMessage) => void

export class WsClient {
  private ws: WebSocket | null = null
  private handlers: Set<MessageHandler> = new Set()
  private reconnectTimer: number | null = null
  private taskId: string | null = null
  private shouldReconnect = false

  connect(taskId?: string) {
    this.shouldReconnect = true
    this.clearReconnectTimer()
    if (this.ws) {
      this.ws.onclose = null
      this.ws.close()
      this.ws = null
    }
    this.taskId = taskId || null
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const host = window.location.host
    const token = useAuthStore().token
    const url = taskId
      ? `${protocol}//${host}/api/ws/?task_id=${taskId}&token=${encodeURIComponent(token)}`
      : `${protocol}//${host}/api/ws/?token=${encodeURIComponent(token)}`

    this.ws = new WebSocket(url)

    this.ws.onopen = () => {
      if (import.meta.env.DEV) console.log('[WS] 连接已建立')
    }

    this.ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data) as ProgressMessage
        this.handlers.forEach((h) => h(msg))
      } catch (e) {
        if (import.meta.env.DEV) console.error('[WS] 消息解析失败:', e)
      }
    }

    const socket = this.ws
    socket.onclose = () => {
      if (import.meta.env.DEV) console.log('[WS] 连接已关闭')
      if (this.ws === socket) this.ws = null
      if (this.shouldReconnect) {
        this.reconnectTimer = window.setTimeout(() => this.connect(this.taskId || undefined), 3000)
      }
    }

    this.ws.onerror = (e) => {
      if (import.meta.env.DEV) console.error('[WS] 错误:', e)
    }
  }

  onMessage(handler: MessageHandler) {
    this.handlers.add(handler)
    return () => this.handlers.delete(handler)
  }

  close() {
    this.shouldReconnect = false
    this.clearReconnectTimer()
    this.taskId = null
    if (this.ws) {
      this.ws.onclose = null
      this.ws.close()
      this.ws = null
    }
    this.handlers.clear()
  }

  private clearReconnectTimer() {
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
  }
}

export function getWsClient(): WsClient {
  if (!wsClient) {
    wsClient = new WsClient()
  }
  return wsClient
}

export function closeWsClient() {
  wsClient?.close()
}
