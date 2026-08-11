<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, h } from 'vue'
import { useRouter } from 'vue-router'
import { usePlanStore } from '@/stores/plan'
import { useTaskStore } from '@/stores/task'
import { getErrorMessage } from '@/api/index'
import { formatMs, formatTime } from '@/utils'
import VChart from 'vue-echarts'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { LineChart, PieChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent, TitleComponent } from 'echarts/components'

use([CanvasRenderer, LineChart, PieChart, GridComponent, TooltipComponent, LegendComponent, TitleComponent])

const router = useRouter()
const planStore = usePlanStore()
const taskStore = useTaskStore()
const dashboardError = ref('')

const trendOption = ref<any>({
  color: ['#61e7d2', '#88a9ff', '#ffb454'],
  tooltip: { trigger: 'axis', backgroundColor: '#12263a', borderColor: 'rgba(97,231,210,.35)', textStyle: { color: '#edf6ff' } },
  legend: { data: ['DNS', 'TTFB', '页面打开'], top: 0, right: 0, textStyle: { color: '#9bb0c2', fontSize: 11 } },
  grid: { left: 44, right: 8, top: 38, bottom: 28 },
  xAxis: {
    type: 'category',
    data: [],
    axisLabel: { color: '#888', formatter: (v: string) => {
      const d = new Date(v);
       return d.toLocaleTimeString('zh-CN', { timeZone: 'Asia/Shanghai', hour12: false });
     }, fontFamily: 'IBM Plex Mono, monospace', fontSize: 10 }
   },
   yAxis: { type: 'value', name: 'ms', nameTextStyle: { color: '#61788e', fontSize: 10 }, axisLabel: { color: '#61788e', fontFamily: 'IBM Plex Mono, monospace', fontSize: 10 }, splitLine: { lineStyle: { color: 'rgba(136,169,201,0.12)', type: 'dashed' } } },
  series: [
    { name: 'DNS', type: 'line', smooth: true, data: [], lineStyle: { width: 2 } },
    { name: 'TTFB', type: 'line', smooth: true, data: [], lineStyle: { width: 2 } },
    { name: '页面打开', type: 'line', smooth: true, data: [], lineStyle: { width: 2 } },
  ],
})

const pieOption = ref<any>({
  color: ['#61e7d2', '#88a9ff', '#ffb454'],
  tooltip: { trigger: 'item', backgroundColor: '#12263a', borderColor: 'rgba(97,231,210,.35)', textStyle: { color: '#edf6ff' } },
  legend: { bottom: 0, textStyle: { color: '#9bb0c2', fontSize: 11 } },
  series: [{
    type: 'pie', radius: ['45%', '75%'], center: ['50%', '45%'],
    data: [],
    label: { show: false },
    emphasis: { label: { show: true, fontSize: 16, fontWeight: 'bold' } },
  }],
})

function updateCharts(stats: any) {
  trendOption.value = {
    ...trendOption.value,
    xAxis: { ...trendOption.value.xAxis, data: stats.trend_data.map((d: any) => d.time) },
    series: [
      { ...trendOption.value.series[0], data: stats.trend_data.map((d: any) => d.dns_ms) },
      { ...trendOption.value.series[1], data: stats.trend_data.map((d: any) => d.ttfb_ms) },
      { ...trendOption.value.series[2], data: stats.trend_data.map((d: any) => d.page_ms) },
    ],
  }
  const counts = { website: 0, video: 0, download: 0 }
  stats.recent_tasks.forEach((t: any) => {
    if (counts[t.task_type as keyof typeof counts] !== undefined) {
      counts[t.task_type as keyof typeof counts]++
    }
  })
  pieOption.value = {
    ...pieOption.value,
    series: [{
      ...pieOption.value.series[0],
      data: [
        { name: '网站测试', value: counts.website },
        { name: '视频测试', value: counts.video },
        { name: '下载测试', value: counts.download },
      ],
    }],
  }
}

async function loadDashboard() {
  dashboardError.value = ''
  try {
    await Promise.all([taskStore.refreshDashboard(), planStore.fetchAllPlans()])
  } catch (e) {
    dashboardError.value = getErrorMessage(e, '概览加载失败')
  }
}

