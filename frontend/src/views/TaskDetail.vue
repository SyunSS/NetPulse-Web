<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDialog, useMessage } from 'naive-ui'
import { taskApi, type TestTask, type WebsiteResult, type VideoResult, type DownloadResult, type PingResult } from '@/api/task'
import { getWsClient, type ProgressMessage } from '@/api/ws'
import { useAuthStore } from '@/stores/auth'
import { formatMs, formatFileSize, formatTime } from '@/utils'
import http from '@/api/index'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const authStore = useAuthStore()
const taskId = route.params.id as string

const task = ref<TestTask | null>(null)
const websiteResults = ref<WebsiteResult[]>([])
const videoResults = ref<VideoResult[]>([])
const downloadResults = ref<DownloadResult[]>([])
const pingResults = ref<PingResult[]>([])
const loading = ref(true)
const progress = ref(0)
const logs = ref<string[]>([])
const taskLogs = ref<LogEntry[]>([])
interface LogEntry { level: string; message: string; created_at: string }
const ws = getWsClient()

async function fetchLogs() {
  try { taskLogs.value = ((await http.get(`/task/${taskId}/logs`)).data || []) as LogEntry[] }
  catch (_) {}
}

function scrollToLogs() { document.querySelector('.log-section')?.scrollIntoView({behavior:'smooth'}) }

let unsubWs: (() => void) | null = null

const isVideoTask = computed(() => task.value?.task_type === 'video')
const isDownloadTask = computed(() => task.value?.task_type === 'download')
const isPingTask = computed(() => task.value?.task_type === 'ping')

async function fetchData() {
  loading.value = true
  try {
    const taskRes = await taskApi.get(taskId)
    task.value = taskRes.data
    progress.value = task.value.progress ?? 0

    if (isVideoTask.value) {
      videoResults.value = (await taskApi.getVideoResults(taskId)).data
    } else if (isDownloadTask.value) {
      downloadResults.value = (await taskApi.getDownloadResults(taskId)).data
    } else if (isPingTask.value) {
      pingResults.value = (await taskApi.getPingResults(taskId)).data
    } else {
      websiteResults.value = (await taskApi.getResults(taskId)).data
    }
  } catch (e: unknown) {
    message.error((e as Error).message || '加载失败')
  } finally {
    loading.value = false
  }
}

function handleWsMessage(msg: ProgressMessage) {
  if (msg.task_id !== taskId) return
  if (msg.type === 'progress_update') progress.value = msg.progress
  if (msg.type === 'log') logs.value.push(msg.message)
  if (['url_completed','task_completed','task_failed'].includes(msg.type)) fetchData()
}
    fetchLogs()

async function handleDelete(force?: boolean) {
  const msg = force ? '强制删除此任务及所有结果？此操作不可恢复。' : '确认删除此任务？'
  const dialog = useDialog()
  dialog.warning({
    title: '删除任务',
    content: msg,
    positiveText: '确认删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        const url = force ? `/task/${taskId}?force=true` : `/task/${taskId}`
        await http.delete(url)
        router.push('/')
      } catch (e: any) { message.error(e.message || '删除失败') }
    },
  })
}

async function handleExport(format: string) {
  try {
    const blob: Blob = await http.get(`/task/${taskId}/export`, {
      params: { format },
      responseType: 'blob',
    })
    const a = document.createElement('a')
    a.href = URL.createObjectURL(blob)
    const ext = format === 'xlsx' ? 'xlsx' : format === 'csv' ? 'csv' : 'json'
    a.download = `result_${taskId.substring(0, 8)}.${ext}`
    a.click()
    URL.revokeObjectURL(a.href)
    message.success('导出成功')
  } catch (e: any) { message.error(e.message || '导出失败') }
}

onMounted(() => {
  fetchLogs()
  fetchData()
  ws.connect(taskId)
  unsubWs = ws.onMessage(handleWsMessage)
})

onUnmounted(() => { if (unsubWs) unsubWs() })

const st = (s: string) => s === 'completed' ? '已完成' : s === 'running' ? '运行中' : s === 'failed' ? '失败' : s === 'pending' ? '等待' : s
const stClass = (s: string) => `st st-${s}`
</script>

