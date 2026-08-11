<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useDialog, useMessage } from 'naive-ui'
import { taskApi, type TestTask } from '@/api/task'
import http from '@/api/index'
import { getErrorMessage } from '@/api/index'
import { formatTime } from '@/utils'

const router = useRouter()
const dialog = useDialog()
const message = useMessage()

const tasks = ref<TestTask[]>([])
const total = ref(0)
const page = ref(1)
const size = ref(20)
const loading = ref(false)
const error = ref('')
const filterType = ref('all')
const filterStatus = ref('all')
const selectedIds = ref(new Set<string>())

const allSelected = computed(() =>
  filteredTasks().length > 0 && filteredTasks().every(t => selectedIds.value.has(t.id))
)

function toggleAll() {
  if (allSelected.value) { selectedIds.value = new Set() }
  else { selectedIds.value = new Set(filteredTasks().map(t => t.id)) }
}

function toggleOne(id: string) {
  const next = new Set(selectedIds.value)
  if (next.has(id)) { next.delete(id) } else { next.add(id) }
  selectedIds.value = next
}

async function fetchTasks() {
  loading.value = true
  error.value = ''
  try {
    const res = await taskApi.list(page.value, size.value)
    tasks.value = res.data.tasks
    total.value = res.data.total
    } catch (e: any) {
      error.value = getErrorMessage(e, '加载失败')
    }
  finally { loading.value = false }
}

function handlePageChange(p: number) { page.value = p; selectedIds.value = new Set(); fetchTasks() }

function handleFilterChange() {
  page.value = 1
  selectedIds.value = new Set()
  fetchTasks()
}

function typeLabel(t: string) {
  const m: Record<string,string> = { website:'网站', video:'视频', download:'下载', ping:'Ping' }
  return m[t] || t
}
function statusLabel(s: string) {
  const m: Record<string,string> = { pending:'等待中', running:'运行中', completed:'已完成', failed:'失败', cancelled:'已取消' }
  return m[s] || s
}
function stClass(s: string) { return `tag-${s}` }

function filteredTasks() {
  return tasks.value.filter(t => {
    if (filterType.value !== 'all' && t.task_type !== filterType.value) return false
    if (filterStatus.value !== 'all' && t.status !== filterStatus.value) return false
    return true
  })
}

async function handleDelete(taskId: string, force?: boolean) {
  const msg = force ? '强制删除此任务（包括运行中）？' : '确认删除此任务？'
  dialog.warning({
    title: '删除任务',
    content: msg,
    positiveText: '确认删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        const url = force ? `/task/${taskId}?force=true` : `/task/${taskId}`
        await http.delete(url)
        fetchTasks()
      } catch (e: unknown) { message.error(getErrorMessage(e, '删除失败')) }
    },
  })
}

async function handleCancel(taskId: string) {
  try { await taskApi.cancel(taskId); fetchTasks() }
  catch (e: unknown) { message.error(getErrorMessage(e, '取消失败')) }
}

async function handleBatchDelete() {
  const ids = Array.from(selectedIds.value)
  if (ids.length === 0) { message.warning('请先选择要删除的任务'); return }
  dialog.warning({
    title: '批量删除',
    content: `确认删除选中的 ${ids.length} 个任务？运行中的不会被删除。`,
    positiveText: '确认删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await http.post('/task/batch-delete', { task_ids: ids })
        selectedIds.value = new Set()
        fetchTasks()
      } catch (e: unknown) { message.error(getErrorMessage(e, '删除失败')) }
    },
  })
}

onMounted(() => { fetchTasks() })
</script>

