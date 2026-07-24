<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDialog, useMessage } from 'naive-ui'
import { usePlanStore } from '@/stores/plan'
import { getWsClient, type ProgressMessage } from '@/api/ws'
import http from '@/api/index'
import { formatTime } from '@/utils'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const planStore = usePlanStore()

const planId = route.params.id as string
const ws = getWsClient()
let unsubWs: (() => void) | null = null

interface TaskMeta { id: string; type: string; status: string }
const taskMeta = ref<Record<string, TaskMeta>>({})

// 搜索/筛选
const searchStart = ref('')
const searchEnd = ref('')
const searchStatus = ref('all')
const autoRefresh = ref(true)
let refreshTimer: number | null = null

function parseTaskIds(ids: string): string[] {
  try { return JSON.parse(ids) } catch { return [] }
}

async function fetchTaskMeta(ids: string[]) {
  for (const tid of ids) {
    if (taskMeta.value[tid]) continue
    try {
      const res = await http.get(`/task/${tid}`)
      taskMeta.value[tid] = { id: tid, type: res.data.task_type, status: res.data.status }
    } catch {}
  }
}

const typeLabel = (t: string) => t === 'website' ? '网站测试' : t === 'video' ? '视频测试' : t === 'download' ? '下载测试' : t === 'ping' ? 'Ping 测试' : t
const typeColor = (t: string) => t === 'website' ? 'var(--color-primary)' : t === 'video' ? 'var(--color-warning)' : t === 'download' ? 'var(--color-success)' : t === 'ping' ? 'var(--color-info)' : 'var(--text-secondary)'

function handleWsMessage(msg: ProgressMessage) {
  if (msg.task_id !== planId) return
  if (msg.type === 'task_completed' || msg.type === 'task_failed') {
    fetchRuns()
  }
}

async function fetchRuns() {
  try {
    const params: any = {}
    if (searchStart.value) params.start = new Date(searchStart.value).toISOString()
    if (searchEnd.value) params.end = new Date(searchEnd.value + 'T23:59:59').toISOString()
    const res = await planStore.fetchPlanRuns(planId, params)
    // 抓 task type 信息
    for (const run of res) {
      const ids = parseTaskIds(run.task_ids)
      if (ids.length) await fetchTaskMeta(ids)
    }
  } catch (e) { if (import.meta.env.DEV) console.error('fetchRuns:', e) }
}

const filteredRuns = () => {
  let list = planStore.planRuns
  if (searchStatus.value !== 'all') {
    list = list.filter(r => r.status === searchStatus.value)
  }
  return list
}

function resetSearch() {
  searchStart.value = ''
  searchEnd.value = ''
  searchStatus.value = 'all'
  fetchRuns()
}

async function handleDelete(runId: string, force = false) {
  const msg = force
    ? '强制停止并删除此次运行的所有子任务？所有关联任务及结果将被永久删除。'
    : '确认删除这条运行记录？'
  const dialog = useDialog()
  dialog.warning({
    title: force ? '强制删除' : '删除记录',
    content: msg,
    positiveText: '确认',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await planStore.deleteRun(planId, runId, force)
        message.success('已删除')
        fetchRuns()
      } catch (e: any) {
        message.error(e.message || '删除失败')
      }
    },
  })
}

async function handleBatchDelete() {
  const ids = filteredRuns().filter(r => r.status !== 'running').map(r => r.id)
  if (ids.length === 0) { message.warning('没有可删除的记录'); return }
  const dialog = useDialog()
  dialog.warning({
    title: '批量删除',
    content: `确认删除 ${ids.length} 条运行记录？`,
    positiveText: '确认删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await Promise.all(ids.map(id => planStore.deleteRun(planId, id)))
        message.success(`已删除 ${ids.length} 条`)
        fetchRuns()
      } catch (e: any) {
        message.error(e.message || '批量删除失败')
      }
    },
  })
}

async function exportRun(runId: string, format: 'xlsx' | 'csv' | 'json') {
  try {
    const blob: Blob = await http.get(`/plan/${planId}/run/${runId}/export`, {
      params: { format },
      responseType: 'blob',
    })
    const a = document.createElement('a')
    a.href = URL.createObjectURL(blob)
    a.download = `plan_run_${runId.substring(0, 8)}.${format === 'xlsx' ? 'xlsx' : format === 'csv' ? 'csv' : 'json'}`
    a.click()
    URL.revokeObjectURL(a.href)
    message.success('导出成功')
  } catch (e: any) { message.error(e.message || '导出失败') }
}

onMounted(async () => {
  await planStore.fetchPlan(planId)
  await fetchRuns()
  ws.connect(planId)
  unsubWs = ws.onMessage(handleWsMessage)
  // 自动刷新（5s 一次）
  refreshTimer = window.setInterval(() => { if (autoRefresh.value) fetchRuns() }, 5000)
})

onUnmounted(() => {
  if (unsubWs) unsubWs()
  if (refreshTimer) clearInterval(refreshTimer)
})

