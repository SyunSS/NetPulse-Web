<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import Sidebar from '@/components/Sidebar.vue'
import { useDark } from '@/utils/theme'

const route = useRoute()
const collapsed = ref(false)
const mobileOpen = ref(false)
const { isDark, toggleDark } = useDark()

const pageMeta: Record<string, { title: string; eyebrow: string }> = {
  Dashboard: { title: '网络态势', eyebrow: 'signal overview' },
  CreateTask: { title: '创建探测', eyebrow: 'new probe' },
  Plans: { title: '测试计划', eyebrow: 'probe schedules' },
  PlanEdit: { title: '计划配置', eyebrow: 'schedule editor' },
  PlanRuns: { title: '运行历史', eyebrow: 'execution archive' },
  History: { title: '历史记录', eyebrow: 'task archive' },
  TaskDetail: { title: '任务详情', eyebrow: 'probe telemetry' },
  Settings: { title: '系统设置', eyebrow: 'control room' },
}

const meta = computed(() => pageMeta[String(route.name)] || { title: 'NetPulse', eyebrow: 'signal room' })
watch(() => route.fullPath, () => { mobileOpen.value = false })
</script>

<template>
  <div class="app-layout" :class="{ 'is-collapsed': collapsed }">
    <div v-if="mobileOpen" class="mobile-scrim" aria-hidden="true" @click="mobileOpen = false"></div>
    <Sidebar class="sidebar-slot" :class="{ 'mobile-open': mobileOpen }" />
    <div class="main-container">
      <header class="topbar">
        <div class="topbar-left">
          <button class="mobile-menu-btn" type="button" aria-label="打开导航" @click="mobileOpen = true">
            <span></span><span></span><span></span>
          </button>
          <button class="collapse-btn" type="button" :title="collapsed ? '展开侧栏' : '收起侧栏'" @click="collapsed = !collapsed">
            <svg viewBox="0 0 24 24"><path :d="collapsed ? 'm9 6 6 6-6 6' : 'm15 6-6 6 6 6'" /></svg>
          </button>
          <div class="topbar-context">
            <span class="topbar-eyebrow">{{ meta.eyebrow }}</span>
            <h1 class="page-title">{{ meta.title }}</h1>
          </div>
        </div>
        <div class="topbar-right">
          <div class="live-readout"><span class="live-dot"></span><span>实时连接</span><b>WS</b></div>
          <button class="theme-btn" type="button" :title="isDark ? '切换浅色模式' : '切换深色模式'" @click="toggleDark">
            <svg v-if="isDark" viewBox="0 0 24 24"><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4M2 12h2m16 0h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></svg>
            <svg v-else viewBox="0 0 24 24"><path d="M20.5 15.2A8.5 8.5 0 0 1 8.8 3.5 8.5 8.5 0 1 0 20.5 15.2Z" /></svg>
          </button>
        </div>
      </header>
      <main class="content">
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in"><component :is="Component" /></transition>
        </router-view>
      </main>
    </div>
  </div>
</template>

