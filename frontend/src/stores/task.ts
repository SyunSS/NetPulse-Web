import { defineStore } from 'pinia'
import { ref } from 'vue'
import { taskApi, dashboardApi, type TestTask, type WebsiteResult, type DashboardStats } from '@/api/task'
import { getWsClient, type ProgressMessage } from '@/api/ws'

export const useTaskStore = defineStore('task', () => {
  const activeTasks = ref<Map<string, TestTask>>(new Map())
  const taskProgress = ref<Map<string, number>>(new Map())
  const taskLogs = ref<Map<string, string[]>>(new Map())
  const dashboardStats = ref<DashboardStats | null>(null)

  let wsConnected = false

  /** 连接 WebSocket */
  function connectWs() {
    if (wsConnected) return
    wsConnected = true
    const ws = getWsClient()
    ws.connect()
    ws.onMessage(handleWsMessage)
  }

  /** 处理 WebSocket 消息 */
  function handleWsMessage(msg: ProgressMessage) {
    switch (msg.type) {
      case 'task_started':
        taskProgress.value.set(msg.task_id, 0)
        break
      case 'progress_update':
        taskProgress.value.set(msg.task_id, msg.progress)
        break
      case 'log':
        addLog(msg.task_id, msg.message)
        break
      case 'task_completed':
      case 'task_failed':
        // 任务结束，刷新 dashboard
        refreshDashboard()
        break
    }
  }

  function addLog(taskId: string, message: string) {
    const logs = taskLogs.value.get(taskId) || []
    logs.push(message)
    taskLogs.value.set(taskId, logs)
  }

  /** 创建任务 */
  async function createTask(taskType: string, urls: string[]) {
    const res = await taskApi.create({ task_type: taskType, urls })
    return res.data
  }

  /** 获取任务列表 */
  async function fetchTaskList(page = 1, size = 20) {
    const res = await taskApi.list(page, size)
    return res.data
  }

  /** 获取任务结果 */
  async function fetchTaskResults(taskId: string): Promise<WebsiteResult[]> {
    const res = await taskApi.getResults(taskId)
    return res.data
  }

  /** 取消任务 */
  async function cancelTask(taskId: string) {
    await taskApi.cancel(taskId)
  }

  /** 重试任务 */
  async function retryTask(taskId: string) {
    const res = await taskApi.retry(taskId)
    return res.data
  }

  /** 刷新 Dashboard 统计 */
  async function refreshDashboard() {
    try {
      const res = await dashboardApi.getStats()
      dashboardStats.value = res.data
    } catch (e) {
      if (import.meta.env.DEV) console.error('刷新Dashboard失败:', e)
    }
  }

  return {
    activeTasks,
    taskProgress,
    taskLogs,
    dashboardStats,
    connectWs,
    createTask,
    fetchTaskList,
    fetchTaskResults,
    cancelTask,
    retryTask,
    refreshDashboard,
  }
})
