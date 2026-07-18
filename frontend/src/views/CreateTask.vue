<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  NButton, NCard, NInput, NTag, NGrid, NGi,
  NUpload, NUploadDragger, NSpace, NDivider,
  NText, NCode, NAlert, NModal, NSpin, NRadioGroup, NRadio, NInputNumber,
  useMessage, NIcon
} from 'naive-ui'
import { taskApi } from '@/api/task'

const message = useMessage()

const loading = ref(false)
const templateData = ref<any>(null)
const showTemplate = ref(false)
const taskType = ref<'ping' | 'website' | 'download' | 'video'>('website')
const urlText = ref('')
const repeatCount = ref(1)
const fileContent = ref('')
const fileError = ref('')
const importResult = ref<{ created: number; failed: number; message: string; task_ids?: string[] } | null>(null)

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

function downloadTemplateFile(): void {
  if (!templateData.value) { loadTemplate().then(() => downloadTemplateFile()); return }
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
  } catch (e: any) { message.error(e?.msg || '导入失败') }
  finally { loading.value = false }
}

async function doCreate() {
  loading.value = true
  try {
    const opts: any = { repeat_count: repeatCount.value }
    if (taskType.value === 'website') opts.metrics = ['basic', 'page', 'resource']
    await taskApi.create({ task_type: taskType.value, urls: urlList.value, options: opts })
    message.success('任务已创建'); importResult.value = null
  } catch (e: any) { message.error(e?.msg || '创建失败') }
  finally { loading.value = false }
}
</script>

<template>
  <div class="create-task">
    <h1 class="page-title">创建测试任务</h1>

    <n-card class="tpl-card">
      <div class="tpl-row">
        <div><h3>📋 不知道格式？</h3><p>下载模板，含全部 4 种类型的 JSON 示例</p></div>
        <n-space><n-button ghost type="info" @click="loadTemplate">👁 预览</n-button><n-button type="primary" @click="downloadTemplateFile">📥 下载模板</n-button></n-space>
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
        <n-card title="✍️ 手动输入" bordered>
          <n-space vertical size="medium">
            <div><n-text depth="3">类型</n-text><n-radio-group v-model:value="taskType"><n-radio v-for="o in typeOptions" :key="o.value" :value="o.value">{{ o.label }}</n-radio></n-radio-group></div>
            <div><n-text depth="3">重复次数 (>1 取平均)</n-text><n-input-number v-model:value="repeatCount" :min="1" :max="10" style="max-width:120px" /></div>
            <div><n-text depth="3">URL（每行一个）</n-text><n-input v-model:value="urlText" type="textarea" placeholder="https://www.baidu.com&#10;https://github.com" :autosize="{ minRows: 5, maxRows: 12 }" /><n-text depth="3" style="font-size:12px">已输入 {{ urlList.length }} 个</n-text></div>
            <n-button type="primary" block :disabled="!urlList.length" :loading="loading" @click="doCreate">创建任务</n-button>
          </n-space>
        </n-card>
      </n-gi>

      <n-gi>
        <n-card title="📁 批量导入" bordered>
          <n-space vertical size="medium">
            <n-upload :multiple="false" accept=".json" :show-file-list="false" @change="handleFileChange">
              <n-upload-dragger>
                <div style="text-align:center;padding:24px">
                  <n-icon size="36" color="var(--primary-color)"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg></n-icon>
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
.tpl-card { background:linear-gradient(135deg,var(--primary-color-suppl,#e8f4fd),transparent) }
.tpl-row { display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:16px }
.tpl-row h3 { margin:0 0 4px } .tpl-row p { margin:0; color:var(--n-text-color-3); font-size:13px }
.tpl-tags { display:flex; gap:8px; margin-top:12px; flex-wrap:wrap }
</style>
