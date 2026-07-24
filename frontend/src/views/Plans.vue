<script setup lang="ts">
import { onMounted, h, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useDialog, useMessage } from 'naive-ui'
import { usePlanStore } from '@/stores/plan'
import { formatTime } from '@/utils'

const router = useRouter()
const message = useMessage()
const planStore = usePlanStore()

const searchText = ref('')

const taskTypeLabel = (t: string) => {
  const map: Record<string, string> = {
    website: '网站测试',
    video: '视频测试',
    download: '下载测试',
    ping: 'Ping 测试',
  }
  return map[t] || t
}

const taskTypeColor = (t: string) => {
  const map: Record<string, string> = {
    website: 'var(--color-primary)',
    video: 'var(--color-warning)',
    download: 'var(--color-success)',
    ping: 'var(--color-info)',
  }
  return map[t] || 'var(--text-secondary)'
}

const parseUrls = (json: string): string[] => {
  try {
    return JSON.parse(json)
  } catch {
    return []
  }
}

async function handleRun(planId: string, planName: string) {
  try {
    const res = await planStore.runPlan(planId)
    message.success(`计划「${planName}」已启动，生成 ${res.task_ids.length} 个任务`)
    router.push(`/plans/${planId}/runs`)
  } catch (e: any) {
    message.error(e.message || '执行失败')
  }
}

async function handleDelete(planId: string, planName: string) {
  const dialog = useDialog()
  dialog.warning({
    title: '删除计划',
    content: `确认删除计划「${planName}」？此操作不可恢复。`,
    positiveText: '确认删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await planStore.deletePlan(planId)
        message.success('已删除')
        await planStore.fetchPlans(1, 20)
      } catch (e: any) {
        message.error(e.message || '删除失败')
      }
    },
  })
}

const filteredPlans = () => {
  if (!searchText.value.trim()) return planStore.plans
  const kw = searchText.value.toLowerCase()
  return planStore.plans.filter(p =>
    p.name.toLowerCase().includes(kw) ||
    (p.description && p.description.toLowerCase().includes(kw))
  )
}

onMounted(async () => {
  await planStore.fetchPlans(1, 20)
})
</script>

<template>
  <div class="plans-page">
    <!-- 工具栏 -->
    <div class="toolbar">
      <div class="search-box">
        <svg class="search-icon" width="14" height="14" viewBox="0 0 14 14" fill="none"><circle cx="6" cy="6" r="4.5" stroke="currentColor" stroke-width="1.5"/><path d="M9.5 9.5L13 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        <input
          v-model="searchText"
          type="text"
          placeholder="搜索计划名或描述..."
          class="search-input"
        />
      </div>
      <button class="primary-btn" @click="router.push('/plans/new')">
        ＋ 新建计划
      </button>
    </div>

    <!-- 加载状态 -->
    <div v-if="planStore.loading && planStore.plans.length === 0" class="empty-state">
      <div class="empty-spinner"></div>
      <p>加载中...</p>
    </div>

    <!-- 空状态 -->
    <div v-else-if="planStore.plans.length === 0" class="empty-state">
      <div class="empty-icon">
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
      </div>
      <h3>还没有计划</h3>
      <p>创建你的第一个测试计划，可手动或定时运行</p>
      <button class="primary-btn" @click="router.push('/plans/new')">＋ 新建计划</button>
    </div>

    <!-- 计划列表 -->
    <div v-else class="plan-grid">
      <div
        v-for="plan in filteredPlans()"
        :key="plan.id"
        class="plan-card"
      >
        <!-- 卡片头 -->
        <div class="plan-card-header">
          <div class="plan-title">
            <h3>{{ plan.name }}</h3>
            <span v-if="plan.enabled === 1" class="status-tag status-completed">启用</span>
            <span v-else class="status-tag status-cancelled">已禁用</span>
          </div>
          <div class="plan-desc" v-if="plan.description">{{ plan.description }}</div>
        </div>

        <!-- 任务项 -->
        <div class="plan-items">
          <div v-for="item in plan.items" :key="item.id" class="plan-item">
            <span class="item-dot" :style="{ background: taskTypeColor(item.task_type) }"></span>
            <span class="item-type">{{ taskTypeLabel(item.task_type) }}</span>
            <span class="item-urls">{{ parseUrls(item.urls).length }} 个 URL</span>
          </div>
        </div>

        <!-- 调度信息 -->
        <div class="plan-schedule" v-if="plan.cron_expression">
          <div class="schedule-dot" style="background: var(--color-primary)"></div>
          <div class="schedule-info">
            <div class="schedule-cron">{{ plan.cron_expression }}</div>
            <div class="schedule-next" v-if="plan.next_run_at">
              下次: {{ formatTime(plan.next_run_at) }}
            </div>
          </div>
        </div>
        <div v-else class="plan-schedule manual">
          <div class="schedule-dot"></div>
          <div class="schedule-info">
            <div class="schedule-cron">仅手动运行</div>
            <div class="schedule-next" v-if="plan.last_run_at">
              上次: {{ formatTime(plan.last_run_at) }}
            </div>
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="plan-actions">
          <button class="action-btn primary" @click="handleRun(plan.id, plan.name)">
            立即运行
          </button>
          <button class="action-btn" @click="router.push(`/plans/${plan.id}/edit`)">
            编辑
          </button>
          <button class="action-btn" @click="router.push(`/plans/${plan.id}/runs`)">
            历史
          </button>
          <button class="action-btn danger" @click="handleDelete(plan.id, plan.name)" title="删除">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none"><path d="M2 4h10M5 4V2.5A.5.5 0 0 1 5.5 2h3a.5.5 0 0 1 .5.5V4m1 0v7.5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.plans-page {
  max-width: 1400px;
  margin: 0 auto;
}

