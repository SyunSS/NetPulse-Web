<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, h } from 'vue'
import { useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { usePlanStore } from '@/stores/plan'
import { useTaskStore } from '@/stores/task'
import { getWsClient, type ProgressMessage } from '@/api/ws'
import { formatMs, formatTime } from '@/utils'
import VChart from 'vue-echarts'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { LineChart, PieChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent, TitleComponent } from 'echarts/components'

use([CanvasRenderer, LineChart, PieChart, GridComponent, TooltipComponent, LegendComponent, TitleComponent])

const router = useRouter()
const message = useMessage()
const planStore = usePlanStore()
const taskStore = useTaskStore()
const ws = getWsClient()

let unsubWs: (() => void) | null = null

const trendOption = ref<any>({
  tooltip: { trigger: 'axis' },
  legend: { data: ['DNS', 'TTFB', '页面打开'], top: 0, textStyle: { color: '#888' } },
  grid: { left: 50, right: 20, top: 40, bottom: 30 },
  xAxis: {
    type: 'category',
    data: [],
    axisLabel: { color: '#888', formatter: (v: string) => v.substring(11, 19) }
  },
  yAxis: { type: 'value', name: 'ms', nameTextStyle: { color: '#888' }, axisLabel: { color: '#888' }, splitLine: { lineStyle: { color: 'rgba(128,128,128,0.1)' } } },
  series: [
    { name: 'DNS', type: 'line', smooth: true, data: [], lineStyle: { width: 2 } },
    { name: 'TTFB', type: 'line', smooth: true, data: [], lineStyle: { width: 2 } },
    { name: '页面打开', type: 'line', smooth: true, data: [], lineStyle: { width: 2 } },
  ],
})

const pieOption = ref<any>({
  tooltip: { trigger: 'item' },
  legend: { bottom: 0, textStyle: { color: '#888' } },
  series: [{
    type: 'pie', radius: ['45%', '75%'], center: ['50%', '45%'],
    data: [],
    label: { show: false },
    emphasis: { label: { show: true, fontSize: 16, fontWeight: 'bold' } },
  }],
})

function updateCharts(stats: any) {
  trendOption.value = {
    ...trendOption.value,
    xAxis: { ...trendOption.value.xAxis, data: stats.trend_data.map((d: any) => d.time) },
    series: [
      { ...trendOption.value.series[0], data: stats.trend_data.map((d: any) => d.dns_ms) },
      { ...trendOption.value.series[1], data: stats.trend_data.map((d: any) => d.ttfb_ms) },
      { ...trendOption.value.series[2], data: stats.trend_data.map((d: any) => d.page_ms) },
    ],
  }
  const counts = { website: 0, video: 0, download: 0 }
  stats.recent_tasks.forEach((t: any) => {
    if (counts[t.task_type as keyof typeof counts] !== undefined) {
      counts[t.task_type as keyof typeof counts]++
    }
  })
  pieOption.value = {
    ...pieOption.value,
    series: [{
      ...pieOption.value.series[0],
      data: [
        { name: '网站测试', value: counts.website },
        { name: '视频测试', value: counts.video },
        { name: '下载测试', value: counts.download },
      ],
    }],
  }
}

function handleWsMessage(msg: ProgressMessage) {
  if (['task_completed', 'task_failed', 'progress_update'].includes(msg.type)) {
    taskStore.refreshDashboard()
  }
}

onMounted(() => {
  taskStore.connectWs()
  taskStore.refreshDashboard()
  unsubWs = ws.onMessage(handleWsMessage)
  planStore.fetchPlans(1, 5)
})

onUnmounted(() => {
  if (unsubWs) unsubWs()
})

watch(() => taskStore.dashboardStats, (stats) => {
  if (stats) updateCharts(stats)
})

// 下次执行时间排序
const upcomingPlans = () => {
  return planStore.plans
    .filter(p => p.cron_expression && p.enabled === 1 && p.next_run_at)
    .sort((a, b) => (a.next_run_at || '').localeCompare(b.next_run_at || ''))
    .slice(0, 5)
}

const enabledPlanCount = () => planStore.plans.filter(p => p.enabled === 1).length
const cronPlanCount = () => planStore.plans.filter(p => p.cron_expression).length
const totalItems = () => planStore.plans.reduce((sum, p) => sum + p.items.length, 0)
</script>

<template>
  <div class="dashboard">
    <!-- 欢迎横幅 -->
    <div class="welcome-banner">
      <div class="banner-content">
        <h2 class="banner-title">👋 欢迎使用 NetPulse Web</h2>
        <p class="banner-subtitle">专业的网络质量测试平台 · 支持网站、视频、下载三大引擎</p>
      </div>
      <button class="banner-btn" @click="router.push('/plans/new')">+ 新建计划</button>
    </div>

    <!-- 统计卡片 -->
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-icon" style="background: rgba(32, 128, 240, 0.1); color: var(--color-primary)">📋</div>
        <div class="stat-content">
          <div class="stat-label">启用计划</div>
          <div class="stat-value">{{ enabledPlanCount() }}</div>
          <div class="stat-sub">共 {{ planStore.total }} 个计划</div>
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-icon" style="background: rgba(24, 160, 88, 0.1); color: var(--color-success)">⏰</div>
        <div class="stat-content">
          <div class="stat-label">定时任务</div>
          <div class="stat-value">{{ cronPlanCount() }}</div>
          <div class="stat-sub">含 cron 调度</div>
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-icon" style="background: rgba(240, 160, 32, 0.1); color: var(--color-warning)">🎯</div>
        <div class="stat-content">
          <div class="stat-label">测试项</div>
          <div class="stat-value">{{ totalItems() }}</div>
          <div class="stat-sub">所有计划合计</div>
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-icon" style="background: rgba(208, 48, 80, 0.1); color: var(--color-danger)">🕘</div>
        <div class="stat-content">
          <div class="stat-label">今日测试</div>
          <div class="stat-value">{{ taskStore.dashboardStats?.today_tests ?? 0 }}</div>
          <div class="stat-sub">{{ (taskStore.dashboardStats?.success_rate ?? 0).toFixed(1) }}% 成功率</div>
        </div>
      </div>
    </div>

    <!-- 性能指标 -->
    <div class="metrics-grid">
      <div class="metric-card">
        <div class="metric-icon">⚡</div>
        <div class="metric-content">
          <div class="metric-label">平均 DNS</div>
          <div class="metric-value">{{ formatMs(taskStore.dashboardStats?.avg_dns ?? 0) }}</div>
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-icon">🌐</div>
        <div class="metric-content">
          <div class="metric-label">平均 TTFB</div>
          <div class="metric-value">{{ formatMs(taskStore.dashboardStats?.avg_ttfb ?? 0) }}</div>
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-icon">📄</div>
        <div class="metric-content">
          <div class="metric-label">平均首屏</div>
          <div class="metric-value">{{ formatMs(taskStore.dashboardStats?.avg_page_time ?? 0) }}</div>
        </div>
      </div>
    </div>

    <!-- 图表 + 即将执行 -->
    <div class="charts-grid">
      <div class="chart-card">
        <h3 class="card-title">📈 性能趋势</h3>
        <v-chart :option="trendOption" style="height: 280px" autoresize />
      </div>
      <div class="chart-card">
        <h3 class="card-title">📊 任务类型分布</h3>
        <v-chart :option="pieOption" style="height: 280px" autoresize />
      </div>
    </div>

    <!-- 最近计划 + 即将执行 -->
    <div class="bottom-grid">
      <div class="bottom-card">
        <div class="card-header">
          <h3 class="card-title">📅 即将执行</h3>
          <button class="link-btn" @click="router.push('/plans')">查看全部 →</button>
        </div>
        <div v-if="upcomingPlans().length === 0" class="mini-empty">
          暂无定时计划
        </div>
        <div v-else class="upcoming-list">
          <div
            v-for="plan in upcomingPlans()"
            :key="plan.id"
            class="upcoming-item"
            @click="router.push(`/plans/${plan.id}/runs`)"
          >
            <div class="upcoming-icon">⏰</div>
            <div class="upcoming-content">
              <div class="upcoming-name">{{ plan.name }}</div>
              <div class="upcoming-time">{{ formatTime(plan.next_run_at!) }}</div>
            </div>
          </div>
        </div>
      </div>
      <div class="bottom-card">
        <div class="card-header">
          <h3 class="card-title">📋 最近计划</h3>
          <button class="link-btn" @click="router.push('/plans')">查看全部 →</button>
        </div>
        <div v-if="planStore.plans.length === 0" class="mini-empty">
          还没有计划
        </div>
        <div v-else class="recent-plans-list">
          <div
            v-for="plan in planStore.plans.slice(0, 5)"
            :key="plan.id"
            class="recent-plan-item"
            @click="router.push(`/plans/${plan.id}/runs`)"
          >
            <div class="recent-plan-name">{{ plan.name }}</div>
            <div class="recent-plan-items">{{ plan.items.length }} 项 · {{ plan.enabled === 1 ? '启用' : '禁用' }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dashboard {
  max-width: 1400px;
  margin: 0 auto;
}

.welcome-banner {
  background: var(--gradient-primary);
  border-radius: var(--radius-lg);
  padding: 24px 32px;
  margin-bottom: 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: white;
  box-shadow: var(--shadow-md);
}

.banner-title {
  font-size: 22px;
  font-weight: 700;
  margin: 0 0 6px 0;
}

.banner-subtitle {
  font-size: 13px;
  opacity: 0.9;
  margin: 0;
}

.banner-btn {
  padding: 10px 20px;
  background: rgba(255, 255, 255, 0.2);
  border: 1px solid rgba(255, 255, 255, 0.3);
  color: white;
  border-radius: var(--radius-md);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
  backdrop-filter: blur(10px);
}

.banner-btn:hover {
  background: rgba(255, 255, 255, 0.3);
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 16px;
}

.stat-card {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  padding: 20px;
  display: flex;
  align-items: center;
  gap: 16px;
  transition: all var(--transition-base);
}

.stat-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
}

.stat-icon {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  flex-shrink: 0;
}

.stat-label {
  font-size: 12px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 500;
}

.stat-value {
  font-size: 26px;
  font-weight: 700;
  color: var(--text-primary);
  margin-top: 2px;
  font-family: var(--font-mono);
}

.stat-sub {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  margin-bottom: 16px;
}

.metric-card {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: 16px 20px;
  display: flex;
  align-items: center;
  gap: 12px;
}

.metric-icon {
  font-size: 24px;
}

.metric-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.metric-value {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary);
  font-family: var(--font-mono);
}

.charts-grid {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 16px;
  margin-bottom: 16px;
}

.chart-card, .bottom-card {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  padding: 20px;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 16px 0;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.card-header .card-title {
  margin: 0;
}

.link-btn {
  background: none;
  border: none;
  color: var(--color-primary);
  font-size: 13px;
  cursor: pointer;
  padding: 4px 8px;
}

.link-btn:hover {
  text-decoration: underline;
}

.bottom-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.mini-empty {
  padding: 40px 0;
  text-align: center;
  color: var(--text-tertiary);
  font-size: 13px;
}

.upcoming-list, .recent-plans-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.upcoming-item, .recent-plan-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  background: var(--bg-body);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.upcoming-item:hover, .recent-plan-item:hover {
  background: var(--bg-hover);
}

.upcoming-icon {
  font-size: 18px;
  flex-shrink: 0;
}

.upcoming-content {
  flex: 1;
  min-width: 0;
}

.upcoming-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.upcoming-time {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--font-mono);
  margin-top: 2px;
}

.recent-plan-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.recent-plan-items {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 2px;
}
</style>
