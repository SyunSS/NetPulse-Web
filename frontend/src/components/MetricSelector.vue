<template>
  <div class="metric-selector">
    <div class="metric-header">
      <span class="metric-title">📊 采集指标</span>
      <button class="quick-btn" @click="toggleAll">{{ allSelected ? '全不选' : '全选' }}</button>
    </div>
    <div v-for="group in groups" :key="group.key" class="metric-group">
      <label class="group-label">
        <input type="checkbox" :checked="isGroupSelected(group)" @change="toggleGroup(group)" />
        <span class="group-name">{{ group.label }}</span>
        <span class="group-desc">{{ group.desc }}</span>
      </label>
      <div class="group-items" v-if="expanded[group.key]">
        <label v-for="m in group.items" :key="m.name" class="metric-item">
          <input type="checkbox" :checked="modelValue.includes(m.name)" @change="toggle(m.name)" />
          <span>{{ m.label }}</span>
        </label>
      </div>
    </div>
    <div class="selected-summary" v-if="modelValue.length > 0">
      已选 {{ modelValue.length }} 项
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

const props = defineProps<{ modelValue: string[] }>()
const emit = defineEmits(['update:modelValue'])

const expanded = ref<Record<string, boolean>>({})

const groups = [
  { key: 'basic', label: '基础网络', desc: 'DNS/TCP/TLS/HTTP状态码', items: [
    { name: 'dns_time', label: 'DNS解析时延' }, { name: 'tcp_time', label: 'TCP连接时延' },
    { name: 'tls_time', label: 'TLS握手时延' }, { name: 'ttfb', label: '首包时延(TTFB)' },
    { name: 'http_status', label: 'HTTP状态码' },
  ]},
  { key: 'page', label: '页面性能', desc: 'FP/FCP/DOM/Load', items: [
    { name: 'fcp', label: '首屏时延(FCP)' }, { name: 'dom_load', label: 'DOM加载时延' },
    { name: 'load_time', label: '首页时延(Load)' },
  ]},
  { key: 'resource', label: '资源统计', desc: 'HTML/CSS/JS/图片大小', items: [
    { name: 'total_size', label: '页面总大小' }, { name: 'html_size', label: 'HTML大小' },
    { name: 'css_size', label: 'CSS大小' }, { name: 'js_size', label: 'JS大小' },
    { name: 'img_size', label: '图片大小' }, { name: 'requests', label: '请求数量' },
  ]},
  { key: 'performance', label: '高级性能', desc: 'LCP/CLS/TTI (较耗时)', items: [
    { name: 'lcp', label: '最大内容绘制' }, { name: 'cls', label: '累计布局偏移' },
    { name: 'tti', label: '可交互时间' },
  ]},
]

const allSelected = computed(() => {
  const all = groups.flatMap(g => g.items.map(i => i.name))
  return all.every(n => props.modelValue.includes(n))
})

function toggle(name: string) {
  const next = props.modelValue.includes(name)
    ? props.modelValue.filter(n => n !== name)
    : [...props.modelValue, name]
  emit('update:modelValue', next)
}

function toggleAll() {
  if (allSelected.value) { emit('update:modelValue', []); return }
  const all = groups.flatMap(g => g.items.map(i => i.name))
  emit('update:modelValue', all)
}

function isGroupSelected(group: typeof groups[0]) {
  return group.items.every(m => props.modelValue.includes(m.name))
}

function toggleGroup(group: typeof groups[0]) {
  const names = group.items.map(i => i.name)
  const allSelected = names.every(n => props.modelValue.includes(n))
  if (allSelected) {
    emit('update:modelValue', props.modelValue.filter(n => !names.includes(n)))
  } else {
    const merged = new Set([...props.modelValue, ...names])
    emit('update:modelValue', [...merged])
  }
}
</script>

<style scoped>
.metric-selector { border: 1px solid var(--border-color); border-radius: var(--radius-md); padding: 12px 16px; margin-top: 8px; }
.metric-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
.metric-title { font-weight: 600; font-size: 14px; }
.quick-btn { height: 26px; padding: 0 12px; border: 1px solid var(--border-color); background: var(--bg-card); border-radius: var(--radius-sm); cursor: pointer; font-size: 12px; }
.metric-group { padding: 4px 0; }
.group-label { display: flex; align-items: center; gap: 6px; cursor: pointer; font-size: 13px; font-weight: 500; }
.group-label input { margin: 0; }
.group-name { min-width: 70px; }
.group-desc { font-size: 11px; color: var(--text-tertiary); }
.group-items { margin-left: 22px; padding: 2px 0; }
.metric-item { display: flex; align-items: center; gap: 6px; font-size: 12px; cursor: pointer; padding: 1px 0; }
.metric-item input { margin: 0; }
.selected-summary { margin-top: 8px; font-size: 11px; color: var(--text-tertiary); }
</style>