<style scoped>
.app-layout { display: flex; min-height: 100vh; background: var(--bg-body); }.sidebar-slot { width: 248px; }.app-layout.is-collapsed .sidebar-slot { width: 72px; }.app-layout.is-collapsed .sidebar-slot :deep(.sidebar) { width: 72px; }.app-layout.is-collapsed :deep(.brand-copy), .app-layout.is-collapsed :deep(.brand-status), .app-layout.is-collapsed :deep(.nav-kicker), .app-layout.is-collapsed :deep(.nav-label), .app-layout.is-collapsed :deep(.nav-hint), .app-layout.is-collapsed :deep(.nav-signal), .app-layout.is-collapsed :deep(.user-details), .app-layout.is-collapsed :deep(.logout-btn) { display: none; }.app-layout.is-collapsed :deep(.sidebar-header) { justify-content: center; padding-inline: 0; }.app-layout.is-collapsed :deep(.nav-item) { justify-content: center; padding-inline: 0; }.app-layout.is-collapsed :deep(.sidebar-divider) { margin-inline: 14px; }.app-layout.is-collapsed :deep(.sidebar-footer) { justify-content: center; padding-inline: 0; }.main-container { display: flex; min-width: 0; flex: 1; flex-direction: column; overflow: hidden; }.topbar { display: flex; align-items: center; justify-content: space-between; min-height: 74px; padding: 0 32px; border-bottom: 1px solid var(--border-color); background: var(--bg-alt); backdrop-filter: blur(16px); }.topbar-left, .topbar-right { display: flex; align-items: center; }.topbar-left { gap: 14px; }.collapse-btn, .mobile-menu-btn, .theme-btn { display: flex; align-items: center; justify-content: center; border: 1px solid var(--border-color); border-radius: 7px; background: transparent; color: var(--text-secondary); cursor: pointer; transition: all var(--transition-fast); }.collapse-btn { width: 31px; height: 31px; }.collapse-btn:hover, .theme-btn:hover { border-color: var(--border-color-hover); color: var(--color-primary); background: var(--bg-hover); }.collapse-btn svg { width: 16px; height: 16px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.7; }.topbar-context { display: flex; align-items: baseline; gap: 12px; }.topbar-eyebrow { color: var(--color-primary); font-family: var(--font-mono); font-size: 9px; letter-spacing: .13em; text-transform: uppercase; }.page-title { color: var(--text-primary); font-family: var(--font-display); font-size: 19px; font-weight: 600; letter-spacing: -.035em; }.topbar-right { gap: 16px; }.live-readout { display: flex; align-items: center; gap: 7px; color: var(--text-secondary); font-family: var(--font-mono); font-size: 10px; }.live-readout b { padding: 2px 5px; border: 1px solid var(--border-color); border-radius: 3px; color: var(--text-tertiary); font-size: 8px; font-weight: 500; }.live-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--color-primary); box-shadow: 0 0 9px var(--color-primary); }.theme-btn { width: 31px; height: 31px; }.theme-btn svg { width: 15px; height: 15px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.6; }.content { min-width: 0; flex: 1; overflow-y: auto; padding: 32px; background: var(--bg-body); background-image: radial-gradient(circle at 92% 0%, rgba(97,231,210,.06), transparent 28%), linear-gradient(rgba(97,231,210,.018) 1px, transparent 1px), linear-gradient(90deg, rgba(97,231,210,.018) 1px, transparent 1px); background-size: auto, 32px 32px, 32px 32px; }.fade-enter-active, .fade-leave-active { transition: opacity .18s ease, transform .18s ease; }.fade-enter-from, .fade-leave-to { opacity: 0; transform: translateY(4px); }.mobile-menu-btn, .mobile-scrim { display: none; }
@media (max-width: 760px) { .app-layout.is-collapsed .sidebar-slot { width: 0; }.topbar { min-height: 64px; padding: 0 16px; }.collapse-btn { display: none; }.mobile-menu-btn { display: flex; flex-direction: column; gap: 3px; width: 31px; height: 31px; }.mobile-menu-btn span { width: 13px; height: 1px; background: currentColor; }.mobile-scrim { position: fixed; z-index: 19; inset: 0; display: block; background: rgba(2, 8, 16, .64); }.topbar-context { display: block; }.topbar-eyebrow { display: block; margin-bottom: 2px; font-size: 8px; }.page-title { font-size: 17px; }.live-readout { display: none; }.content { padding: 20px 14px 28px; } }
</style>

<style scoped>
@media (max-width:760px) { .app-layout { display:block; }.sidebar-slot { width:0; }.app-layout .sidebar-slot.mobile-open { width:min(280px,88vw); }.main-container { min-height:100vh; }.topbar { position:sticky; top:0; z-index:10; }.topbar-left { min-width:0; gap:10px; }.topbar-context { min-width:0; }.page-title { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; max-width:58vw; }.topbar-right { gap:8px; }.theme-btn { width:34px; height:34px; }.content { min-height:calc(100vh - 64px); } }
@media (max-width:390px) { .topbar { padding:0 12px; }.topbar-eyebrow { display:none; }.page-title { max-width:62vw; font-size:16px; }.content { padding:16px 10px 24px; } }
</style>
