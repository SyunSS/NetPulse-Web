<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { usePlanStore } from '@/stores/plan'
import type { PlanItemInput } from '@/api/plan'
import MetricSelector from '@/components/MetricSelector.vue'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const planStore = usePlanStore()

const isEdit = computed(() => route.name === 'PlanEdit')
const planId = computed(() => route.params.id as string | undefined)

// 表单状态
const name = ref('')
const description = ref('')
const cronMode = ref('preset')  // preset | custom
const cronPreset = ref('0 9 * * *')  // 默认每天9点
const cronCustom = ref('0 0 * * *')
const enabled = ref(true)
const items = ref<PlanItemInput[]>([])
const saving = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)

// 文件批量导入
function handleFileImport(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file) return

  const reader = new FileReader()
  reader.onload = (e) => {
    const text = e.target?.result as string
    if (!text) return
    parseAndImport(text, file.name)
  }
  reader.readAsText(file)
  // reset input
  ;(event.target as HTMLInputElement).value = ''
}

function parseAndImport(text: string, filename: string) {
  const lines = text.split('\n').map(l => l.trim()).filter(l => l && !l.startsWith('//') && !l.startsWith('#'))
  if (lines.length === 0) { message.warning('文件中没有找到 URL'); return }

  // 自动分组：按 task_type 标题行分组
  const groups: Record<string, string[]> = { website: [], video: [], download: [], ping: [] }
  let currentType = 'website'

  for (const line of lines) {
    if (line.startsWith('[website]') || line === '网站测试') { currentType = 'website'; continue }
    if (line.startsWith('[video]') || line === '视频测试') { currentType = 'video'; continue }
    if (line.startsWith('[download]') || line === '下载测试') { currentType = 'download'; continue }
    if (line.startsWith('[ping]') || line === 'Ping测试' || line === 'ping') { currentType = 'ping'; continue }
    // URL 匹配：ping 也接受纯域名/IP，其他类型只接受 http(s)
    if (currentType === 'ping') {
      if (line.startsWith('http://') || line.startsWith('https://') || /^[\w.-]+(\.[\w.-]+)+(:\d+)?/.test(line)) {
        groups.ping.push(line)
      }
    } else {
      if (line.startsWith('http://') || line.startsWith('https://')) {
        groups[currentType].push(line)
      }
    }
  }

  // 如果没有任何类型标签，全部归为 website
  const allUrls = lines.filter(l => l.startsWith('http://') || l.startsWith('https://'))
  if (allUrls.length > 0 && !Object.values(groups).some(g => g.length > 0)) {
    groups.website = allUrls
  }

  // 为每个有数据的类型创建/追加 item
  let added = 0
  for (const [type, urls] of Object.entries(groups)) {
    if (urls.length === 0) continue
    // 查找现有同类型 item
    const existing = items.value.find(it => it.task_type === type)
    if (existing) {
      existing.urls = [...new Set([...existing.urls.filter(u => u.trim()), ...urls])]
    } else {
      items.value.push({ task_type: type, urls, options: {} })
    }
    added += urls.length
  }

  message.success(`已从 ${filename} 导入 ${added} 个 URL（${added} 项）`)
}

const taskTypeOptions = [
  { value: 'website', label: '网站测试', color: 'var(--color-primary)' },
  { value: 'video', label: '视频测试', color: 'var(--color-warning)' },
  { value: 'download', label: '下载测试', color: 'var(--color-success)' },
  { value: 'ping', label: 'Ping 测试', color: 'var(--color-info)' },
]

const cronPresets = [
  { value: '', label: '不定时（仅手动）' },
  { value: '0 9 * * *', label: '每天 9:00' },
  { value: '0 */2 * * *', label: '每 2 小时' },
  { value: '0 */6 * * *', label: '每 6 小时' },
  { value: '0 0 * * *', label: '每天 0:00' },
  { value: '0 0 * * 1', label: '每周一 0:00' },
  { value: '0 0 1 * *', label: '每月 1 号 0:00' },
  { value: 'custom', label: '自定义...' },
]

const currentCron = computed(() => {
  if (cronMode.value === 'preset' && cronPreset.value !== 'custom') {
    return cronPreset.value || null
  }
  return cronCustom.value || null
})