<template>
  <div class="history">
    <div class="view-intro">
      <div><div class="section-eyebrow">task archive / retained signals</div><h1>历史记录</h1><p>检索每一次探测的状态、进度与结果。</p></div>
      <div class="archive-total"><b>{{ total }}</b><span>条任务</span></div>
    </div>
    <div class="page-header">
      <h1 class="page-title">历史记录</h1>
      <div class="header-actions">
        <select v-model="filterType" @change="handleFilterChange" class="filter-select">
          <option value="all">全部类型</option>
          <option value="website">网站</option><option value="video">视频</option>
          <option value="download">下载</option><option value="ping">Ping</option>
        </select>
        <select v-model="filterStatus" @change="handleFilterChange" class="filter-select">
          <option value="all">全部状态</option>
          <option value="completed">已完成</option><option value="failed">失败</option>
          <option value="running">运行中</option><option value="cancelled">已取消</option>
        </select>
         <button class="btn" @click="fetchTasks">刷新</button>
        <button v-if="selectedIds.size > 0" class="btn danger" @click="handleBatchDelete">
          删除选中 ({{ selectedIds.size }})
        </button>
      </div>
    </div>

     <div v-if="error" class="empty-text error-text">
       {{ error }} <button class="link" @click="fetchTasks">重试</button>
     </div>
     <table class="dt" v-else-if="filteredTasks().length">
      <thead><tr>
        <th><input type="checkbox" :checked="allSelected" @change="toggleAll" /></th>
        <th>ID</th><th>类型</th><th>状态</th><th>进度</th><th>创建时间</th><th>操作</th>
      </tr></thead>
      <tbody>
        <tr v-for="t in filteredTasks()" :key="t.id">
          <td><input type="checkbox" :checked="selectedIds.has(t.id)" @change="toggleOne(t.id)" /></td>
          <td><code>{{ t.id.substring(0,8) }}...</code></td>
          <td>{{ typeLabel(t.task_type) }}</td>
          <td><span :class="stClass(t.status)">{{ statusLabel(t.status) }}</span></td>
          <td>{{ (t.progress ?? 0).toFixed(0) }}%</td>
          <td>{{ formatTime(t.created_at) }}</td>
          <td class="action-col">
            <button class="link" @click="router.push('/task/'+t.id)">查看</button>
            <button v-if="t.status==='running'||t.status==='pending'" class="link" @click="handleCancel(t.id)">停止</button>
            <button v-if="t.status==='completed'||t.status==='failed'||t.status==='cancelled'" class="link danger" @click="handleDelete(t.id)">删除</button>
            <button v-if="t.status==='running'" class="link danger" @click="handleDelete(t.id, true)">强制删除</button>
          </td>
        </tr>
      </tbody>
    </table>
      <div v-else class="empty-text">
        {{ filterType !== 'all' || filterStatus !== 'all' ? '当前页没有符合筛选条件的记录' : '暂无记录' }}
      </div>
      <div v-if="filterType !== 'all' || filterStatus !== 'all'" class="filter-scope">筛选仅应用于当前页，任务列表接口暂不支持服务端筛选</div>
     <div v-if="total > size" class="pagination">
       <button class="btn" :disabled="page <= 1" @click="handlePageChange(page - 1)">上一页</button>
       <span>第 {{ page }} / {{ Math.ceil(total / size) }} 页</span>
       <button class="btn" :disabled="page >= Math.ceil(total / size)" @click="handlePageChange(page + 1)">下一页</button>
     </div>
  </div>
</template>

<style scoped>
.history { padding: 8px 0; }
.page-header { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 12px; margin-bottom: 20px; }
.page-title { font-size: 20px; font-weight: 700; margin: 0; }
.header-actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.filter-select { height: 32px; padding: 0 10px; border: 1px solid var(--border-color); border-radius: var(--radius-sm); background: var(--bg-card); color: var(--text-primary); font-size: 13px; }
.filter-scope { margin-top: 8px; text-align: center; color: var(--text-tertiary); font-size: 12px; }
.btn { height: 32px; padding: 0 12px; border: 1px solid var(--border-color); background: var(--bg-card); color: var(--text-primary); border-radius: var(--radius-sm); cursor: pointer; font-size: 13px; }
.btn.danger { background: var(--color-danger); color: white; border-color: var(--color-danger); }
.dt { width: 100%; border-collapse: collapse; font-size: 13px; }
.dt th, .dt td { padding: 8px 12px; border-bottom: 1px solid var(--border-color); text-align: left; }
.dt th { background: var(--bg-card); font-weight: 600; }
.action-col { white-space: nowrap; }
.action-col .link { margin-right: 8px; }
.link { background: none; border: none; color: var(--color-primary); cursor: pointer; font-size: 13px; padding: 0; }
.link.danger { color: var(--color-danger); }
.tag-completed { color: var(--color-success); background: rgba(26,174,57,0.12); padding: 2px 8px; border-radius: var(--radius-pill); font-size: 12px; font-weight: 600; letter-spacing: 0.125px; }
.tag-failed { color: var(--color-danger); background: rgba(208,48,80,0.12); padding: 2px 8px; border-radius: var(--radius-pill); font-size: 12px; font-weight: 600; letter-spacing: 0.125px; }
.tag-running { color: var(--color-primary-text); background: var(--color-primary-bg); padding: 2px 8px; border-radius: var(--radius-pill); font-size: 12px; font-weight: 600; letter-spacing: 0.125px; }
.tag-pending, .tag-cancelled { color: var(--text-tertiary); background: rgba(163,158,152,0.15); padding: 2px 8px; border-radius: var(--radius-pill); font-size: 12px; font-weight: 600; letter-spacing: 0.125px; }
.empty-text { text-align: center; padding: 40px; color: var(--text-tertiary); }
.error-text { color: var(--color-danger); }
.pagination { display: flex; justify-content: center; align-items: center; gap: 12px; margin-top: 16px; }
</style>

