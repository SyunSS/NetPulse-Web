<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  NButton, NCard, NInput, NTag, NGrid, NGi,
  NUpload, NUploadDragger, NSpace, NDivider,
  NText, NCode, NAlert, NModal, NSpin, NRadioGroup, NRadio, NInputNumber,
  useMessage, NIcon
} from 'naive-ui'
import { taskApi } from '@/api/task'
import { getErrorMessage } from '@/api/index'
import MetricSelector from '@/components/MetricSelector.vue'

const message = useMessage()

const loading = ref(false)
const templateData = ref<any>(null)
const showTemplate = ref(false)
const taskType = ref<'ping' | 'website' | 'download' | 'video'>('website')
const urlText = ref('')
const repeatCount = ref(1)
const pingCount = ref(10)
const fileContent = ref('')
const fileError = ref('')
const importResult = ref<{ created: number; failed: number; message: string; task_ids?: string[] } | null>(null)
const selectedMetrics = ref(['dns_time','tcp_time','tls_time','ttfb','http_status','fcp','dom_load','load_time','total_size'])

const typeOptions = [
  { label: 'Ping (连通性)', value: 'ping' },
  { label: 'Website (网站)', value: 'website' },
  { label: 'Download (下载)', value: 'download' },
  { label: 'Video (视频)', value: 'video' },
]

const urlList = computed(() => urlText.value.split('\n').map(s => s.trim()).filter(s => s))

async function loadTemplate() {
  loading.value = true
  try { templateData.value = (await taskApi.getTemplate()).data; showTemplate.value = true }
  catch { message.error('获取模板失败') }
  finally { loading.value = false }
}

async function downloadTemplateFile(): Promise<void> {
  if (!templateData.value) {
    await loadTemplate()
    if (!templateData.value) return
  }
  const batch = { tasks: templateData.value.examples.map((ex: any) => ({ task_type: ex.task_type, urls: ex.urls, options: ex.options || { repeat_count: 1 } })) }
  const blob = new Blob([JSON.stringify(batch, null, 2)], { type: 'application/json' })
  const a = document.createElement('a'); a.href = URL.createObjectURL(blob); a.download = 'netpulse-template.json'; a.click()
  URL.revokeObjectURL(a.href); message.success('模板已下载')
}

function fillTemplate() {
  if (!templateData.value) return
  const first = templateData.value.examples[0]
  taskType.value = first.task_type; urlText.value = first.urls.join('\n')
  if (first.options?.repeat_count) repeatCount.value = first.options.repeat_count
  showTemplate.value = false; message.info('已填入示例')
}

function handleFileChange(data: any) {
  const file = data.file?.file; if (!file) return
  const reader = new FileReader()
  reader.onload = (e) => {
    fileContent.value = e.target?.result as string || ''; fileError.value = ''
    try { JSON.parse(fileContent.value) } catch { fileError.value = 'JSON 格式无效' }
  }
  reader.readAsText(file)
}

async function doImport() {
  fileError.value = ''; importResult.value = null
  let parsed: any; try { parsed = JSON.parse(fileContent.value) } catch { fileError.value = 'JSON 无效'; return }
  const tasks = Array.isArray(parsed) ? parsed : parsed.tasks
  if (!Array.isArray(tasks)) { fileError.value = '需要 tasks 数组'; return }
  loading.value = true
  try {
    const res = await taskApi.importBatch({ tasks })
    importResult.value = res.data; message.success(res.data.message)
  } catch (e: unknown) { message.error(getErrorMessage(e, '导入失败')) }
  finally { loading.value = false }
}

async function doCreate() {
  loading.value = true
  try {
    const opts: any = { repeat_count: repeatCount.value }
    if (taskType.value === 'website') opts.metrics = [...selectedMetrics.value]
    if (taskType.value === 'ping') opts.ping_count = pingCount.value
    await taskApi.create({ task_type: taskType.value, urls: urlList.value, options: opts })
    message.success('任务已创建'); importResult.value = null
  } catch (e: unknown) { message.error(getErrorMessage(e, '创建失败')) }
  finally { loading.value = false }
}
</script>