const triggerLabel = (t: string) => t === 'cron' ? '定时' : '手动'
const triggerColor = (t: string) => t === 'cron' ? 'var(--color-primary)' : 'var(--color-success)'
const statusText = (s: string) => s === 'completed' ? '已完成' : s === 'running' ? '运行中' : s === 'failed' ? '失败' : s
</script>

<template>
  <div class="runs-page">
    <div class="page-header">
      <div>
        <button class="back-btn" @click="router.push('/plans')">← 返回</button>
        <h1 class="page-title">运行历史</h1>
        <p class="page-subtitle" v-if="planStore.currentPlan">计划: <strong>{{ planStore.currentPlan.name }}</strong></p>
      </div>
      <div class="header-actions">
        <button class="btn primary" @click="router.push(`/plans/${planId}/edit`)">编辑</button>
        <button class="btn" @click="fetchRuns">刷新</button>
      </div>
    </div>

    <!-- 搜索/筛选 -->
    <div class="filter-bar">
      <span class="lbl">起:</span>
      <input type="date" v-model="searchStart" class="date-input" />
      <span class="lbl">止:</span>
      <input type="date" v-model="searchEnd" class="date-input" />
      <span class="lbl">状态:</span>
      <select v-model="searchStatus" class="select">
        <option value="all">全部</option>
        <option value="completed">已完成</option>
        <option value="failed">失败</option>
        <option value="cancelled">已取消</option>
        <option value="running">运行中</option>
      </select>
      <button class="btn sm" @click="fetchRuns">搜索</button>
      <button class="btn sm ghost" @click="resetSearch">重置</button>
      <span class="spacer"></span>
      <label class="auto-refresh">
        <input type="checkbox" v-model="autoRefresh" /> 自动刷新
      </label>
      <button class="btn sm danger" @click="handleBatchDelete">批量删除</button>
    </div>

    <div v-if="planStore.planRuns.length === 0" class="empty-state">
      <div class="empty-icon">
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
      </div><h3>暂无运行历史</h3><p>运行计划后会在此显示结果</p>
    </div>

    <div v-else class="runs-list">
      <div v-for="run in filteredRuns()" :key="run.id" class="run-card">
        <div class="run-header">
          <div class="run-meta">
            <span class="trigger-tag" :style="{ background: triggerColor(run.triggered_by) + '20', color: triggerColor(run.triggered_by) }">
              {{ triggerLabel(run.triggered_by) }}
            </span>
            <span class="run-time">{{ formatTime(run.started_at) }}</span>
          </div>
          <div class="run-status-row">
            <span class="status-tag" :class="`status-${run.status}`">{{ statusText(run.status) }}</span>
            <span class="task-progress" v-if="run.task_count > 0">({{ run.completed_count }}/{{ run.task_count }})</span>
            <button v-if="run.status !== 'running'" class="btn-icon danger" @click="handleDelete(run.id)" title="删除">
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none"><path d="M2 4h10M5 4V2.5A.5.5 0 0 1 5.5 2h3a.5.5 0 0 1 .5.5V4m1 0v7.5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </button>
            <button v-if="run.status === 'running'" class="btn-icon danger" @click="handleDelete(run.id, true)" title="强制停止并删除">
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none"><rect x="3" y="3" width="8" height="8" rx="1" fill="currentColor"/></svg>
            </button>
          </div>
        </div>

        <div class="run-body">
          <div class="run-detail"><span class="detail-label">任务数:</span><span class="detail-value">{{ run.task_count }} 个</span></div>
          <div class="run-detail" v-if="run.finished_at">
            <span class="detail-label">耗时:</span>
            <span class="detail-value">{{ Math.round((new Date(run.finished_at).getTime() - new Date(run.started_at).getTime()) / 1000) }} 秒</span>
          </div>
        </div>

        <div class="task-links" v-if="parseTaskIds(run.task_ids).length > 0">
          <div class="task-links-title">关联任务:</div>
          <div v-for="tid in parseTaskIds(run.task_ids)" :key="tid" class="task-link-row">
            <span class="task-type-badge" :style="{ background: typeColor(taskMeta[tid]?.type || '') + '20', color: typeColor(taskMeta[tid]?.type || '') }">
              {{ typeLabel(taskMeta[tid]?.type || '...') }}
            </span>
            <code class="task-id" @click="router.push(`/task/${tid}`)">{{ tid.substring(0, 8) }}...</code>
            <span v-if="taskMeta[tid]" class="task-status-mini" :class="`mini-${taskMeta[tid].status}`">
              {{ taskMeta[tid].status }}
            </span>
            <button class="btn sm" @click="router.push(`/task/${tid}`)">详情</button>
          </div>
        </div>

        <div class="run-export" v-if="run.status === 'completed'">
          <span class="export-label">导出整次结果:</span>
          <button class="btn sm primary" @click="exportRun(run.id, 'xlsx')">Excel</button>
          <button class="btn sm" @click="exportRun(run.id, 'csv')">CSV</button>
          <button class="btn sm" @click="exportRun(run.id, 'json')">JSON</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.runs-page { max-width: 1000px; margin: 0 auto; }