const nextRunPreview = ref<string | null>(null)

function downloadPlanTemplate() {
  const template = `\
// ═══════════════════════════════════════════════════
// NetPulse 测试计划导入模板 v1.1
// ═══════════════════════════════════════════════════
// 支持类型: [website] [video] [download] [ping]
// 每个 URL 一行, ping 支持纯域名或 IP:端口
//
// 选项说明 (手动添加时在编辑页面设置):
//   repeat_count — 重复测试次数, >1 取平均值 (默认1)
//   metrics      — website 采集指标, 可选:
//       dns_time / tcp_time / tls_time / ttfb / http_status
//       fcp / dom_load / load_time
//       total_size / html_size / css_size / js_size / img_size / requests
//       lcp / cls / tti (高级性能, 较耗时)
// ═══════════════════════════════════════════════════

[website]
https://www.baidu.com
https://github.com

[video]
https://www.bilibili.com/video/BV1GJ411x7h7
https://www.youtube.com/watch?v=dQw4w9WgXcQ

[download]
http://speedtest.tele2.net/1MB.zip
https://d.zhipin.com/boss/boss/13.141/64/boss_13.141_c0.apk

[ping]
lobby-prod-b.df.qq.com
receiver.tdm.qq.com
1.1.1.1:443
8.8.8.8
`
  const blob = new Blob([template], { type: 'text/plain' })
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = 'netpulse-plan-template.txt'
  a.click()
  URL.revokeObjectURL(a.href)
  message.success('模板已下载: netpulse-plan-template.txt')
}

function addItem() {
  items.value.push({
    task_type: 'website',
    urls: [],
    options: { metrics: ['dns_time','tcp_time','tls_time','ttfb','http_status','fcp','dom_load','load_time','total_size'] },
    repeat_count: 1,
    engine: 'headless_chrome',
  })
}

function removeItem(index: number) {
  items.value.splice(index, 1)
}

function getItemMetrics(idx: number): string[] {
  const opts = items.value[idx]?.options
  if (typeof opts === 'object' && opts && !Array.isArray(opts)) {
    return (opts as any).metrics || []
  }
  return []
}
function setItemMetrics(idx: number, metrics: string[]) {
  if (!items.value[idx]) return
  const opts = items.value[idx].options
  if (typeof opts === 'object' && opts && !Array.isArray(opts)) {
    (opts as any).metrics = metrics
  }
}

function addUrlToItem(index: number) {
  const urls = items.value[index].urls
  if (urls.length === 0 || urls[urls.length - 1].trim()) {
    urls.push('')
  }
}

function removeUrlFromItem(itemIdx: number, urlIdx: number) {
  items.value[itemIdx].urls.splice(urlIdx, 1)
}

// 简单的 cron 下次执行时间预览（前端近似计算）
function computeNextRunLocal(cronExpr: string): string {
  // 后端会计算准确的时间，这里只做 UI 提示
  // 使用 cron 表达式简单解析
  const parts = cronExpr.split(' ')
  if (parts.length !== 5) return '请检查 cron 格式'
  const [minute, hour] = parts
  if (minute === '0' && hour !== '*' && hour !== '*') {
    return `每天 ${hour}:00`
  }
  if (minute === '0' && hour === '*/2') return '每 2 小时'
  if (minute === '0' && hour === '*/6') return '每 6 小时'
  if (minute === '0' && hour === '0' && parts[4] === '1') return '每周一 0:00'
  if (minute === '0' && hour === '0' && parts[2] === '1') return '每月 1 号 0:00'
  return cronExpr
}

watch(currentCron, (val) => {
  if (val) {
    nextRunPreview.value = computeNextRunLocal(val)
  } else {
    nextRunPreview.value = null
  }
})