<template>
  <div class="create-task">
    <div class="view-intro">
      <div><div class="section-eyebrow">new probe / manual or batch</div><h1>创建探测</h1><p>选择一个测试引擎，向网络发出一条可读的信号。</p></div>
      <div class="probe-status"><span></span><b>READY</b><small>engine pool available</small></div>
    </div>

    <n-card class="tpl-card">
      <div class="tpl-row">
        <div><h3>不知道格式？</h3><p>下载模板，含全部 4 种类型的 JSON 示例</p></div>
        <n-space><n-button ghost type="info" @click="loadTemplate">预览</n-button><n-button type="primary" @click="downloadTemplateFile">下载模板</n-button></n-space>
      </div>
      <div v-if="templateData" class="tpl-tags">
        <n-tag v-for="t in templateData.supported_types" :key="t" size="small" round type="info">{{ t }}</n-tag>
      </div>
      <n-modal v-model:show="showTemplate" preset="card" title="模板预览" style="max-width:900px;max-height:80vh;overflow:auto">
        <template v-if="templateData">
          <n-alert type="info" style="margin-bottom:16px">{{ templateData.description }}</n-alert>
          <div v-for="(ex, i) in templateData.examples" :key="i" style="margin-bottom:20px">
            <n-text strong>{{ ex.name }}</n-text>
            <n-tag size="small" style="margin-left:8px">{{ ex.task_type }}</n-tag>
            <n-code :code="JSON.stringify({task_type:ex.task_type,urls:ex.urls,options:ex.options},null,2)" language="json" />
          </div>
          <n-divider />
          <n-text strong>导入格式：</n-text>
          <n-code :code="JSON.stringify(templateData.batch_import_format.json_body,null,2)" language="json" />
          <n-space justify="end" style="margin-top:16px"><n-button type="primary" @click="fillTemplate">用此填入</n-button></n-space>
        </template>
        <n-spin v-else />
      </n-modal>
    </n-card>

    <n-divider />

    <n-grid :cols="2" :x-gap="24">
      <n-gi>
        <n-card title="手动输入" bordered>
          <n-space vertical size="medium">
            <div><n-text depth="3">类型</n-text><n-radio-group v-model:value="taskType"><n-radio v-for="o in typeOptions" :key="o.value" :value="o.value">{{ o.label }}</n-radio></n-radio-group></div>
            <div><n-text depth="3">重复次数 (>1 取平均)</n-text><n-input-number v-model:value="repeatCount" :min="1" :max="10" style="max-width:120px" /></div>
            <div v-if="taskType === 'ping'"><n-text depth="3">发包数 (-c)</n-text><n-input-number v-model:value="pingCount" :min="1" :max="100" style="max-width:120px" /></div>
            <div v-if="taskType === 'website'"><MetricSelector v-model="selectedMetrics" /></div>
            <div><n-text depth="3">URL（每行一个）</n-text><n-input v-model:value="urlText" type="textarea" placeholder="https://www.baidu.com&#10;https://github.com" :autosize="{ minRows: 5, maxRows: 12 }" /><n-text depth="3" style="font-size:12px">已输入 {{ urlList.length }} 个</n-text></div>
            <n-button type="primary" block :disabled="!urlList.length" :loading="loading" @click="doCreate">创建任务</n-button>
          </n-space>
        </n-card>
      </n-gi>

      <n-gi>
        <n-card title="批量导入" bordered>
          <n-space vertical size="medium">
            <n-upload :multiple="false" accept=".json" :show-file-list="false" @change="handleFileChange">
              <n-upload-dragger>
                <div style="text-align:center;padding:24px">
                   <n-icon size="36" color="var(--color-primary)"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg></n-icon>
                  <p style="margin:8px 0 4px;font-weight:500">拖拽 JSON 文件到此处</p>
                  <p style="margin:0;font-size:12px;color:var(--n-text-color-3)"><n-button text type="primary" @click.stop="downloadTemplateFile" size="tiny">下载模板</n-button> 获取示例</p>
                </div>
              </n-upload-dragger>
            </n-upload>
            <div v-if="fileContent"><n-text depth="3">预览:</n-text><n-code :code="fileContent.slice(0,2000)+(fileContent.length>2000?'\n...':'')" language="json" style="max-height:200px;overflow:auto" /><n-alert v-if="fileError" type="error">{{ fileError }}</n-alert></div>
            <n-button type="primary" block :disabled="!fileContent||!!fileError" :loading="loading" @click="doImport">导入并创建</n-button>
            <n-alert v-if="importResult" type="success">{{ importResult.message }}<br/><n-text depth="3" style="font-size:11px">ID: {{ importResult.task_ids?.slice(0,5).join(', ') }}{{ (importResult.task_ids?.length??0)>5?' ...':'' }}</n-text></n-alert>
          </n-space>
        </n-card>
      </n-gi>
    </n-grid>
  </div>
