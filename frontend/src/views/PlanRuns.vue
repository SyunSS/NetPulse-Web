<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { usePlanStore } from '@/stores/plan'
import { useAuthStore } from '@/stores/auth'
import { getWsClient, type ProgressMessage } from '@/api/ws'
import { formatTime } from '@/utils'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const planStore = usePlanStore()
const authStore = useAuthStore()

const planId = route.params.id as string
const ws = getWsClient()
let unsubWs: (() => void) | null = null

function parseTaskIds(ids: string): string[] {
  try { return JSON.parse(ids) } catch { return [] }
}

function handleWsMessage(msg: ProgressMessage) {
  if (msg.task_id === planId) planStore.fetchPlanRuns(planId)
}

async function handleExport(taskId: string, format: string, type: string) {
  try {
    const url = type === 'website' ? `/api/task/${taskId}/export?format=${format}`
      : type === 'video' ? `/api/task/${taskId}/export?format=${format}`
      : `/api/task/${taskId}/export?format=${format}`
    const resp = await fetch(url, { headers: { Authorization: `Bearer ${authStore.token}` } })
    if (!resp.ok) throw new Error('导出失败')
    const blob = await resp.blob()
    const a = document.createElement('a')
    a.href = URL.createObjectURL(blob)
    a.download = `result_${taskId.substring(0, 8)}.${format === 'xlsx' ? 'xlsx' : format === 'csv' ? 'csv' : 'json'}`
    a.click()
    URL.revokeObjectURL(a.href)
    message.success('导出成功')
  } catch (e: any) { message.error(e.message || '导出失败') }
}

onMounted(() => {
  planStore.fetchPlan(planId)
  planStore.fetchPlanRuns(planId)
  ws.connect(planId)
  unsubWs = ws.onMessage(handleWsMessage)
})

onUnmounted(() => { if (unsubWs) unsubWs() })

const triggerLabel = (t: string) => t === 'cron' ? '定时' : '手动'
const triggerColor = (t: string) => t === 'cron' ? 'var(--color-primary)' : 'var(--color-success)'
const statusText = (s: string) => s === 'completed' ? '已完成' : s === 'running' ? '运行中' : s === 'failed' ? '失败' : s
</script>

<template>
  <div class="runs-page">
    <div class="page-header">
      <button class="back-btn" @click="router.push('/plans')">← 返回</button>
      <div class="header-info">
        <h1 class="page-title">运行历史</h1>
        <p class="page-subtitle" v-if="planStore.currentPlan">
          计划: <strong>{{ planStore.currentPlan.name }}</strong>
        </p>
      </div>
      <div class="header-actions">
        <button class="btn primary" @click="router.push(`/plans/${planId}/edit`)">✎ 编辑</button>
        <button class="btn" @click="planStore.fetchPlanRuns(planId)">↻ 刷新</button>
      </div>
    </div>

    <div v-if="planStore.planRuns.length === 0" class="empty-state">
      <div class="empty-icon">🕘</div><h3>暂无运行历史</h3><p>运行计划后会在此显示结果</p>
    </div>

    <div v-else class="runs-list">
      <div v-for="run in planStore.planRuns" :key="run.id" class="run-card lift">
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
          </div>
        </div>

        <div class="run-body">
          <div class="run-detail">
            <span class="detail-label">任务数:</span>
            <span class="detail-value">{{ run.task_count }} 个</span>
          </div>
          <div class="run-detail" v-if="run.finished_at">
            <span class="detail-label">耗时:</span>
            <span class="detail-value">{{ Math.round((new Date(run.finished_at).getTime() - new Date(run.started_at).getTime()) / 1000) }} 秒</span>
          </div>
        </div>

        <!-- 任务列表 + 操作 -->
        <div class="task-links" v-if="parseTaskIds(run.task_ids).length > 0">
          <div class="task-links-title">关联任务:</div>
          <div v-for="tid in parseTaskIds(run.task_ids)" :key="tid" class="task-link-row">
            <code class="task-id">{{ tid.substring(0, 8) }}...</code>
            <button class="btn sm" @click="router.push(`/task/${tid}`)">📊 查看详情</button>
            <button class="btn sm" @click="handleExport(tid, 'json', '')">📥 JSON</button>
            <button class="btn sm" @click="handleExport(tid, 'csv', '')">📥 CSV</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.runs-page { max-width: 1000px; margin: 0 auto; }
.page-header { display: flex; align-items: center; gap: 16px; margin-bottom: 24px; }
.back-btn { background: none; border: 1px solid var(--border-color); color: var(--text-secondary); padding: 8px 14px; border-radius: var(--radius-sm); cursor: pointer; font-size: 13px; }
.back-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
.header-info { flex: 1; }
.page-title { font-size: 22px; font-weight: 700; color: var(--text-primary); margin: 0; }
.page-subtitle { font-size: 13px; color: var(--text-secondary); margin-top: 4px; }
.header-actions { display: flex; gap: 8px; }
.btn { height: 34px; padding: 0 14px; border: 1px solid var(--border-color); background: var(--bg-card); color: var(--text-primary); border-radius: var(--radius-sm); font-size: 13px; cursor: pointer; font-weight: 500; }
.btn:hover { background: var(--bg-hover); border-color: var(--border-color-hover); }
.btn.primary { background: var(--gradient-primary); color: white; border: none; }
.btn.primary:hover { box-shadow: var(--shadow-glow); }
.btn.sm { height: 28px; padding: 0 10px; font-size: 12px; }
.empty-state { text-align: center; padding: 80px 20px; color: var(--text-secondary); }
.empty-icon { font-size: 64px; margin-bottom: 16px; opacity: 0.5; }
.empty-state h3 { font-size: 18px; color: var(--text-primary); margin-bottom: 8px; }
.runs-list { display: flex; flex-direction: column; gap: 12px; }
.run-card { background: var(--bg-card); border: 1px solid var(--border-color); border-radius: var(--radius-lg); padding: 16px 20px; display: flex; flex-direction: column; gap: 12px; }
.run-header { display: flex; align-items: center; justify-content: space-between; }
.run-meta { display: flex; align-items: center; gap: 10px; }
.run-status-row { display: flex; align-items: center; gap: 6px; }
.task-progress { font-size: 12px; color: var(--text-tertiary); }
.trigger-tag { padding: 2px 8px; font-size: 11px; font-weight: 500; border-radius: 4px; }
.run-time { font-size: 13px; color: var(--text-secondary); font-family: var(--font-mono); }
.run-body { display: flex; gap: 24px; padding-top: 8px; border-top: 1px solid var(--border-color); }
.run-detail { display: flex; align-items: center; gap: 6px; font-size: 13px; }
.detail-label { color: var(--text-tertiary); }
.detail-value { color: var(--text-primary); font-family: var(--font-mono); font-size: 12px; }
.task-links { padding-top: 8px; border-top: 1px solid var(--border-color); display: flex; flex-direction: column; gap: 6px; }
.task-links-title { font-size: 12px; color: var(--text-tertiary); font-weight: 500; }
.task-link-row { display: flex; align-items: center; gap: 6px; }
.task-id { font-size: 11px; color: var(--color-primary); background: rgba(32,128,240,0.08); padding: 2px 6px; border-radius: 4px; }
</style>
