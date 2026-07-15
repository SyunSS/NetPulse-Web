<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import { useRouter } from 'vue-router'
import { taskApi, type TestTask } from '@/api/task'
import { formatTime } from '@/utils'

const router = useRouter()

const tasks = ref<TestTask[]>([])
const total = ref(0)
const page = ref(1)
const size = ref(20)
const loading = ref(false)
const filterType = ref('all')
const filterStatus = ref('all')

async function fetchTasks() {
  loading.value = true
  try {
    const res = await taskApi.list(page.value, size.value)
    tasks.value = res.data.tasks
    total.value = res.data.total
  } catch (e: any) {
    console.error('加载任务列表失败:', e)
  } finally {
    loading.value = false
  }
}

function handlePageChange(p: number) {
  page.value = p
  fetchTasks()
}

function handleRetry(taskId: string) {
  router.push('/task/' + taskId)
}

const typeLabel = (t: string) => t === 'website' ? '网站测试' : t === 'video' ? '视频测试' : t === 'download' ? '下载测试' : t
const statusLabel = (s: string) => s === 'completed' ? '已完成' : s === 'running' ? '运行中' : s === 'pending' ? '等待' : s === 'failed' ? '失败' : s === 'cancelled' ? '取消' : s
const stClass = (s: string) => `st st-${s}`

const filteredTasks = () => {
  return tasks.value.filter(t => {
    if (filterType.value !== 'all' && t.task_type !== filterType.value) return false
    if (filterStatus.value !== 'all' && t.status !== filterStatus.value) return false
    return true
  })
}
</script>

<template>
  <div class="history">
    <div class="page-header">
      <h1 class="page-title">历史记录</h1>
      <button class="btn" @click="fetchTasks">↻ 刷新</button>
    </div>

    <!-- 筛选 -->
    <div class="filter-bar">
      <span class="filter-label">类型:</span>
      <select v-model="filterType" class="select">
        <option value="all">全部</option>
        <option value="website">网站测试</option>
        <option value="video">视频测试</option>
        <option value="download">下载测试</option>
      </select>
      <span class="filter-label">状态:</span>
      <select v-model="filterStatus" class="select">
        <option value="all">全部</option>
        <option value="completed">已完成</option>
        <option value="running">运行中</option>
        <option value="failed">失败</option>
        <option value="cancelled">已取消</option>
      </select>
      <span class="total-info">共 {{ total }} 条</span>
    </div>

    <!-- 加载 -->
    <div v-if="loading && tasks.length === 0" class="empty">⏳ 加载中...</div>

    <!-- 空 -->
    <div v-else-if="filteredTasks().length === 0" class="empty">
      <div class="empty-icon">📭</div>
      <h3>暂无记录</h3>
      <p>运行任务或计划后会在此显示</p>
      <button class="btn" @click="fetchTasks">↻ 刷新</button>
    </div>

    <!-- 列表 -->
    <div v-else class="table-card">
      <table class="dt">
        <thead><tr>
          <th>ID</th><th>类型</th><th>状态</th><th>进度</th><th>创建时间</th><th>操作</th>
        </tr></thead>
        <tbody>
          <tr v-for="t in filteredTasks()" :key="t.id">
            <td><code>{{ t.id.substring(0,8) }}...</code></td>
            <td>{{ typeLabel(t.task_type) }}</td>
            <td><span :class="stClass(t.status)">{{ statusLabel(t.status) }}</span></td>
            <td>{{ (t.progress ?? 0).toFixed(0) }}%</td>
            <td>{{ formatTime(t.created_at) }}</td>
            <td>
              <button class="link" @click="router.push('/task/'+t.id)">查看</button>
            </td>
          </tr>
        </tbody>
      </table>

      <!-- 分页 -->
      <div class="pagination" v-if="total > size">
        <button class="page-btn" :disabled="page<=1" @click="handlePageChange(page-1)">上一页</button>
        <span class="page-info">第 {{ page }} 页 / 共 {{ Math.ceil(total/size) }} 页</span>
        <button class="page-btn" :disabled="page>=Math.ceil(total/size)" @click="handlePageChange(page+1)">下一页</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.history { max-width: 1200px; margin: 0 auto; }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.page-title { font-size: 22px; font-weight: 700; margin: 0; }

.btn { height: 34px; padding: 0 14px; border: 1px solid var(--border-color); background: var(--bg-card); color: var(--text-primary); border-radius: var(--radius-sm); font-size: 13px; cursor: pointer; font-weight: 500; }
.btn:hover { background: var(--bg-hover); border-color: var(--border-color-hover); }

.filter-bar { display: flex; align-items: center; gap: 10px; margin-bottom: 16px; padding: 12px 16px; background: var(--bg-card); border: 1px solid var(--border-color); border-radius: var(--radius-md); }
.filter-label { font-size: 13px; color: var(--text-secondary); }
.select { height: 30px; padding: 0 8px; border: 1px solid var(--border-color); border-radius: var(--radius-sm); background: var(--bg-card); color: var(--text-primary); font-size: 13px; }
.total-info { margin-left: auto; font-size: 12px; color: var(--text-tertiary); }

.empty { text-align: center; padding: 80px 20px; color: var(--text-secondary); }
.empty-icon { font-size: 64px; margin-bottom: 16px; opacity: 0.5; }
.empty h3 { font-size: 18px; color: var(--text-primary); margin: 0 0 8px 0; }

.table-card { background: var(--bg-card); border: 1px solid var(--border-color); border-radius: var(--radius-lg); padding: 16px; }
.dt { width: 100%; border-collapse: collapse; font-size: 13px; }
.dt th { background: var(--gradient-primary); color: white; padding: 10px 12px; text-align: left; font-weight: 600; }
.dt td { padding: 8px 12px; border-bottom: 1px solid var(--border-color); }
.dt tr:hover td { background: var(--bg-hover); }
.dt code { font-size: 11px; color: var(--color-primary); font-family: var(--font-mono); }

.st { padding: 2px 8px; border-radius: 4px; font-size: 11px; font-weight: 500; }
.st-completed { background: rgba(24,160,88,.15); color: var(--color-success); }
.st-running { background: rgba(32,128,240,.15); color: var(--color-primary); }
.st-pending { background: rgba(156,163,175,.15); color: var(--text-secondary); }
.st-failed { background: rgba(208,48,80,.15); color: var(--color-danger); }
.st-cancelled { background: rgba(240,160,32,.15); color: var(--color-warning); }

.link { background: none; border: none; color: var(--color-primary); cursor: pointer; font-size: 13px; padding: 0; }
.link:hover { text-decoration: underline; }

.pagination { display: flex; align-items: center; justify-content: center; gap: 12px; padding-top: 16px; }
.page-btn { height: 32px; padding: 0 12px; border: 1px solid var(--border-color); background: var(--bg-card); color: var(--text-primary); border-radius: var(--radius-sm); font-size: 13px; cursor: pointer; }
.page-btn:hover:not(:disabled) { background: var(--bg-hover); }
.page-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.page-info { font-size: 13px; color: var(--text-secondary); }
</style>