async function loadPlan() {
  if (!isEdit.value || !planId.value) return
  try {
    const plan = await planStore.fetchPlan(planId.value)
    name.value = plan.name
    description.value = plan.description || ''
    enabled.value = plan.enabled === 1

    if (plan.cron_expression) {
      const preset = cronPresets.find(p => p.value === plan.cron_expression)
      if (preset) {
        cronMode.value = 'preset'
        cronPreset.value = plan.cron_expression
      } else {
        cronMode.value = 'custom'
        cronCustom.value = plan.cron_expression
      }
    } else {
      cronMode.value = 'preset'
      cronPreset.value = ''
    }

    items.value = plan.items.map(it => ({
      task_type: it.task_type,
      urls: typeof it.urls === 'string' ? JSON.parse(it.urls) : it.urls,
      options: it.options ? (typeof it.options === 'string' ? JSON.parse(it.options) : it.options) : {},
      repeat_count: it.repeat_count || 1,
    }))
  } catch (e: any) {
    message.error(e.message || '加载失败')
    router.push('/plans')
  }
}

async function handleSave() {
  if (!name.value.trim()) {
    message.error('请输入计划名')
    return
  }
  if (items.value.length === 0) {
    message.error('请至少添加一个测试项')
    return
  }
  for (const it of items.value) {
    const validUrls = it.urls.filter(u => u.trim())
    if (validUrls.length === 0) {
      message.error('每个测试项至少需要一个 URL')
      return
    }
  }

  saving.value = true
  try {
    const data = {
      name: name.value,
      description: description.value || undefined,
      cron_expression: currentCron.value || undefined,
      enabled: enabled.value,
      items: items.value.map(it => ({
        task_type: it.task_type,
        urls: it.urls.filter(u => u.trim()),
        options: it.options || {},
        repeat_count: it.repeat_count || 1,
      })),
    }

    if (isEdit.value && planId.value) {
      await planStore.updatePlan(planId.value, data)
      message.success('已保存')
    } else {
      await planStore.createPlan(data)
      message.success('已创建')
    }
    router.push('/plans')
  } catch (e: any) {
    message.error(e.message || '保存失败')
  } finally {
    saving.value = false
  }
}

async function handleRunNow() {
  if (!isEdit.value || !planId.value) {
    message.warning('请先保存计划')
    return
  }
  try {
    const res = await planStore.runPlan(planId.value)
    message.success(`已启动 ${res.task_ids.length} 个任务`)
    router.push(`/plans/${planId.value}/runs`)
  } catch (e: any) {
    message.error(e.message || '启动失败')
  }
}

onMounted(() => {
  if (items.value.length === 0 && !isEdit.value) {
    addItem()
  }
  loadPlan()
})
</script>