.page-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 20px; }
.page-title { font-size: 22px; font-weight: 700; margin: 8px 0 0 0; }
.page-subtitle { font-size: 13px; color: var(--text-secondary); margin-top: 4px; }
.header-actions { display: flex; gap: 8px; }

.back-btn { background: none; border: 1px solid var(--border-color); color: var(--text-secondary); padding: 8px 14px; border-radius: var(--radius-sm); cursor: pointer; font-size: 13px; }
.back-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
.btn { height: 32px; padding: 0 14px; border: 1px solid var(--border-color); background: var(--bg-card); color: var(--text-primary); border-radius: var(--radius-sm); font-size: 13px; cursor: pointer; font-weight: 500; }
.btn:hover { background: var(--bg-hover); border-color: var(--border-color-hover); }
.btn.primary { background: var(--color-primary); color: white; border: none; }
.btn.primary:hover { background: var(--color-primary-active); }
.btn.danger { color: var(--color-danger); border-color: var(--color-danger); }
.btn.danger:hover { background: var(--color-danger); color: white; }
.btn.ghost { background: transparent; }
.btn.sm { height: 28px; padding: 0 10px; font-size: 12px; }
.btn-icon { width: 28px; height: 28px; border: 1px solid var(--border-color); background: var(--bg-card); color: var(--text-secondary); border-radius: var(--radius-sm); cursor: pointer; }
.btn-icon:hover { background: var(--color-danger); color: white; border-color: var(--color-danger); }
.btn-icon.danger { color: var(--color-danger); }

.filter-bar { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 16px; padding: 10px 14px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: var(--radius-md); }
.lbl { font-size: 12px; color: var(--text-secondary); }
.date-input, .select { height: 28px; padding: 0 8px; border: 1px solid var(--border-color); border-radius: var(--radius-sm); background: var(--bg-card); color: var(--text-primary); font-size: 12px; }
.spacer { flex: 1; }
.auto-refresh { font-size: 12px; color: var(--text-secondary); display: flex; align-items: center; gap: 4px; }
.auto-refresh input { margin: 0; }

.empty-state { text-align: center; padding: 80px 20px; color: var(--text-secondary); }
.empty-icon { color: var(--text-tertiary); margin-bottom: 12px; }
.empty-state h3 { font-size: 16px; color: var(--text-primary); margin-bottom: 4px; }

.runs-list { display: flex; flex-direction: column; gap: 12px; }
.run-card { background: var(--bg-card); border: 1px solid var(--border-color); border-radius: var(--radius-lg); padding: 16px 20px; display: flex; flex-direction: column; gap: 12px; }
.run-header { display: flex; align-items: center; justify-content: space-between; }
.run-meta { display: flex; align-items: center; gap: 10px; }
.run-status-row { display: flex; align-items: center; gap: 6px; }
.task-progress { font-size: 12px; color: var(--text-tertiary); }
.trigger-tag { padding: 2px 8px; font-size: 11px; font-weight: 500; border-radius: var(--radius-sm); }
.run-time { font-size: 13px; color: var(--text-secondary); font-family: var(--font-mono); }
.run-body { display: flex; gap: 24px; padding-top: 8px; border-top: 1px solid var(--border-color); }
.run-detail { display: flex; align-items: center; gap: 6px; font-size: 13px; }
.detail-label { color: var(--text-tertiary); }
.detail-value { color: var(--text-primary); font-family: var(--font-mono); font-size: 12px; }
.task-links { padding-top: 8px; border-top: 1px solid var(--border-color); display: flex; flex-direction: column; gap: 6px; }
.task-links-title { font-size: 12px; color: var(--text-tertiary); font-weight: 500; }
.task-link-row { display: flex; align-items: center; gap: 8px; }
.task-type-badge { padding: 2px 8px; font-size: 11px; font-weight: 500; border-radius: var(--radius-sm); }
.task-id { font-size: 11px; color: var(--color-primary); background: var(--color-primary-bg); padding: 3px 8px; border-radius: var(--radius-sm); cursor: pointer; font-family: var(--font-mono); }
.task-id:hover { background: var(--color-primary-bg); }
.task-status-mini { font-size: 11px; padding: 2px 6px; border-radius: var(--radius-sm); }
.mini-completed { background: rgba(26,174,57,0.12); color: var(--color-success); }
.mini-failed { background: rgba(208,48,80,0.12); color: var(--color-danger); }
.mini-running, .mini-pending { background: var(--color-primary-bg); color: var(--color-primary-text); }
.run-export { display: flex; align-items: center; gap: 6px; padding-top: 8px; border-top: 1px solid var(--border-color); }
.export-label { font-size: 12px; color: var(--text-tertiary); margin-right: 4px; }

.status-tag { padding: 2px 8px; border-radius: var(--radius-pill); font-size: 11px; font-weight: 600; letter-spacing: 0.125px; }
.status-completed { background: rgba(26,174,57,0.12); color: var(--color-success); }
.status-running { background: var(--color-primary-bg); color: var(--color-primary-text); }
.status-pending, .status-cancelled { background: rgba(163,158,152,0.15); color: var(--text-tertiary); }
.status-failed { background: rgba(208,48,80,0.12); color: var(--color-danger); }
</style>
