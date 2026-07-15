<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute } from 'vue-router'
import Sidebar from '@/components/Sidebar.vue'

const route = useRoute()
const collapsed = ref(false)

const pageTitle = computed(() => {
  const titles: Record<string, string> = {
    'Dashboard': '概览',
    'Plans': '测试计划',
    'PlanEdit': '计划编辑',
    'PlanRuns': '运行历史',
    'History': '历史记录',
    'TaskDetail': '任务详情',
    'Settings': '系统设置',
  }
  return titles[String(route.name)] || 'NetPulse Web'
})
</script>

<template>
  <div class="app-layout">
    <Sidebar class="sidebar-slot" :class="{ collapsed }" />
    <div class="main-container">
      <header class="topbar">
        <div class="topbar-left">
          <button class="collapse-btn" @click="collapsed = !collapsed" :title="collapsed ? '展开' : '折叠'">
            {{ collapsed ? '»' : '«' }}
          </button>
          <h1 class="page-title">{{ pageTitle }}</h1>
        </div>
        <div class="topbar-right">
          <span class="env-tag">v0.2 · Plan System</span>
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
  width: 240px;
  transition: width var(--transition-base);
}

.sidebar-slot.collapsed {
  width: 64px;
}

.sidebar-slot.collapsed :deep(.logo-text),
.sidebar-slot.collapsed :deep(.nav-label),
.sidebar-slot.collapsed :deep(.user-details) {
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
  height: 60px;
  padding: 0 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-card);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.topbar-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.collapse-btn {
  width: 32px;
  height: 32px;
  border: 1px solid var(--border-color);
  background: var(--bg-card);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 14px;
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
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}

.topbar-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.env-tag {
  padding: 4px 10px;
  font-size: 11px;
  color: var(--text-tertiary);
  background: var(--bg-hover);
  border-radius: var(--radius-sm);
  font-weight: 500;
}

.content {
  flex: 1;
  padding: 24px;
  overflow-y: auto;
}

/* 路由切换动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.fade-enter-from {
  opacity: 0;
  transform: translateY(8px);
}
.fade-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