<template>
  <div class="task-detail">
    <div class="detail-header">
      <div>
        <button class="back-btn" @click="router.push('/')">← 返回</button>
        <h1 class="page-title">任务详情</h1>
      </div>
      <div class="header-actions">
        <button v-if="task && (task.status==='pending'||task.status==='running')" class="btn warning" @click="taskApi.cancel(taskId).then(fetchData)">取消任务</button>
        <button v-if="task && task.status==='running'" class="btn danger" @click="handleDelete(true)">强制删除</button>
        <button v-if="task && ['completed','failed','cancelled'].includes(task.status)" class="btn primary" @click="taskApi.retry(taskId).then(r=>router.push('/task/'+r.data.task_id))">重新测试</button>
        <button v-if="task && !['pending','running'].includes(task.status)" class="btn danger" @click="handleDelete()">删除</button>
        <button class="btn" @click="scrollToLogs">日志</button>
        <div v-if="task?.status==='completed'" style="display:flex;gap:6px">
          <button class="btn" @click="handleExport('xlsx')">Excel</button>
          <button class="btn" @click="handleExport('csv')">CSV</button>
          <button class="btn" @click="handleExport('json')">JSON</button>
        </div>
      </div>
    </div>

    <!-- 加载中 -->
    <div v-if="loading" class="loading-box">加载中...</div>

    <!-- 任务信息 -->
    <div v-if="task" class="info-card">
      <div class="card-title">任务信息</div>
      <div class="info-grid">
        <div class="info-item"><span class="il">任务ID</span><code>{{ task.id.substring(0,8) }}...</code></div>
        <div class="info-item"><span class="il">类型</span>{{ task.task_type === 'website' ? '网站测试' : task.task_type === 'video' ? '视频测试' : task.task_type === 'download' ? '下载测试' : task.task_type === 'ping' ? 'Ping 测试' : task.task_type }}</div>
        <div class="info-item"><span class="il">状态</span><span :class="stClass(task.status)">{{ st(task.status) }}</span></div>
        <div class="info-item"><span class="il">创建</span>{{ formatTime(task.created_at) }}</div>
        <div class="info-item"><span class="il">开始</span>{{ task.started_at ? formatTime(task.started_at) : '-' }}</div>
        <div class="info-item"><span class="il">完成</span>{{ task.finished_at ? formatTime(task.finished_at) : '-' }}</div>
      </div>
      <div v-if="task.status==='running'||task.status==='pending'" class="progress-bar">
        <div class="progress-fill" :style="{width:progress+'%'}"></div>
        <span class="progress-text">{{ progress.toFixed(0) }}%</span>
      </div>
    </div>

    <!-- 实时日志 -->
    <div v-if="logs.length" class="card">
      <div class="card-title">实时日志</div>
      <div class="log-box"><div v-for="(l,i) in logs" :key="i" class="log-line">{{ l }}</div></div>
    </div>

    <!-- 网站测试结果 -->
    <div v-if="!isVideoTask && !isDownloadTask && !isPingTask && websiteResults.length" class="card">
      <div class="card-title">网站测试结果 ({{ websiteResults.length }} 条)</div>
      <div class="table-wrap">
        <table class="dt">
          <thead><tr>
            <th>URL</th><th>DNS解析时延(ms)</th><th>DNS解析成功率(%)</th><th>TCP连接时延(ms)</th><th>访问成功率(%)</th><th>首包时延(ms)</th><th>首屏时延(ms)</th><th>首页时延(ms)</th><th>最大内容绘制(ms)</th><th>总请求</th><th>总大小(KB)</th><th>HTML(KB)</th><th>CSS(KB)</th><th>JS(KB)</th><th>图片(KB)</th>
          </tr></thead>
          <tbody>
            <tr v-for="r in websiteResults" :key="r.id">
              <td class="url-cell">{{ r.url }}</td>
              <td>{{ r.dns_time_ms?.toFixed(1) ?? '-' }}</td>
              <td>{{ (r.dns_success ?? null) !== null ? r.dns_success + '%' : '-' }}</td>
              <td>{{ r.tcp_time_ms?.toFixed(1) ?? '-' }}</td>
              <td>{{ r.error_msg ? '0' : '100' }}</td>
              <td>{{ r.ttfb_ms?.toFixed(1) ?? '-' }}</td>
              <td>{{ r.fcp_ms?.toFixed(0) ?? '-' }}</td>
              <td>{{ r.load_event_ms?.toFixed(0) ?? '-' }}</td>
              <td>{{ r.lcp_ms?.toFixed(0) ?? '-' }}</td>
              <td>{{ r.total_requests ?? '-' }}</td>
              <td>{{ r.site_size_kb?.toFixed(1) ?? '-' }}</td>
              <td>{{ r.html_size ? (r.html_size / 1024).toFixed(1) : '-' }}</td>
              <td>{{ r.css_size ? (r.css_size / 1024).toFixed(1) : '-' }}</td>
              <td>{{ r.js_size ? (r.js_size / 1024).toFixed(1) : '-' }}</td>
              <td>{{ r.image_size ? (r.image_size / 1024).toFixed(1) : '-' }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 视频测试结果 -->
    <div v-if="isVideoTask && videoResults.length" class="card">
      <div class="card-title">视频测试结果 ({{ videoResults.length }} 条)</div>
      <div class="table-wrap">
        <table class="dt">
          <thead><tr>
            <th>URL</th><th>平台</th><th>方式</th><th>DNS(ms)</th><th>TCP(ms)</th><th>首帧(S)</th><th>卡顿次数</th><th>卡顿占比(%)</th><th>播放时长(S)</th><th>分辨率</th><th>丢帧</th><th>状态</th>
          </tr></thead>
          <tbody>
            <tr v-for="r in videoResults" :key="r.id">
              <td class="url-cell">{{ r.url }}</td>
              <td>{{ r.platform || '-' }}</td>
              <td>{{ r.trigger_method || '-' }}</td>
              <td>{{ r.dns_time_ms?.toFixed(0) ?? '-' }}</td>
              <td>{{ r.tcp_time_ms?.toFixed(0) ?? '-' }}</td>
              <td>{{ r.first_play_time_ms ? (r.first_play_time_ms / 1000).toFixed(2) : '-' }}</td>
              <td>{{ r.stutter_count ?? '-' }}</td>
              <td>{{ r.stutter_ratio?.toFixed(1) ?? '-' }}</td>
              <td>{{ r.play_duration_sec?.toFixed(0) ?? '-' }}</td>
              <td>{{ r.video_width ? r.video_width+'×'+r.video_height : '-' }}</td>
              <td>{{ r.dropped_frames ?? '-' }}</td>
              <td><span :class="r.play_success===1 ? 'badge ok' : 'badge err'">{{ r.play_success===1 ? '成功' : '待确认' }}</span></td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 下载测试结果 -->
    <div v-if="isDownloadTask && downloadResults.length" class="card">
      <div class="card-title">下载测试结果 ({{ downloadResults.length }} 条)</div>
      <div class="table-wrap">
        <table class="dt">
          <thead><tr>
            <th>URL</th><th>文件大小(MB)</th><th>文件DNS时延(ms)</th><th>文件DNS解析成功率(%)</th><th>文件TCP连接时延(ms)</th><th>文件下载速率(Mbps)</th><th>文件下载成功率(%)</th>
          </tr></thead>
          <tbody>
            <tr v-for="r in downloadResults" :key="r.id">
              <td class="url-cell">{{ r.url }}</td>
              <td>{{ r.file_size ? (r.file_size / 1048576).toFixed(2) : '-' }}</td>
              <td>{{ r.dns_time_ms?.toFixed(1) ?? '-' }}</td>
              <td>{{ (r.dns_success ?? null) !== null ? r.dns_success + '%' : '-' }}</td>
              <td>{{ r.tcp_time_ms?.toFixed(1) ?? '-' }}</td>
              <td>{{ r.download_speed ? (r.download_speed / 125).toFixed(2) : '-' }}</td>
              <td>{{ r.success === 1 ? '100' : '0' }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Ping 测试结果 -->
    <div v-if="isPingTask && pingResults.length" class="card">
      <div class="card-title">Ping 测试结果 ({{ pingResults.length }} 条)</div>
      <div class="table-wrap">
        <table class="dt">
          <thead><tr>
            <th>目标</th><th>检测方式</th><th>时延(ms)</th><th>丢包率(%)</th><th>抖动时延(ms)</th><th>状态</th>
          </tr></thead>
          <tbody>
            <tr v-for="r in pingResults" :key="r.id">
              <td class="url-cell">{{ r.host }}</td>
              <td>{{ r.method || 'ICMP' }}</td>
              <td>{{ r.avg_latency_ms?.toFixed(1) ?? '-' }}</td>
              <td>{{ r.packet_loss_rate?.toFixed(1) ?? '-' }}%</td>
              <td>{{ r.jitter_ms?.toFixed(1) ?? '-' }}</td>
              <td><span :class="r.success===100 ? 'badge ok' : 'badge err'">{{ r.success===100 ? '成功' : '失败' }}</span></td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 空状态 -->
    <div v-if="!loading && ((!isVideoTask&&!isDownloadTask&&!isPingTask&&!websiteResults.length)||(isVideoTask&&!videoResults.length)||(isDownloadTask&&!downloadResults.length)||(isPingTask&&!pingResults.length)) && task?.status==='completed'" class="card empty">
      <div class="empty-icon">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>
      </div><h3>暂无测试结果</h3><p>任务已完成但未返回数据</p>
    </div>

    <!-- 运行日志 -->
    <div class="card log-section">
      <div class="card-title">运行日志 {{ taskLogs.length ? '('+taskLogs.length+')' : '' }}</div>
      <div v-if="!taskLogs.length" class="log-empty">暂无日志</div>
      <div v-else class="log-list">
        <div v-for="(log, i) in taskLogs" :key="i" class="log-line" :class="'log-'+log.level">
          <span class="log-time">{{ log.created_at?.substring(11,19) || '-' }}</span>
          <span class="log-level">{{ log.level }}</span>
          <span class="log-msg">{{ log.message }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.task-detail { max-width:1200px; margin:0 auto; }
.detail-header { display:flex; justify-content:space-between; align-items:flex-start; margin-bottom:20px; }
.page-title { font-size:22px; font-weight:700; margin:8px 0 0; }
.header-actions { display:flex; gap:8px; flex-wrap:wrap; }

.back-btn { background:none; border:1px solid var(--border-color); color:var(--text-secondary); padding:8px 14px; border-radius:var(--radius-sm); cursor:pointer; font-size:13px; }
.back-btn:hover { background:var(--bg-hover); color:var(--text-primary); }
.btn { height:34px; padding:0 14px; border:1px solid var(--border-color); background:var(--bg-card); color:var(--text-primary); border-radius:var(--radius-sm); font-size:13px; cursor:pointer; font-weight:500; white-space:nowrap; }
.btn:hover { background:var(--bg-hover); }
.btn.primary { background:var(--color-primary); color:white; border:none; }
.btn.primary:hover { background:var(--color-primary-active); }
.btn.warning { background:var(--color-warning); color:white; border:none; }

.loading-box { text-align:center; padding:80px 20px; color:var(--text-tertiary); font-size:15px; }

.card, .info-card { background:var(--bg-card); border:1px solid var(--border-color); border-radius:var(--radius-lg); padding:20px; margin-bottom:16px; }
.card-title { font-size:15px; font-weight:600; margin-bottom:14px; color:var(--text-primary); }
.info-grid { display:grid; grid-template-columns:repeat(3,1fr); gap:8px; }
.info-item { padding:8px 12px; background:var(--bg-hover); border-radius:var(--radius-sm); display:flex; align-items:center; gap:8px; font-size:13px; }
.info-item code { font-size:11px; color:var(--color-primary); font-family:var(--font-mono); }
.il { color:var(--text-tertiary); min-width:32px; }

.progress-bar { margin-top:14px; height:6px; background:var(--bg-hover); border-radius:3px; overflow:hidden; position:relative; }
.progress-fill { height:100%; background:var(--color-primary); border-radius:3px; transition:width .3s; }
.progress-text { position:absolute; right:0; top:-18px; font-size:11px; color:var(--text-tertiary); }

.log-box { max-height:200px; overflow-y:auto; background:var(--bg-hover); padding:10px; border-radius:var(--radius-sm); font-family:var(--font-mono); font-size:12px; }
.log-line { color:var(--text-secondary); margin-bottom:2px; }

.table-wrap { overflow-x:auto; }
.dt { width:100%; border-collapse:collapse; font-size:12px; }
.dt th { background:var(--bg-alt); color:var(--text-primary); padding:8px 10px; text-align:left; font-weight:600; white-space:nowrap; border-bottom:1px solid var(--border-color); }
.dt td { padding:7px 10px; border-bottom:1px solid var(--border-color); white-space:nowrap; }
.dt tr:hover td { background:var(--bg-hover); }
.url-cell { max-width:220px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.title-cell { max-width:160px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }

.badge { padding:2px 8px; border-radius:var(--radius-pill); font-size:11px; font-weight:600; letter-spacing:0.125px; }
.badge.ok { background:rgba(26,174,57,0.12); color:var(--color-success); }
.badge.err { background:rgba(208,48,80,0.12); color:var(--color-danger); }

.st { padding:2px 8px; border-radius:var(--radius-pill); font-size:11px; font-weight:600; letter-spacing:0.125px; }
.st-completed { background:rgba(26,174,57,0.12); color:var(--color-success); }
.st-running { background:var(--color-primary-bg); color:var(--color-primary-text); }
.st-pending, .st-cancelled { background:rgba(163,158,152,0.15); color:var(--text-tertiary); }
.st-failed { background:rgba(208,48,80,0.12); color:var(--color-danger); }

.empty { text-align:center; padding:60px 20px; color:var(--text-secondary); }
.empty-icon { color:var(--text-tertiary); margin-bottom:12px; }
.empty h3 { font-size:16px; color:var(--text-primary); margin-bottom:4px; }
</style>
