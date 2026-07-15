<script setup lang="ts">
import { ref, onMounted, h } from 'vue'
import { useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { taskApi, type TestTask } from '@/api/task'
import { formatTime } from '@/utils'

const router = useRouter()
const message = useMessage()

const tasks = ref<TestTask[]>([])
const total = ref(0)
const page = ref(1)
const size = ref(20)
const loading = ref(false)

async function fetchTasks() {
  loading.value = true
  try {
    const res = await taskApi.list(page.value, size.value)
    tasks.value = res.data.tasks
    total.value = res.data.total
  } catch (e: unknown) {
    const err = e as Error
    message.error(err.message || '加载失败')
  } finally {
    loading.value = false
  }
}

function handlePageChange(p: number) {
  page.value = p
  fetchTasks()
}

async function handleRetry(taskId: string) {
  try {
    const res = await taskApi.retry(taskId)
    message.success('任务已重新创建')
    router.push(`/task/${res.data.task_id}`)
  } catch (e: unknown) {
    const err = e as Error
    message.error(err.message)
  }
}

onMounted(() => {
  fetchTasks()
})
</script>

<template>
  <div class="history">
    <h1 class="page-title">历史记录</h1>

    <n-card>
      <n-spin :show="loading">
        <n-data-table
          :columns="[
            { title: '任务ID', key: 'id', width: 120, render: (row: TestTask) => row.id.substring(0, 8) + '...' },
            { title: '类型', key: 'task_type', width: 100 },
            {
              title: '状态',
              key: 'status',
              width: 120,
              render: (row: TestTask) => h('span', {
                class: `status-tag status-${row.status}`
              }, row.status)
            },
            { title: '进度', key: 'progress', width: 80, render: (row: TestTask) => (row.progress ?? 0).toFixed(0) + '%' },
            { title: '创建时间', key: 'created_at', render: (row: TestTask) => formatTime(row.created_at) },
            {
              title: '操作',
              key: 'actions',
              width: 200,
              render: (row: TestTask) => h('div', { style: 'display: flex; gap: 8px' }, [
                h('a', {
                  style: 'color: var(--primary-color); cursor: pointer;',
                  onClick: () => router.push(`/task/${row.id}`)
                }, '查看详情'),
                h('a', {
                  style: 'color: var(--primary-color); cursor: pointer;',
                  onClick: () => handleRetry(row.id)
                }, '重新测试')
              ])
            }
          ]"
          :data="tasks"
          :bordered="false"
          size="small"
          :pagination="{
            page: page,
            pageSize: size,
            itemCount: total,
            onChange: handlePageChange,
            showSizePicker: false
          }"
        />
      </n-spin>
    </n-card>
  </div>
</template>

<style scoped>
.history {
  padding: 8px 0;
}
.page-title {
  font-size: 24px;
  font-weight: 600;
  margin-bottom: 24px;
}
.status-tag {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 12px;
}
.status-completed { background: #18a058; color: white; }
.status-running { background: #2080f0; color: white; }
.status-pending { background: #909399; color: white; }
.status-failed { background: #d03050; color: white; }
.status-cancelled { background: #f0a020; color: white; }
</style>
