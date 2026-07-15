<script setup lang="ts">
import { ref, onMounted, onUnmounted, h, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { taskApi, type TestTask, type WebsiteResult, type VideoResult, type DownloadResult } from '@/api/task'
import { getWsClient, type ProgressMessage } from '@/api/ws'
import { formatMs, formatFileSize, formatTime } from '@/utils'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const taskId = route.params.id as string

const task = ref<TestTask | null>(null)
const websiteResults = ref<WebsiteResult[]>([])
const videoResults = ref<VideoResult[]>([])
const downloadResults = ref<DownloadResult[]>([])
const loading = ref(true)
const progress = ref(0)
const logs = ref<string[]>([])
const ws = getWsClient()

let unsubWs: (() => void) | null = null

const isVideoTask = computed(() => task.value?.task_type === 'video')
const isDownloadTask = computed(() => task.value?.task_type === 'download')

async function fetchData() {
  loading.value = true
  try {
    const taskRes = await taskApi.get(taskId)
    task.value = taskRes.data
    progress.value = task.value.progress ?? 0

    if (isVideoTask.value) {
      const resultRes = await taskApi.getVideoResults(taskId)
      videoResults.value = resultRes.data
    } else if (isDownloadTask.value) {
      const resultRes = await taskApi.getDownloadResults(taskId)
      downloadResults.value = resultRes.data
    } else {
      const resultRes = await taskApi.getResults(taskId)
      websiteResults.value = resultRes.data
    }
  } catch (e: unknown) {
    const err = e as Error
    message.error(err.message || '加载失败')
  } finally {
    loading.value = false
  }
}

function handleWsMessage(msg: ProgressMessage) {
  if (msg.task_id !== taskId) return
  switch (msg.type) {
    case 'progress_update':
      progress.value = msg.progress
      break
    case 'log':
      logs.value.push(msg.message)
      break
    case 'url_completed':
    case 'task_completed':
    case 'task_failed':
      fetchData()
      break
  }
}

async function handleCancel() {
  try {
    await taskApi.cancel(taskId)
    message.success('任务已取消')
    fetchData()
  } catch (e: unknown) {
    message.error((e as Error).message)
  }
}

async function handleRetry() {
  try {
    const res = await taskApi.retry(taskId)
    message.success('任务已重新创建')
    router.push(`/task/${res.data.task_id}`)
  } catch (e: unknown) {
    message.error((e as Error).message)
  }
}

function handleExport(format: string) {
  const url = `/api/task/${taskId}/export?format=${format}`
  window.open(url, '_blank')
}

onMounted(() => {
  fetchData()
  ws.connect(taskId)
  unsubWs = ws.onMessage(handleWsMessage)
})

onUnmounted(() => {
  if (unsubWs) unsubWs()
})
</script>

<template>
  <div class="task-detail">
    <div class="detail-header">
      <div>
        <n-button text @click="router.push('/')">← 返回</n-button>
        <h1 class="page-title">任务详情</h1>
      </div>
      <div class="header-actions">
        <n-button
          v-if="task && (task.status === 'pending' || task.status === 'running')"
          type="warning"
          @click="handleCancel"
        >取消任务</n-button>
        <n-button
          v-if="task && ['completed', 'failed', 'cancelled'].includes(task.status)"
          type="primary"
          @click="handleRetry"
        >重新测试</n-button>
        <n-dropdown
          v-if="task && task.status === 'completed'"
          trigger="click"
          :options="[
            { label: '导出 Excel (.xlsx)', key: 'xlsx' },
            { label: '导出 CSV', key: 'csv' },
            { label: '导出 JSON', key: 'json' },
          ]"
          @select="handleExport"
        >
          <n-button>导出报表 ▼</n-button>
        </n-dropdown>
      </div>
    </div>

    <n-spin :show="loading">
      <!-- 任务信息 -->
      <n-card v-if="task" title="任务信息" class="info-card">
        <n-descriptions :column="3" bordered size="small">
          <n-descriptions-item label="任务ID">{{ task.id.substring(0, 8) }}...</n-descriptions-item>
          <n-descriptions-item label="类型">{{ task.task_type }}</n-descriptions-item>
          <n-descriptions-item label="状态">
            <span :class="`status-tag status-${task.status}`">{{ task.status }}</span>
          </n-descriptions-item>
          <n-descriptions-item label="创建时间">{{ formatTime(task.created_at) }}</n-descriptions-item>
          <n-descriptions-item label="开始时间">{{ task.started_at ? formatTime(task.started_at) : '-' }}</n-descriptions-item>
          <n-descriptions-item label="完成时间">{{ task.finished_at ? formatTime(task.finished_at) : '-' }}</n-descriptions-item>
        </n-descriptions>
        <div v-if="task.status === 'running' || task.status === 'pending'" class="progress-section">
          <n-progress type="line" :percentage="progress" :status="task.status === 'running' ? 'info' : 'default'" />
        </div>
      </n-card>

      <!-- 实时日志 -->
      <n-card v-if="logs.length > 0" title="实时日志" class="logs-card">
        <div class="logs-container">
          <div v-for="(log, i) in logs" :key="i" class="log-line">{{ log }}</div>
        </div>
      </n-card>

      <!-- 网站测试结果 -->
      <n-card v-if="!isVideoTask && websiteResults.length > 0" title="网站测试结果" class="results-card">
        <n-data-table
          :columns="[
            { title: 'URL', key: 'url', width: 200, ellipsis: { tooltip: true } },
            { title: 'DNS(ms)', key: 'dns_time_ms', render: (r: WebsiteResult) => r.dns_time_ms?.toFixed(1) ?? '-' },
            { title: 'TCP(ms)', key: 'tcp_time_ms', render: (r: WebsiteResult) => r.tcp_time_ms?.toFixed(1) ?? '-' },
            { title: 'TLS(ms)', key: 'tls_time_ms', render: (r: WebsiteResult) => r.tls_time_ms?.toFixed(1) ?? '-' },
            { title: 'HTTP', key: 'http_status', render: (r: WebsiteResult) => r.http_status ?? '-' },
            { title: 'TTFB(ms)', key: 'ttfb_ms', render: (r: WebsiteResult) => r.ttfb_ms?.toFixed(1) ?? '-' },
            { title: '打开时间(ms)', key: 'page_open_time_ms', render: (r: WebsiteResult) => r.page_open_time_ms?.toFixed(0) ?? '-' },
            { title: '资源数', key: 'resource_count', render: (r: WebsiteResult) => r.resource_count ?? '-' },
            { title: '资源大小', key: 'resource_total_size', render: (r: WebsiteResult) => r.resource_total_size ? formatFileSize(r.resource_total_size) : '-' },
            { title: '状态', key: 'error_msg', render: (r: WebsiteResult) => r.error_msg ? h('span', { style: 'color: #d03050' }, '失败') : h('span', { style: 'color: #18a058' }, '成功') }
          ]"
          :data="websiteResults"
          :bordered="true"
          size="small"
          :scroll-x="1000"
        />
      </n-card>

      <!-- 视频测试结果 -->
      <n-card v-if="isVideoTask && videoResults.length > 0" title="视频测试结果" class="results-card">
        <n-data-table
          :columns="[
            { title: 'URL', key: 'url', width: 200, ellipsis: { tooltip: true } },
            { title: '平台', key: 'platform', render: (r: VideoResult) => r.platform ?? '-' },
            { title: '首次播放(ms)', key: 'first_play_time_ms', render: (r: VideoResult) => r.first_play_time_ms?.toFixed(0) ?? '-' },
            { title: '缓冲次数', key: 'buffer_count', render: (r: VideoResult) => r.buffer_count ?? '-' },
            { title: '缓冲时间(ms)', key: 'total_buffer_time_ms', render: (r: VideoResult) => r.total_buffer_time_ms?.toFixed(0) ?? '-' },
            { title: '时长(ms)', key: 'video_duration_ms', render: (r: VideoResult) => r.video_duration_ms?.toFixed(0) ?? '-' },
            { title: '下载速度(KB/s)', key: 'video_download_speed', render: (r: VideoResult) => r.video_download_speed?.toFixed(1) ?? '-' },
            { title: '视频大小', key: 'video_size', render: (r: VideoResult) => r.video_size ? formatFileSize(r.video_size) : '-' },
            { title: '丢帧', key: 'dropped_frames', render: (r: VideoResult) => r.dropped_frames ?? '-' },
            { title: '解码帧', key: 'decoded_frames', render: (r: VideoResult) => r.decoded_frames ?? '-' },
            { title: '状态', key: 'play_success', render: (r: VideoResult) => r.play_success === 1 ? h('span', { style: 'color: #18a058' }, '成功') : h('span', { style: 'color: #d03050' }, '失败') }
          ]"
          :data="videoResults"
          :bordered="true"
          size="small"
          :scroll-x="1100"
        />
      </n-card>

      <!-- 下载测试结果 -->
      <n-card v-if="isDownloadTask && downloadResults.length > 0" title="下载测试结果" class="results-card">
        <n-data-table
          :columns="[
            { title: 'URL', key: 'url', width: 250, ellipsis: { tooltip: true } },
            { title: '下载速度(KB/s)', key: 'download_speed', render: (r: DownloadResult) => r.download_speed?.toFixed(1) ?? '-' },
            { title: '平均速度(KB/s)', key: 'avg_speed', render: (r: DownloadResult) => r.avg_speed?.toFixed(1) ?? '-' },
            { title: '峰值速度(KB/s)', key: 'peak_speed', render: (r: DownloadResult) => r.peak_speed?.toFixed(1) ?? '-' },
            { title: '下载时间(ms)', key: 'download_time_ms', render: (r: DownloadResult) => r.download_time_ms?.toFixed(0) ?? '-' },
            { title: '文件大小', key: 'file_size', render: (r: DownloadResult) => r.file_size ? formatFileSize(r.file_size) : '-' },
            { title: '状态', key: 'success', render: (r: DownloadResult) => r.success === 1 ? h('span', { style: 'color: #18a058' }, '成功') : r.error_msg ? h('span', { style: 'color: #d03050' }, r.error_msg) : h('span', { style: 'color: #d03050' }, '失败') }
          ]"
          :data="downloadResults"
          :bordered="true"
          size="small"
        />
      </n-card>

      <!-- 空状态 -->
      <n-card v-if="!loading && ((isVideoTask && videoResults.length === 0) || (isDownloadTask && downloadResults.length === 0) || (!isVideoTask && !isDownloadTask && websiteResults.length === 0)) && task && task.status === 'completed'" class="empty-card">
        <n-empty description="暂无测试结果" />
      </n-card>
    </n-spin>
  </div>
</template>

<script lang="ts">
export default { name: 'TaskDetail' }
</script>

<style scoped>
.task-detail { padding: 8px 0; }
.detail-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 24px; }
.page-title { font-size: 24px; font-weight: 600; margin-top: 8px; }
.header-actions { display: flex; gap: 12px; }
.info-card, .logs-card, .results-card { margin-bottom: 16px; }
.progress-section { margin-top: 16px; }
.logs-container { max-height: 200px; overflow-y: auto; font-family: monospace; font-size: 13px; background: var(--n-color-target); padding: 12px; border-radius: 4px; }
.log-line { margin-bottom: 4px; color: var(--n-text-color-2); }
.status-tag { padding: 2px 8px; border-radius: 4px; font-size: 12px; }
.status-completed { background: #18a058; color: white; }
.status-running { background: #2080f0; color: white; }
.status-pending { background: #909399; color: white; }
.status-failed { background: #d03050; color: white; }
.status-cancelled { background: #f0a020; color: white; }
</style>