.toolbar {
  display: flex;
  gap: 12px;
  margin-bottom: 24px;
}

.search-box {
  flex: 1;
  position: relative;
  max-width: 400px;
}

.search-icon {
  position: absolute;
  left: 12px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-tertiary);
}

.search-input {
  width: 100%;
  height: 36px;
  padding: 0 12px 0 34px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 14px;
  transition: all var(--transition-fast);
}

.search-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-bg);
}

.primary-btn {
  height: 36px;
  padding: 0 16px;
  border: none;
  background: var(--color-primary);
  color: white;
  border-radius: var(--radius-sm);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.primary-btn:hover {
  background: var(--color-primary-active);
}

.plan-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
  gap: 20px;
}

.plan-card {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.plan-title {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 4px;
}

.plan-title h3 {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}

.plan-desc {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.5;
}

.plan-items {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background: var(--bg-body);
  border-radius: var(--radius-md);
}

.plan-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.item-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.item-type {
  font-weight: 500;
  color: var(--text-primary);
}

.item-urls {
  margin-left: auto;
  color: var(--text-tertiary);
  font-size: 12px;
}

.plan-schedule {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: var(--bg-hover);
  border-radius: var(--radius-sm);
}

.plan-schedule.manual {
  background: var(--bg-hover);
}

.schedule-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-tertiary);
  flex-shrink: 0;
}

.schedule-info {
  flex: 1;
  min-width: 0;
}

.schedule-cron {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--color-primary);
  font-weight: 600;
}

.plan-schedule.manual .schedule-cron {
  color: var(--text-secondary);
}

.schedule-next {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

.plan-actions {
  display: flex;
  gap: 6px;
  padding-top: 12px;
  border-top: 1px solid var(--border-color);
}

.action-btn {
  flex: 1;
  height: 32px;
  padding: 0 10px;
  border: 1px solid var(--border-color);
  background: var(--bg-card);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: 12px;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.action-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
  border-color: var(--border-color-hover);
}

.action-btn.primary {
  background: var(--color-primary);
  color: white;
  border-color: var(--color-primary);
}

.action-btn.primary:hover {
  background: #1976d2;
}

.action-btn.danger {
  flex: 0 0 32px;
}

.action-btn.danger:hover {
  background: var(--color-danger);
  color: white;
  border-color: var(--color-danger);
}

.empty-state {
  text-align: center;
  padding: 80px 20px;
  color: var(--text-secondary);
}

.empty-icon {
  color: var(--text-tertiary);
  margin-bottom: 16px;
}

.empty-spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--border-color);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
  margin: 0 auto 16px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.empty-state h3 {
  font-size: 16px;
  color: var(--text-primary);
  margin-bottom: 8px;
  font-weight: 600;
}

.empty-state p {
  font-size: 14px;
  color: var(--text-secondary);
  margin-bottom: 20px;
}
</style>