</template>

<style scoped>
.create-task { padding:8px 0; max-width:1200px }
.page-title { font-size:24px; font-weight:600; margin-bottom:24px }
.tpl-card { background: var(--bg-alt) }
.tpl-row { display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:16px }
.tpl-row h3 { margin:0 0 4px } .tpl-row p { margin:0; color:var(--text-tertiary); font-size:13px }
.tpl-tags { display:flex; gap:8px; margin-top:12px; flex-wrap:wrap }
</style>

<style scoped>
.view-intro { display:flex; align-items:flex-end; justify-content:space-between; gap:20px; margin-bottom:26px; }.section-eyebrow { color:var(--color-primary); font-family:var(--font-mono); font-size:9px; letter-spacing:.14em; text-transform:uppercase; }.view-intro h1 { margin-top:8px; color:var(--text-primary); font-family:var(--font-display); font-size:clamp(30px,4vw,43px); font-weight:500; letter-spacing:-.06em; line-height:1; }.view-intro p { margin-top:10px; color:var(--text-secondary); font-size:13px; }.probe-status { display:flex; align-items:center; gap:7px; padding-bottom:4px; color:var(--color-primary); font-family:var(--font-mono); font-size:10px; }.probe-status span { width:6px; height:6px; border-radius:50%; background:var(--color-primary); box-shadow:0 0 9px var(--color-primary); }.probe-status small { color:var(--text-tertiary); font-size:8px; }.create-task :deep(.n-card) { border-color:var(--border-color); border-radius:10px; background:linear-gradient(145deg,var(--bg-card),rgba(15,32,50,.75)); box-shadow:var(--shadow-card); }.create-task :deep(.n-card-header) { color:var(--text-primary); font-family:var(--font-display); }.create-task :deep(.n-card__content) { color:var(--text-secondary); }.create-task :deep(.n-divider) { border-color:var(--border-color); }.create-task :deep(.n-text) { color:var(--text-secondary); }.tpl-card { border-left:2px solid var(--color-primary) !important; background:linear-gradient(100deg,rgba(97,231,210,.1),var(--bg-card)) !important; }.tpl-row h3 { color:var(--text-primary); font-family:var(--font-display); font-weight:600; }.tpl-row p { color:var(--text-secondary); }.create-task :deep(.n-button) { font-family:var(--font-mono); font-size:10px; }.create-task :deep(.n-radio-group) { gap:8px; }.create-task :deep(.n-radio) { padding:8px 10px; border:1px solid var(--border-color); border-radius:5px; background:var(--bg-input); }.create-task :deep(.n-radio--checked) { border-color:var(--border-bright); background:var(--color-primary-bg); }.create-task :deep(.n-input),.create-task :deep(.n-input-number),.create-task :deep(.n-upload-trigger) { border-color:var(--border-color); }.create-task :deep(.n-input:hover),.create-task :deep(.n-input:focus-within) { border-color:var(--border-color-hover); }.create-task :deep(.n-upload-dragger) { border-color:var(--border-color); background:rgba(97,231,210,.025); }.create-task :deep(.n-upload-dragger:hover) { border-color:var(--color-primary); background:var(--color-primary-bg); }.create-task :deep(.n-code) { border:1px solid var(--border-color); background:var(--bg-alt); }.create-task :deep(.n-tag) { font-family:var(--font-mono); font-size:9px; }
@media (max-width:650px) { .view-intro { align-items:flex-start; flex-direction:column; }.probe-status { padding:0; }.create-task :deep(.n-grid) { --n-cols:1 !important; } }
</style>
