<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()

interface NavItem {
  key: string
  label: string
  to: string
}

const navItems: NavItem[] = [
  { key: 'dashboard', label: '概览', to: '/' },
  { key: 'create', label: '创建任务', to: '/create' },
  { key: 'plans', label: '测试计划', to: '/plans' },
  { key: 'history', label: '历史记录', to: '/history' },
  { key: 'settings', label: '系统设置', to: '/settings' },
]

const isActive = (path: string) => {
  if (path === '/') return route.path === '/'
  return route.path.startsWith(path)
}

const goTo = (path: string) => {
  if (path !== route.path) router.push(path)
}

const handleLogout = () => {
  authStore.logout()
  router.push('/login')
}
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-header">
      <div class="logo">
        <span class="logo-marker">◆</span>
        <span class="logo-title">NetPulse</span>
      </div>
    </div>

    <nav class="sidebar-nav">
      <div
        v-for="item in navItems"
        :key="item.key"
        class="nav-item"
        :class="{ active: isActive(item.to) }"
        @click="goTo(item.to)"
      >
        <span class="nav-marker" v-if="isActive(item.to)"></span>
        <span class="nav-label">{{ item.label }}</span>
      </div>
    </nav>

    <div class="sidebar-footer">
      <div class="user-info" v-if="authStore.user">
        <div class="user-avatar">{{ authStore.user.username.charAt(0).toUpperCase() }}</div>
        <div class="user-details">
          <div class="user-name">{{ authStore.user.username }}</div>
          <div class="user-role">{{ authStore.user.role === 'admin' ? '管理员' : '用户' }}</div>
        </div>
      </div>
      <button class="logout-btn" @click="handleLogout" title="退出登录">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M6 2H3a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3m5-4 2-2-2-2m2 2H7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 220px;
  height: 100vh;
  background: var(--bg-body);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  transition: width var(--transition-base);
}

.sidebar-header {
  padding: 20px 16px 12px;
}

.logo {
  display: flex;
  align-items: center;
  gap: 8px;
}

.logo-marker {
  font-size: 14px;
  color: var(--color-primary);
}

.logo-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -0.25px;
}

.sidebar-nav {
  flex: 1;
  padding: 8px 8px;
  overflow-y: auto;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  color: var(--text-secondary);
  font-size: 15px;
  font-weight: 500;
  margin-bottom: 2px;
  transition: all var(--transition-fast);
  position: relative;
}

.nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.nav-item.active {
  background: var(--color-primary-bg);
  color: var(--color-primary-text);
  font-weight: 600;
}

.nav-marker {
  position: absolute;
  left: -8px;
  width: 3px;
  height: 16px;
  background: var(--color-primary);
  border-radius: 0 2px 2px 0;
}

.nav-label {
  flex: 1;
}

.sidebar-footer {
  padding: 12px 12px;
  border-top: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  gap: 8px;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}

.user-avatar {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--bg-hover);
  border: 1px solid var(--border-color);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
  font-weight: 600;
  font-size: 12px;
  flex-shrink: 0;
}

.user-details {
  flex: 1;
  min-width: 0;
}

.user-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.user-role {
  font-size: 11px;
  color: var(--text-tertiary);
}

.logout-btn {
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

.logout-btn:hover {
  color: var(--color-danger);
  border-color: var(--color-danger);
}
</style>