<template>
  <div class="plan-edit-page">
    <div class="page-header">
      <button class="back-btn" @click="router.push('/plans')">← 返回</button>
      <h1 class="page-title">{{ isEdit ? '编辑计划' : '新建计划' }}</h1>
      <div class="header-actions">
        <button v-if="isEdit" class="action-btn primary" @click="handleRunNow">立即运行</button>
        <button class="action-btn primary" :disabled="saving" @click="handleSave">
          {{ saving ? '保存中...' : '保存计划' }}
        </button>
      </div>
    </div>

    <div class="form-container">
      <!-- 基本信息 -->
      <section class="form-section">
        <h2 class="section-title">计划信息</h2>
        <div class="form-row">
          <label class="form-label">计划名 <span class="required">*</span></label>
          <input v-model="name" type="text" class="form-input" placeholder="例如：每日首页检测" />
        </div>
        <div class="form-row">
          <label class="form-label">描述</label>
          <input v-model="description" type="text" class="form-input" placeholder="简单描述这个计划的用途" />
        </div>
        <div class="form-row">
          <label class="form-label">启用</label>
          <label class="switch">
            <input type="checkbox" v-model="enabled" />
            <span class="slider"></span>
          </label>
        </div>
      </section>

      <!-- 调度 -->
      <section class="form-section">
        <h2 class="section-title">调度</h2>
        <div class="cron-tabs">
          <div
            class="cron-tab"
            :class="{ active: cronMode === 'preset' }"
            @click="cronMode = 'preset'"
          >预设</div>
          <div
            class="cron-tab"
            :class="{ active: cronMode === 'custom' }"
            @click="cronMode = 'custom'"
          >自定义</div>
        </div>
        <div v-if="cronMode === 'preset'" class="cron-grid">
          <div
            v-for="p in cronPresets"
            :key="p.value"
            class="cron-preset"
            :class="{ active: cronPreset === p.value }"
            @click="cronPreset = p.value; if (p.value === 'custom') cronMode = 'custom'"
          >
            <div class="preset-dot"></div>
            <div class="preset-label">{{ p.label }}</div>
          </div>
        </div>
        <div v-else class="cron-custom">
          <div class="form-row">
            <label class="form-label">Cron 表达式 <span class="required">*</span></label>
            <input v-model="cronCustom" type="text" class="form-input cron-input" placeholder="0 9 * * *" />
          </div>
          <div class="cron-hint">
            <strong>格式：</strong> 分 时 日 月 周<br/>
            <code>0 9 * * *</code> = 每天 9:00 &nbsp;&nbsp;
            <code>*/5 * * * *</code> = 每 5 分钟 &nbsp;&nbsp;
            <code>0 0 * * 1</code> = 每周一 0:00
          </div>
        </div>
        <div v-if="nextRunPreview" class="next-run-preview">
          预计执行: <strong>{{ nextRunPreview }}</strong>
        </div>
      </section>

      <!-- 测试项 -->
      <section class="form-section">
        <div class="section-header">
          <h2 class="section-title">测试项 <span class="required">*</span></h2>
          <div style="display:flex;gap:8px">
            <input type="file" ref="fileInput" accept=".txt,.csv" style="display:none" @change="handleFileImport" />
            <button class="action-btn" @click="fileInput?.click()">导入文件</button>
            <button class="action-btn" @click="downloadPlanTemplate">下载模板</button>
            <button class="add-item-btn" @click="addItem">+ 添加测试项</button>
          </div>
        </div>

        <div v-if="items.length === 0" class="empty-items">
          <p>还没有测试项</p>
          <button class="add-item-btn" @click="addItem">+ 添加第一个测试项</button>
        </div>

        <div v-else class="items-list">
          <div
            v-for="(item, idx) in items"
            :key="idx"
            class="item-card"
          >
            <div class="item-header">
              <select v-model="item.task_type" class="type-select">
                <option v-for="opt in taskTypeOptions" :key="opt.value" :value="opt.value">
                  {{ opt.label }}
                </option>
              </select>
              <label class="repeat-label">重复次数:</label>
              <input v-model.number="item.repeat_count" type="number" min="1" max="100" class="repeat-input" />
              <label v-if="item.task_type==='ping'" class="repeat-label">发包数:</label>
              <input v-if="item.task_type==='ping'"
                :value="(item.options as any)?.ping_count ?? 10"
                @input="(e: any) => { const v = parseInt(e.target.value)||10; (item.options as any).ping_count = v }"
                type="number" min="1" max="100" class="repeat-input" />
              <button class="remove-btn" @click="removeItem(idx)" title="删除">
                <svg width="14" height="14" viewBox="0 0 14 14" fill="none"><path d="M2 4h10M5 4V2.5A.5.5 0 0 1 5.5 2h3a.5.5 0 0 1 .5.5V4m1 0v7.5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>
              </button>
            </div>
            <div class="urls-section">
              <label class="urls-label">URL 列表 <span class="required">*</span></label>
              <div v-for="(_url, urlIdx) in item.urls" :key="urlIdx" class="url-row">
                <input
                  v-model="item.urls[urlIdx]"
                  type="text"
                  class="form-input url-input"
                  placeholder="https://example.com"
                />
                <button class="url-remove" @click="removeUrlFromItem(idx, urlIdx)">×</button>
              </div>
              <button class="add-url-btn" @click="addUrlToItem(idx)">+ 添加 URL</button>
            </div>
            <div v-if="item.task_type === 'website'" style="margin-top:8px">
              <MetricSelector :model-value="getItemMetrics(idx)" @update:model-value="(v) => setItemMetrics(idx, v)" />
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.plan-edit-page {
  max-width: 900px;
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

.page-title {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  flex: 1;
  margin: 0;
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

.action-btn:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.action-btn.primary {
  background: var(--color-primary);
  color: white;
  border: none;
  font-weight: 600;
}

.action-btn.primary:hover:not(:disabled) {
  background: var(--color-primary-active);
}

.action-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.form-container {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.form-section {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  padding: 24px;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 16px;
}

.section-header .section-title {
  margin-bottom: 0;
}

.required {
  color: var(--color-danger);
}

.form-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.form-row:last-child {
  margin-bottom: 0;
}

.form-label {
  width: 100px;
  flex-shrink: 0;
  font-size: 13px;
  color: var(--text-secondary);
  font-weight: 500;
}

.form-input {
  flex: 1;
  height: 36px;
  padding: 0 12px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 14px;
  transition: all var(--transition-fast);
}

.form-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-bg);
}

/* 开关 */
.switch {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 24px;
  flex-shrink: 0;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: var(--border-color);
  border-radius: 24px;
  transition: var(--transition-fast);
}

.slider::before {
  content: '';
  position: absolute;
  height: 18px;
  width: 18px;
  left: 3px;
  bottom: 3px;
  background: white;
  border-radius: 50%;
  transition: var(--transition-fast);
}

.switch input:checked + .slider {
  background: var(--color-primary);
}

.switch input:checked + .slider::before {
  transform: translateX(20px);
}

/* Cron 标签 */
.cron-tabs {
  display: flex;
  gap: 4px;
  padding: 4px;
  background: var(--bg-body);
  border-radius: var(--radius-md);
  margin-bottom: 16px;
  width: fit-content;
}

.cron-tab {
  padding: 6px 16px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 13px;
  color: var(--text-secondary);
  transition: all var(--transition-fast);
}

.cron-tab.active {
  background: var(--bg-card);
  color: var(--text-primary);
  font-weight: 500;
  box-shadow: var(--shadow-sm);
}

.cron-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 8px;
}

