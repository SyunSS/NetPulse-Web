<script setup lang="ts">
import { ref, onMounted, onUnmounted, h, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { useTaskStore } from '@/stores/task'
import { getWsClient, type ProgressMessage } from '@/api/ws'
import type { DashboardStats } from '@/api/task'
import { formatMs, formatTime } from '@/utils'
import VChart from 'vue-echarts'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { LineChart, PieChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent, TitleComponent } from 'echarts/components'

use([CanvasRenderer, LineChart, PieChart, GridComponent, TooltipComponent, LegendComponent, TitleComponent])

const router = useRouter()
const message = useMessage()
const taskStore = useTaskStore()

const showCreateModal = ref(false)
const taskType = ref('website')
const urlInput = ref('')
const creating = ref(false)
const ws = getWsClient()

let unsubWs: (() => void) | null = null

// 预置测试 URL
const presetUrls: Record<string, string[]> = {
  '国内常用': ['https://www.baidu.com', 'https://www.qq.com', 'https://www.taobao.com', 'https://www.jd.com'],
  '国际网站': ['https://www.google.com', 'https://www.youtube.com', 'https://www.github.com', 'https://www.wikipedia.org'],
  '视频网站': ['https://www.bilibili.com', 'https://www.youtube.com/watch?v=dQw4w9WgXcQ'],
  '下载测试': ['http://speedtest.tele2.net/1MB.zip', 'http://speedtest.tele2.net/10MB.zip'],
}

const recentTasks = ref<any[]>([])

const taskTypeOptions = [
  { label: '网站测试', value: 'website' },
  { label: '下载测试', value: 'download' },
  { label: '视频测试', value: 'video' },
]

// ECharts 趋势图
const trendOption = ref({
  tooltip: { trigger: 'axis' },
  legend: { data: ['DNS', 'TTFB', '页面打开'], top: 0 },
  grid: { left: 50, right: 20, top: 40, bottom: 30 },
  xAxis: { type: 'category', data: [] as string[], axisLabel: { formatter: (v: string) => v.substring(11, 19) } },
  yAxis: { type: 'value', name: 'ms' },
  series: [
    { name: 'DNS', type: 'line', smooth: true, data: [] as number[], lineStyle: { width: 2 } },
    { name: 'TTFB', type: 'line', smooth: true, data: [] as number[], lineStyle: { width: 2 } },
    { name: '页面打开', type: 'line', smooth: true, data: [] as number[], lineStyle: { width: 2 } },
  ],
})

// 任务类型分布饼图
const pieOption = ref({
  tooltip: { trigger: 'item' },
  legend: { bottom: 0 },
  series: [{
    type: 'pie', radius: ['45%', '75%'], center: ['50%', '45%'],
    data: [] as { name: string; value: number }[],
    label: { show: false },
    emphasis: { label: { show: true, fontSize: 16, fontWeight: 'bold' } },
  }],
})

function updateCharts(stats: DashboardStats) {
  // 趋势图
  trendOption.value = {
    ...trendOption.value,
    xAxis: { ...trendOption.value.xAxis, data: stats.trend_data.map(d => d.time.substring(11, 19)) },
    series: [
      { ...trendOption.value.series[0], data: stats.trend_data.map(d => d.dns_ms) },
      { ...trendOption.value.series[1], data: stats.trend_data.map(d => d.ttfb_ms) },
      { ...trendOption.value.series[2], data: stats.trend_data.map(d => d.page_ms) },
    ],
  }
  // 饼图
  const counts: Record<string, number> = { website: 0, video: 0, download: 0 }
  stats.recent_tasks.forEach(t => { if (counts[t.task_type] !== undefined) counts[t.task_type]++ })
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
  recentTasks.value = stats.recent_tasks
}

function handleWsMessage(msg: ProgressMessage) {
  if (msg.type === 'task_completed' || msg.type === 'task_failed' || msg.type === 'progress_update') {
    taskStore.refreshDashboard()
  }
}

async function handleCreateTask() {
  const urls = urlInput.value.split('\n').map(u => u.trim()).filter(u => u)
  if (!urls.length) { message.warning('请输入至少一个URL'); return }
  creating.value = true
  try {
    const result = await taskStore.createTask(taskType.value, urls)
    message.success('任务创建成功')
    showCreateModal.value = false
    urlInput.value = ''
    router.push(`/task/${result.task_id}`)
  } catch (e: any) {
    message.error(e.message || '创建失败')
  } finally { creating.value = false }
}

function usePreset(key: string) {
  const urls = presetUrls[key]
  if (urls) {
    taskType.value = key === '下载测试' ? 'download' : key === '视频网站' ? 'video' : 'website'
    urlInput.value = urls.join('\n')
  }
}

watch(() => taskStore.dashboardStats, (stats) => {
  if (stats) updateCharts(stats)
})

onMounted(() => {
  taskStore.connectWs()
  taskStore.refreshDashboard()
  unsubWs = ws.onMessage(handleWsMessage)
})

onUnmounted(() => {
  if (unsubWs) unsubWs()
})
</script>

<template>
  <div class="dashboard">
    <div class="dashboard-header">
      <h1 class="page-title">Dashboard</h1>
      <n-space>
        <n-button @click="showCreateModal = true" type="primary">创建测试任务</n-button>
      </n-space>
    </div>

    <!-- 统计卡片 -->
    <n-grid :cols="5" :x-gap="16" :y-gap="16" class="stats-grid">
      <n-gi><n-card hoverable><n-statistic label="今日测试" :value="taskStore.dashboardStats?.today_tests ?? 0" /></n-card></n-gi>
      <n-gi><n-card hoverable><n-statistic label="成功率" :value="(taskStore.dashboardStats?.success_rate ?? 0).toFixed(1) + '%'" /></n-card></n-gi>
      <n-gi><n-card hoverable><n-statistic label="平均 DNS" :value="formatMs(taskStore.dashboardStats?.avg_dns ?? 0)" /></n-card></n-gi>
      <n-gi><n-card hoverable><n-statistic label="平均 TTFB" :value="formatMs(taskStore.dashboardStats?.avg_ttfb ?? 0)" /></n-card></n-gi>
      <n-gi><n-card hoverable><n-statistic label="平均首页" :value="formatMs(taskStore.dashboardStats?.avg_page_time ?? 0)" /></n-card></n-gi>
    </n-grid>

    <!-- 图表区域 -->
    <n-grid :cols="2" :x-gap="16" :y-gap="16" class="charts-grid">
      <n-gi>
        <n-card title="性能趋势" size="small">
          <v-chart :option="trendOption" style="height:300px" autoresize />
        </n-card>
      </n-gi>
      <n-gi>
        <n-card title="任务分布" size="small">
          <v-chart :option="pieOption" style="height:300px" autoresize />
        </n-card>
      </n-gi>
    </n-grid>

    <!-- 快捷测试 -->
    <n-card title="快捷测试" class="preset-card">
      <n-space>
        <n-button v-for="(urls, key) in presetUrls" :key="key" @click="usePreset(key)" secondary>
          {{ key }} ({{ urls.length }}个)
        </n-button>
      </n-space>
    </n-card>

    <!-- 最近任务 -->
    <n-card title="最近测试" class="recent-card">
      <n-data-table
        :columns="[
          { title: 'ID', key: 'id', width: 90, render: (r: any) => r.id.substring(0,6)+'...' },
          { title: '类型', key: 'task_type', width: 80 },
          { title: '状态', key: 'status', width: 90, render: (r: any) => h('span', { class: `st st-${r.status}` }, r.status) },
          { title: '进度', key: 'progress', width: 70, render: (r: any) => (r.progress ?? 0).toFixed(0) + '%' },
          { title: '时间', key: 'created_at', render: (r: any) => formatTime(r.created_at) },
          { title: '操作', key: 'a', width: 120, render: (r: any) => h('a', { style: 'cursor:pointer;color:var(--primary-color)', onClick: () => router.push('/task/'+r.id) }, '详情') },
        ]"
        :data="recentTasks"
        :bordered="false"
        size="small"
        :pagination="{ pageSize: 10 }"
      />
    </n-card>

    <!-- 创建任务弹窗 -->
    <n-modal v-model:show="showCreateModal" preset="card" title="创建测试任务" style="width:640px">
      <n-form>
        <n-form-item label="测试类型">
          <n-select v-model:value="taskType" :options="taskTypeOptions" />
        </n-form-item>
        <n-form-item label="测试URL（每行一个）">
          <n-input v-model:value="urlInput" type="textarea" placeholder="https://www.example.com" :rows="6" />
        </n-form-item>
        <n-form-item>
          <n-space>
            <n-button v-for="(urls, key) in presetUrls" :key="key" size="small" @click="usePreset(key)">填入{{ key }}</n-button>
          </n-space>
        </n-form-item>
        <div class="modal-footer">
          <n-button @click="showCreateModal = false">取消</n-button>
          <n-button type="primary" :loading="creating" @click="handleCreateTask">创建</n-button>
        </div>
      </n-form>
    </n-modal>
  </div>
</template>

<script lang="ts">
export default { name: 'Dashboard' }
</script>

<style scoped>
.dashboard { padding: 8px 0; }
.dashboard-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.page-title { font-size: 24px; font-weight: 600; }
.stats-grid { margin-bottom: 16px; }
.charts-grid { margin-bottom: 16px; }
.preset-card { margin-bottom: 16px; }
.recent-card { margin-bottom: 16px; }
.modal-footer { display: flex; justify-content: flex-end; gap: 12px; margin-top: 8px; }
.st { padding: 1px 6px; border-radius: 3px; font-size: 11px; }
.st-completed { background: #18a058; color: white; }
.st-running { background: #2080f0; color: white; }
.st-pending { background: #909399; color: white; }
.st-failed { background: #d03050; color: white; }
.st-cancelled { background: #f0a020; color: white; }
</style>
