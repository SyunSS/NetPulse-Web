<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { usePlanStore } from '@/stores/plan'
import { getWsClient, type ProgressMessage } from '@/api/ws'
import { formatTime } from '@/utils'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const planStore = usePlanStore()

const planId = route.params.id as string
const ws = getWsClient()
let unsubWs: (() => void) | null = null

function handleWsMessage(msg: ProgressMessage) {
  // 收到与当前 plan_run 相关的进度就刷新
  if (msg.task_id === planId) {
    planStore.fetchPlanRuns(planId)
  }
}

onMounted(() => {
  planStore.fetchPlan(planId)
  planStore.fetchPlanRuns(planId)
  ws.connect(planId)
  unsubWs = ws.onMessage(handleWsMessage)
})

onUnmounted(() => {
  if (unsubWs) unsubWs()
})

const triggerLabel = (t: string) => {
  return t === 'cron' ? '定时' : '手动'
}

const triggerColor = (t: string) => {
  return t === 'cron' ? 'var(--color-primary)' : 'var(--color-success)'
}
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
        <button class="action-btn primary" @click="router.push(`/plans/${planId}/edit`)">✎ 编辑计划</button>
        <button class="action-btn" @click="planStore.fetchPlanRuns(planId)">↻ 刷新</button>
      </div>
    </div>

    <div v-if="planStore.planRuns.length === 0" class="empty-state">
      <div class="empty-icon">🕘</div>
      <h3>暂无运行历史</h3>
      <p>运行计划后会在此显示结果</p>
    </div>

    <div v-else class="runs-list">
      <div
        v-for="run in planStore.planRuns"
        :key="run.id"
        class="run-card lift"
      >
        <div class="run-header">
          <div class="run-meta">
            <span
              class="trigger-tag"
              :style="{ background: triggerColor(run.triggered_by) + '20', color: triggerColor(run.triggered_by) }"
            >
              {{ triggerLabel(run.triggered_by) }}
            </span>
            <span class="run-time">{{ formatTime(run.started_at) }}</span>
          </div>
          <span class="status-tag" :class="`status-${run.status}`">
            {{ run.status }}
          </span>
        </div>
        <div class="run-body">
          <div class="run-detail">
            <span class="detail-label">任务ID:</span>
            <code class="detail-value">{{ run.task_id?.substring(0, 8) || '-' }}...</code>
          </div>
          <div class="run-detail" v-if="run.finished_at">
            <span class="detail-label">耗时:</span>
            <span class="detail-value">{{
              Math.round((new Date(run.finished_at).getTime() - new Date(run.started_at).getTime()) / 1000)
            }} 秒</span>
          </div>
          <div class="run-detail" v-else>
            <span class="detail-label">状态:</span>
            <span class="detail-value">运行中...</span>
          </div>
        </div>
        <div v-if="run.task_id" class="run-actions">
          <button class="action-btn" @click="router.push(`/task/${run.task_id}`)">查看任务详情</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.runs-page {
  max-width: 1000px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 24px;
}

.back-btn {
  background: none;
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  padding: 8px 14px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 13px;
  transition: all var(--transition-fast);
}

.back-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.header-info {
  flex: 1;
}

.page-title {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.page-subtitle {
  font-size: 13px;
  color: var(--text-secondary);
  margin-top: 4px;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.action-btn {
  height: 36px;
  padding: 0 16px;
  border: 1px solid var(--border-color);
  background: var(--bg-card);
  color: var(--text-secondary);
  border-radius: var(--radius-md);
  font-size: 13px;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.action-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.action-btn.primary {
  background: var(--gradient-primary);
  color: white;
  border: none;
  font-weight: 600;
}

.action-btn.primary:hover {
  box-shadow: var(--shadow-glow);
}

.empty-state {
  text-align: center;
  padding: 80px 20px;
  color: var(--text-secondary);
}

.empty-icon {
  font-size: 64px;
  margin-bottom: 16px;
  opacity: 0.5;
}

.empty-state h3 {
  font-size: 18px;
  color: var(--text-primary);
  margin-bottom: 8px;
}

.runs-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.run-card {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.run-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.run-meta {
  display: flex;
  align-items: center;
  gap: 10px;
}

.trigger-tag {
  padding: 2px 8px;
  font-size: 11px;
  font-weight: 500;
  border-radius: 4px;
}

.run-time {
  font-size: 13px;
  color: var(--text-secondary);
  font-family: var(--font-mono);
}

.run-body {
  display: flex;
  gap: 24px;
  padding-top: 8px;
  border-top: 1px solid var(--border-color);
}

.run-detail {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
}

.detail-label {
  color: var(--text-tertiary);
}

.detail-value {
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: 12px;
}

.run-actions {
  display: flex;
  gap: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--border-color);
}
</style>