onMounted(() => {
  taskStore.connectWs()
  loadDashboard()
})

onUnmounted(() => {
  taskStore.disconnectWs()
})

watch(() => taskStore.dashboardStats, (stats) => {
  if (stats) updateCharts(stats)
})

// 下次执行时间排序
const upcomingPlans = () => {
  return planStore.plans
    .filter(p => p.cron_expression && p.enabled === 1 && p.next_run_at)
    .sort((a, b) => (a.next_run_at || '').localeCompare(b.next_run_at || ''))
    .slice(0, 5)
}

const enabledPlanCount = () => planStore.plans.filter(p => p.enabled === 1).length
const cronPlanCount = () => planStore.plans.filter(p => p.cron_expression).length
const totalItems = () => planStore.plans.reduce((sum, p) => sum + p.items.length, 0)
</script>

<template>
  <div class="dashboard">
    <section class="dashboard-intro">
      <div class="intro-copy">
        <div class="section-eyebrow"><span class="eyebrow-mark"></span> network telemetry / 24h window</div>
        <h2>让每一次探测，<em>都有信号可读。</em></h2>
        <p>把 DNS、首包、页面和播放质量放在同一张实时态势图里。</p>
      </div>
      <div class="intro-actions">
        <div class="system-pulse"><span></span><div><b>系统在线</b><small>last sync just now</small></div></div>
        <button class="signal-button" type="button" @click="router.push('/create')"><span>+</span> 发起探测</button>
      </div>
    </section>

    <div v-if="dashboardError" class="load-error">
      {{ dashboardError }} <button class="link-btn" @click="loadDashboard">重试</button>
    </div>

    <section class="signal-grid">
      <div class="health-panel">
        <div class="panel-topline"><span>NETWORK HEALTH</span><span class="panel-index">01 / 04</span></div>
        <div class="health-main"><div><div class="health-value">{{ (taskStore.dashboardStats?.success_rate ?? 0).toFixed(1) }}<small>%</small></div><div class="health-label">探测成功率</div></div><div class="health-ring"><span></span><b>{{ taskStore.dashboardStats?.today_tests ?? 0 }}</b><small>today tests</small></div></div>
        <div class="health-meter"><span :style="{ width: `${taskStore.dashboardStats?.success_rate ?? 0}%` }"></span></div>
        <div class="health-foot"><span>过去 24 小时</span><span>目标阈值 95%</span></div>
      </div>
      <div v-for="(item, index) in [
        { label: '启用计划', value: enabledPlanCount(), sub: `共 ${planStore.total} 个计划`, tone: 'cyan' },
        { label: '定时探测', value: cronPlanCount(), sub: 'cron schedules', tone: 'blue' },
        { label: '测试项', value: totalItems(), sub: 'across all plans', tone: 'amber' },
      ]" :key="item.label" class="metric-panel" :class="`metric-${item.tone}`">
        <div class="panel-topline"><span>{{ item.label }}</span><span class="panel-index">0{{ index + 2 }} / 04</span></div>
        <div class="metric-value">{{ item.value }}</div><div class="metric-sub">{{ item.sub }}</div><div class="metric-rule"><i></i><i></i><i></i><i></i><i></i></div>
      </div>
    </section>

    <section class="telemetry-grid">
      <div class="panel chart-panel">
        <div class="panel-heading"><div><div class="section-eyebrow">latency trace / milliseconds</div><h3>性能趋势</h3></div><span class="live-chip"><i></i> LIVE TRACE</span></div>
        <v-chart :option="trendOption" class="trend-chart" autoresize />
        <div class="chart-legend"><span><i class="legend-cyan"></i> DNS</span><span><i class="legend-blue"></i> TTFB</span><span><i class="legend-amber"></i> 页面打开</span><span class="chart-unit">单位：ms</span></div>
      </div>
      <div class="panel distribution-panel">
        <div class="panel-heading"><div><div class="section-eyebrow">probe mix / by engine</div><h3>探测构成</h3></div><span class="panel-index">24H</span></div>
        <v-chart :option="pieOption" class="pie-chart" autoresize />
        <div class="latency-strip"><div><span>DNS</span><b>{{ formatMs(taskStore.dashboardStats?.avg_dns ?? 0) }}</b></div><div><span>TTFB</span><b>{{ formatMs(taskStore.dashboardStats?.avg_ttfb ?? 0) }}</b></div><div><span>FCP</span><b>{{ formatMs(taskStore.dashboardStats?.avg_page_time ?? 0) }}</b></div></div>
      </div>
    </section>

    <section class="bottom-grid">
      <div class="panel list-panel">
        <div class="panel-heading"><div><div class="section-eyebrow">next scheduled signal</div><h3>即将执行</h3></div><button class="text-action" type="button" @click="router.push('/plans')">查看全部 <span>↗</span></button></div>
        <div v-if="upcomingPlans().length === 0" class="mini-empty">暂无定时计划，创建一个调度探测吧。</div>
        <div v-else class="upcoming-list"><button v-for="plan in upcomingPlans()" :key="plan.id" type="button" class="upcoming-item" @click="router.push(`/plans/${plan.id}/runs`)"><span class="upcoming-track"><i></i></span><span class="upcoming-content"><b>{{ plan.name }}</b><small>{{ formatTime(plan.next_run_at!) }}</small></span><span class="row-arrow">↗</span></button></div>
      </div>
      <div class="panel list-panel">
        <div class="panel-heading"><div><div class="section-eyebrow">configured probes</div><h3>最近计划</h3></div><button class="text-action" type="button" @click="router.push('/plans')">管理计划 <span>↗</span></button></div>
        <div v-if="planStore.plans.length === 0" class="mini-empty">还没有计划，从一次手动探测开始。</div>
        <div v-else class="recent-plans-list"><button v-for="plan in planStore.plans.slice(0, 5)" :key="plan.id" type="button" class="recent-plan-item" @click="router.push(`/plans/${plan.id}/runs`)"><span class="plan-code">{{ String(plan.id).substring(0, 4).toUpperCase() }}</span><span><b>{{ plan.name }}</b><small>{{ plan.items.length }} 项 · {{ plan.enabled === 1 ? '运行中' : '已暂停' }}</small></span><span class="row-arrow">↗</span></button></div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.dashboard { max-width: 1440px; margin: 0 auto; }
