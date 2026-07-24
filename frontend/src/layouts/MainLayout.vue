<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute } from 'vue-router'
import Sidebar from '@/components/Sidebar.vue'

const route = useRoute()
const collapsed = ref(false)

const pageTitle = computed(() => {
  const titles: Record<string, string> = {
    'Dashboard': '概览',
    'CreateTask': '创建任务',
    'Plans': '测试计划',
    'PlanEdit': '计划编辑',
    'PlanRuns': '运行历史',
    'History': '历史记录',
    'TaskDetail': '任务详情',
    'Settings': '系统设置',
  }
  return titles[String(route.name)] || 'NetPulse'
})
</script>

<template>
  <div class="app-layout">
    <Sidebar class="sidebar-slot" :class="{ collapsed }" />
    <div class="main-container">
      <header class="topbar">
        <div class="topbar-left">
          <button class="collapse-btn" @click="collapsed = !collapsed" :title="collapsed ? '展开' : '折叠'">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path v-if="!collapsed" d="M10 4L6 8l4 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              <path v-else d="M6 4l4 4-4 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <h1 class="page-title">{{ pageTitle }}</h1>
        </div>
      </header>

      <main class="content">
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </main>
    </div>
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  min-height: 100vh;
  background: var(--bg-body);
}

.sidebar-slot {
  width: 220px;
  transition: width var(--transition-base);
}

.sidebar-slot.collapsed {
  width: 64px;
}

.sidebar-slot.collapsed :deep(.logo-marker) {
  margin: 0 auto;
}

.sidebar-slot.collapsed :deep(.logo-title),
.sidebar-slot.collapsed :deep(.nav-label),
.sidebar-slot.collapsed :deep(.user-details),
.sidebar-slot.collapsed :deep(.nav-marker) {
  display: none;
}

.sidebar-slot.collapsed :deep(.nav-item) {
  justify-content: center;
  padding: 8px;
}

.sidebar-slot.collapsed :deep(.sidebar-header) {
  display: flex;
  justify-content: center;
}

.sidebar-slot.collapsed :deep(.user-info) {
  justify-content: center;
}

.sidebar-slot.collapsed :deep(.logout-btn) {
  display: none;
}

.main-container {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.topbar {
  height: 48px;
  padding: 0 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-body);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.topbar-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.collapse-btn {
  width: 28px;
  height: 28px;
  border: 1px solid var(--border-color);
  background: transparent;
  color: var(--text-tertiary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--transition-fast);
}

.collapse-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.page-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}

.content {
  flex: 1;
  padding: 24px;
  overflow-y: auto;
  background: var(--bg-alt);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
