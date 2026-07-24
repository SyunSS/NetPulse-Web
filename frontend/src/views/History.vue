<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useDialog, useMessage } from 'naive-ui'
import { taskApi, type TestTask } from '@/api/task'
import http from '@/api/index'
import { formatTime } from '@/utils'

const router = useRouter()
const dialog = useDialog()
const message = useMessage()

const tasks = ref<TestTask[]>([])
const total = ref(0)
const page = ref(1)
const size = ref(20)
const loading = ref(false)
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
  try {
    const res = await taskApi.list(page.value, size.value)
    tasks.value = res.data.tasks
    total.value = res.data.total
    } catch (e: any) { if (import.meta.env.DEV) console.error(e) }
  finally { loading.value = false }
}

function handlePageChange(p: number) { page.value = p; fetchTasks() }

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
      } catch (e: any) { message.error(e.message || '删除失败') }
    },
  })
}

async function handleCancel(taskId: string) {
  try { await taskApi.cancel(taskId); fetchTasks() }
  catch (e: any) { message.error(e.message || '取消失败') }
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
      } catch (e: any) { message.error(e.message || '删除失败') }
    },
  })
}

onMounted(() => { fetchTasks() })
</script>

<template>
  <div class="history">
    <div class="page-header">
      <h1 class="page-title">历史记录</h1>
      <div class="header-actions">
        <select v-model="filterType" @change="fetchTasks" class="filter-select">
          <option value="all">全部类型</option>
          <option value="website">网站</option><option value="video">视频</option>
          <option value="download">下载</option><option value="ping">Ping</option>
        </select>
        <select v-model="filterStatus" @change="fetchTasks" class="filter-select">
          <option value="all">全部状态</option>
          <option value="completed">已完成</option><option value="failed">失败</option>
          <option value="running">运行中</option><option value="cancelled">已取消</option>
        </select>
        <button class="btn" @click="fetchTasks">↻ 刷新</button>
        <button v-if="selectedIds.size > 0" class="btn danger" @click="handleBatchDelete">
          删除选中 ({{ selectedIds.size }})
        </button>
      </div>
    </div>

    <table class="dt" v-if="filteredTasks().length">
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
    <div v-else class="empty-text">暂无记录</div>
  </div>
</template>

<style scoped>
.history { padding: 8px 0; }
.page-header { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 12px; margin-bottom: 20px; }
.page-title { font-size: 20px; font-weight: 700; margin: 0; }
.header-actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.filter-select { height: 32px; padding: 0 10px; border: 1px solid var(--border-color); border-radius: var(--radius-sm); background: var(--bg-card); color: var(--text-primary); font-size: 13px; }
.btn { height: 32px; padding: 0 12px; border: 1px solid var(--border-color); background: var(--bg-card); color: var(--text-primary); border-radius: var(--radius-sm); cursor: pointer; font-size: 13px; }
.btn.danger { background: var(--color-danger); color: white; border-color: var(--color-danger); }
.dt { width: 100%; border-collapse: collapse; font-size: 13px; }
.dt th, .dt td { padding: 8px 12px; border-bottom: 1px solid var(--border-color); text-align: left; }
.dt th { background: var(--bg-card); font-weight: 600; }
.action-col { white-space: nowrap; }
.action-col .link { margin-right: 8px; }
.link { background: none; border: none; color: var(--color-primary); cursor: pointer; font-size: 13px; padding: 0; }
.link.danger { color: var(--color-danger); }
.tag-completed { color: #16a34a; background: #dcfce7; padding: 2px 8px; border-radius: 10px; font-size: 12px; }
.tag-failed { color: #dc2626; background: #fee2e2; padding: 2px 8px; border-radius: 10px; font-size: 12px; }
.tag-running { color: #2563eb; background: #dbeafe; padding: 2px 8px; border-radius: 10px; font-size: 12px; }
.tag-pending, .tag-cancelled { color: #6b7280; background: #f3f4f6; padding: 2px 8px; border-radius: 10px; font-size: 12px; }
.empty-text { text-align: center; padding: 40px; color: var(--text-tertiary); }
</style>