<style scoped>
.view-intro { display:flex; align-items:flex-end; justify-content:space-between; gap:20px; margin-bottom:25px; }.view-intro h1 { margin-top:8px; color:var(--text-primary); font-family:var(--font-display); font-size:clamp(30px,4vw,43px); font-weight:500; letter-spacing:-.06em; line-height:1; }.view-intro p { margin-top:10px; color:var(--text-secondary); font-size:13px; }.section-eyebrow { color:var(--color-primary); font-family:var(--font-mono); font-size:9px; letter-spacing:.14em; text-transform:uppercase; }.archive-total { display:flex; align-items:baseline; gap:8px; color:var(--text-tertiary); font-family:var(--font-mono); font-size:9px; }.archive-total b { color:var(--color-primary); font-size:26px; font-weight:500; letter-spacing:-.08em; }
.page-header { padding:10px; margin-bottom:12px; border:1px solid var(--border-color); border-radius:10px; background:rgba(15,32,50,.55); }.page-header .page-title { display:none; }.filter-select,.btn { height:34px; border-color:var(--border-color); border-radius:5px; background:var(--bg-input); color:var(--text-primary); font-family:var(--font-mono); font-size:10px; }.btn:hover { border-color:var(--border-color-hover); background:var(--bg-hover); }.btn.danger { border-color:var(--color-danger); background:var(--color-danger); color:#25120d; }.dt { overflow:hidden; border:1px solid var(--border-color); border-radius:10px; background:var(--bg-card); }.dt th,.dt td { padding:13px 14px; border-bottom-color:var(--border-color); }.dt th { background:var(--bg-elevated); color:var(--text-tertiary); font-family:var(--font-mono); font-size:9px; font-weight:500; letter-spacing:.05em; text-transform:uppercase; }.dt td { color:var(--text-secondary); font-size:12px; }.dt tbody tr { transition:background var(--transition-fast); }.dt tbody tr:hover td { background:var(--bg-hover); }.dt td code { padding:3px 5px; border:1px solid var(--border-color); border-radius:3px; background:var(--bg-alt); font-size:10px; }.action-col .link { color:var(--color-primary); font-family:var(--font-mono); font-size:10px; }.action-col .link.danger { color:var(--color-danger); }.empty-text { border:1px dashed var(--border-color); border-radius:10px; color:var(--text-tertiary); }.filter-scope { font-family:var(--font-mono); }.pagination { font-family:var(--font-mono); font-size:10px; }
@media (max-width:650px) { .view-intro { align-items:flex-start; flex-direction:column; }.page-header { align-items:stretch; }.header-actions { width:100%; }.filter-select,.header-actions .btn { flex:1; min-width:0; }.dt { display:block; overflow-x:auto; white-space:nowrap; } }
</style>

<style scoped>
.page-header { background:var(--panel-soft); }
@media (max-width:760px) { .history { padding:0; }.view-intro { margin-bottom:18px; }.page-header { align-items:stretch; flex-direction:column; }.header-actions { display:grid; grid-template-columns:1fr 1fr; }.header-actions .btn:last-child:nth-child(5) { grid-column:1 / -1; }.dt { display:block; max-width:100%; overflow-x:auto; border-radius:9px; }.dt th,.dt td { padding:11px 12px; }.empty-text { padding:28px 16px; } }
</style>