.cron-preset {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  background: var(--bg-body);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.cron-preset:hover {
  border-color: var(--color-primary);
}

.cron-preset.active {
  background: var(--color-primary-bg);
  border-color: var(--color-primary);
}

.preset-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--color-primary);
  opacity: 0.3;
  flex-shrink: 0;
}

.cron-preset.active .preset-dot {
  opacity: 1;
}

.preset-label {
  font-size: 13px;
  color: var(--text-primary);
}

.cron-custom {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.cron-input {
  font-family: var(--font-mono);
  font-size: 16px;
  font-weight: 600;
  text-align: center;
  height: 44px;
}

.cron-hint {
  padding: 12px 16px;
  background: var(--bg-body);
  border-radius: var(--radius-md);
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.8;
}

.cron-hint code {
  background: var(--bg-card);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: var(--font-mono);
  color: var(--color-primary);
}

.next-run-preview {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
  padding: 10px 14px;
  background: var(--bg-hover);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 13px;
}

.next-run-preview strong {
  font-weight: 600;
  color: var(--color-success);
}

/* 测试项 */
.add-item-btn {
  height: 32px;
  padding: 0 14px;
  border: 1px dashed var(--border-color);
  background: transparent;
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: 13px;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.add-item-btn:hover {
  border-color: var(--color-primary);
  color: var(--color-primary);
  background: var(--color-primary-bg);
}

.empty-items {
  text-align: center;
  padding: 40px 20px;
  color: var(--text-secondary);
  border: 1px dashed var(--border-color);
  border-radius: var(--radius-md);
}

.items-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.item-card {
  background: var(--bg-body);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  padding: 14px;
}

.item-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.type-select {
  height: 32px;
  padding: 0 10px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
}

.remove-btn {
  margin-left: auto;
  width: 32px;
  height: 32px;
  border: 1px solid var(--border-color);
  background: var(--bg-card);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.remove-btn:hover {
  background: var(--color-danger);
  color: white;
  border-color: var(--color-danger);
}

.urls-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.urls-label {
  font-size: 12px;
  color: var(--text-secondary);
  font-weight: 500;
}

.url-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.url-input {
  flex: 1;
}

.url-remove {
  width: 32px;
  height: 32px;
  border: 1px solid var(--border-color);
  background: var(--bg-card);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 16px;
}

.url-remove:hover {
  background: var(--color-danger);
  color: white;
  border-color: var(--color-danger);
}

.add-url-btn {
  align-self: flex-start;
  height: 28px;
  padding: 0 12px;
  border: 1px dashed var(--border-color);
  background: transparent;
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: 12px;
  cursor: pointer;
}

.add-url-btn:hover {
  border-color: var(--color-primary);
  color: var(--color-primary);
}
</style>