.dashboard-intro { display: flex; align-items: flex-end; justify-content: space-between; gap: 24px; padding: 7px 0 28px; }
.section-eyebrow { display: flex; align-items: center; gap: 8px; color: var(--color-primary); font-family: var(--font-mono); font-size: 9px; letter-spacing: .14em; text-transform: uppercase; }
.eyebrow-mark { width: 20px; height: 1px; background: var(--color-primary); box-shadow: 0 0 9px var(--color-primary); }
.intro-copy h2 { max-width: 570px; margin-top: 12px; color: var(--text-primary); font-family: var(--font-display); font-size: clamp(30px, 4vw, 48px); font-weight: 500; letter-spacing: -.055em; line-height: 1.04; }.intro-copy h2 em { color: var(--color-primary); font-style: normal; }.intro-copy p { margin-top: 14px; color: var(--text-secondary); font-size: 13px; }.intro-actions { display: flex; align-items: center; gap: 20px; }.system-pulse { display: flex; align-items: center; gap: 9px; }.system-pulse > span { width: 7px; height: 7px; border-radius: 50%; background: var(--color-primary); box-shadow: 0 0 0 5px var(--color-primary-bg), 0 0 14px var(--color-primary); }.system-pulse b, .system-pulse small { display: block; }.system-pulse b { color: var(--text-primary); font-family: var(--font-mono); font-size: 10px; font-weight: 500; }.system-pulse small { margin-top: 3px; color: var(--text-tertiary); font-family: var(--font-mono); font-size: 8px; }.signal-button { height: 39px; padding: 0 16px; border: 1px solid var(--color-primary); border-radius: 6px; background: var(--color-primary); color: #06201f; cursor: pointer; font-size: 12px; font-weight: 700; transition: all var(--transition-fast); }.signal-button:hover { background: var(--color-primary-active); box-shadow: var(--shadow-glow); transform: translateY(-1px); }.signal-button span { margin-right: 6px; font-family: var(--font-mono); font-size: 16px; vertical-align: -1px; }
.signal-grid { display: grid; grid-template-columns: minmax(280px, 1.4fr) repeat(3, 1fr); gap: 12px; margin-bottom: 12px; }.panel { position: relative; overflow: hidden; border: 1px solid var(--border-color); border-radius: var(--radius-lg); background: linear-gradient(145deg, var(--bg-card), var(--bg-elevated)); box-shadow: var(--shadow-card); }.panel::after { position: absolute; right: -30px; bottom: -38px; width: 130px; height: 130px; border: 1px solid rgba(97,231,210,.08); border-radius: 50%; content: ''; pointer-events: none; }.health-panel, .metric-panel { min-height: 170px; padding: 20px; }.panel-topline { display: flex; align-items: center; justify-content: space-between; color: var(--text-secondary); font-family: var(--font-mono); font-size: 9px; letter-spacing: .1em; text-transform: uppercase; }.panel-index { color: var(--text-tertiary); font-size: 8px; }.health-main { display: flex; align-items: center; justify-content: space-between; margin-top: 19px; }.health-value { color: var(--color-primary); font-family: var(--font-mono); font-size: 38px; letter-spacing: -.08em; line-height: 1; text-shadow: 0 0 20px rgba(97,231,210,.22); }.health-value small { margin-left: 3px; font-size: 16px; letter-spacing: 0; }.health-label { margin-top: 8px; color: var(--text-secondary); font-size: 12px; }.health-ring { position: relative; display: flex; flex-direction: column; align-items: center; justify-content: center; width: 74px; height: 74px; border: 1px solid var(--border-bright); border-radius: 50%; background: radial-gradient(circle, rgba(97,231,210,.12), transparent 68%); }.health-ring::before { position: absolute; inset: 4px; border: 1px dashed rgba(97,231,210,.26); border-radius: 50%; content: ''; }.health-ring span { position: absolute; top: -3px; right: 9px; width: 5px; height: 5px; border-radius: 50%; background: var(--color-primary); box-shadow: 0 0 8px var(--color-primary); }.health-ring b { color: var(--text-primary); font-family: var(--font-mono); font-size: 17px; font-weight: 500; }.health-ring small { color: var(--text-tertiary); font-family: var(--font-mono); font-size: 7px; }.health-meter { height: 4px; margin-top: 22px; overflow: hidden; border-radius: 4px; background: rgba(136,169,201,.12); }.health-meter span { display: block; height: 100%; border-radius: inherit; background: var(--color-primary); box-shadow: 0 0 10px rgba(97,231,210,.6); transition: width .6s ease; }.health-foot { display: flex; justify-content: space-between; margin-top: 8px; color: var(--text-tertiary); font-family: var(--font-mono); font-size: 8px; }.metric-panel { display: flex; flex-direction: column; }.metric-value { margin-top: auto; color: var(--text-primary); font-family: var(--font-mono); font-size: 40px; font-weight: 500; letter-spacing: -.08em; line-height: 1; }.metric-sub { margin-top: 9px; color: var(--text-secondary); font-family: var(--font-mono); font-size: 10px; }.metric-rule { display: flex; gap: 3px; margin-top: 18px; }.metric-rule i { display: block; width: 16px; height: 3px; border-radius: 2px; background: var(--color-primary); opacity: .25; }.metric-rule i:nth-child(-n+3) { opacity: .85; }.metric-blue .metric-rule i { background: var(--color-info); }.metric-amber .metric-rule i { background: var(--color-warning); }.metric-cyan { border-top-color: var(--color-primary); }.metric-blue { border-top-color: var(--color-info); }.metric-amber { border-top-color: var(--color-warning); }
.telemetry-grid { display: grid; grid-template-columns: minmax(0, 1.75fr) minmax(280px, 1fr); gap: 12px; margin-bottom: 12px; }.chart-panel, .distribution-panel, .list-panel { padding: 20px; }.panel-heading { position: relative; z-index: 1; display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }.panel-heading h3 { margin-top: 5px; color: var(--text-primary); font-family: var(--font-display); font-size: 17px; font-weight: 600; letter-spacing: -.035em; }.live-chip { display: flex; align-items: center; gap: 6px; padding: 5px 7px; border: 1px solid var(--border-color); border-radius: 4px; color: var(--color-primary); font-family: var(--font-mono); font-size: 8px; letter-spacing: .08em; }.live-chip i { width: 5px; height: 5px; border-radius: 50%; background: currentColor; box-shadow: 0 0 6px currentColor; }.trend-chart { width: 100%; height: 265px; margin-top: 5px; }.chart-legend { display: flex; align-items: center; gap: 18px; padding-top: 11px; border-top: 1px solid var(--border-color); color: var(--text-secondary); font-family: var(--font-mono); font-size: 9px; }.chart-legend span { display: flex; align-items: center; gap: 5px; }.chart-legend i { width: 5px; height: 5px; border-radius: 50%; }.legend-cyan { background: var(--color-primary); }.legend-blue { background: var(--color-info); }.legend-amber { background: var(--color-warning); }.chart-unit { margin-left: auto; color: var(--text-tertiary); }.pie-chart { width: 100%; height: 205px; }.latency-strip { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; padding-top: 10px; border-top: 1px solid var(--border-color); }.latency-strip div { display: flex; flex-direction: column; gap: 4px; }.latency-strip span { color: var(--text-tertiary); font-family: var(--font-mono); font-size: 8px; }.latency-strip b { color: var(--text-primary); font-family: var(--font-mono); font-size: 12px; font-weight: 500; }.bottom-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }.text-action { border: 0; background: transparent; color: var(--color-primary); cursor: pointer; font-family: var(--font-mono); font-size: 9px; }.text-action span { margin-left: 4px; font-size: 13px; }.upcoming-list, .recent-plans-list { position: relative; z-index: 1; display: flex; flex-direction: column; gap: 7px; margin-top: 17px; }.upcoming-item, .recent-plan-item { display: flex; align-items: center; width: 100%; gap: 11px; padding: 10px 11px; border: 1px solid transparent; border-radius: 7px; background: var(--bg-alt); color: var(--text-primary); cursor: pointer; text-align: left; transition: all var(--transition-fast); }.upcoming-item:hover, .recent-plan-item:hover { border-color: var(--border-bright); background: var(--bg-hover); }.upcoming-track { display: flex; align-items: center; justify-content: center; width: 17px; height: 17px; border: 1px solid var(--border-bright); border-radius: 50%; }.upcoming-track i { width: 5px; height: 5px; border-radius: 50%; background: var(--color-primary); box-shadow: 0 0 7px var(--color-primary); }.upcoming-content, .recent-plan-item > span:nth-child(2) { display: flex; min-width: 0; flex: 1; flex-direction: column; gap: 3px; }.upcoming-content b, .recent-plan-item b { overflow: hidden; font-size: 12px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }.upcoming-content small, .recent-plan-item small { color: var(--text-tertiary); font-family: var(--font-mono); font-size: 9px; }.row-arrow { color: var(--text-tertiary); font-size: 14px; }.plan-code { display: flex; align-items: center; justify-content: center; width: 29px; height: 29px; border: 1px solid var(--border-bright); border-radius: 5px; color: var(--color-primary); font-family: var(--font-mono); font-size: 8px; }.mini-empty { padding: 34px 0 16px; color: var(--text-tertiary); font-size: 12px; text-align: center; }
.load-error { margin-bottom: 12px; padding: 11px 14px; border: 1px solid rgba(255,138,101,.35); border-radius: var(--radius-md); color: var(--color-danger); background: rgba(255,138,101,.08); font-size: 12px; }.link-btn { border: 0; background: transparent; color: var(--color-primary); cursor: pointer; font-size: 12px; }
@media (max-width: 1050px) { .signal-grid { grid-template-columns: repeat(2, 1fr); }.health-panel { grid-column: span 2; }.telemetry-grid { grid-template-columns: 1fr; } }
@media (max-width: 650px) { .dashboard-intro { display: block; padding-bottom: 22px; }.intro-actions { justify-content: space-between; margin-top: 22px; }.signal-grid, .bottom-grid { grid-template-columns: 1fr; }.health-panel { grid-column: auto; }.health-panel, .metric-panel { min-height: 155px; }.chart-panel, .distribution-panel, .list-panel { padding: 16px; }.trend-chart { height: 220px; }.chart-legend { flex-wrap: wrap; gap: 10px; }.chart-unit { width: 100%; margin-left: 0; }.topbar + .content {} }
</style>
