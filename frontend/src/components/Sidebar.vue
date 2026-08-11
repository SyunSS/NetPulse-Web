<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

defineProps<{ collapsed?: boolean }>()

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()

interface NavItem {
  key: string
  label: string
  hint: string
  to: string
  icon: string
}

const navItems: NavItem[] = [
  { key: 'dashboard', label: '概览', hint: '态势', to: '/', icon: 'pulse' },
  { key: 'create', label: '创建任务', hint: '探测', to: '/create', icon: 'crosshair' },
  { key: 'plans', label: '测试计划', hint: '调度', to: '/plans', icon: 'layers' },
  { key: 'history', label: '历史记录', hint: '档案', to: '/history', icon: 'list' },
  { key: 'settings', label: '系统设置', hint: '设置', to: '/settings', icon: 'sliders' },
]

const isActive = (path: string) => path === '/' ? route.path === '/' : route.path.startsWith(path)
const goTo = (path: string) => { if (path !== route.path) router.push(path) }
const handleLogout = () => { authStore.logout(); router.push('/login') }
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-header">
      <button class="brand" type="button" title="NetPulse" @click="goTo('/')">
        <span class="brand-mark" aria-hidden="true"><i></i><i></i><i></i></span>
        <span class="brand-copy"><strong>NetPulse</strong><small>signal room</small></span>
      </button>
      <span class="brand-status" aria-label="系统在线"></span>
    </div>

    <div class="nav-kicker">WORKSPACE</div>
    <nav class="sidebar-nav" aria-label="主导航">
      <button
        v-for="item in navItems"
        :key="item.key"
        type="button"
        class="nav-item"
        :class="{ active: isActive(item.to) }"
        :title="item.label"
        @click="goTo(item.to)"
      >
        <span class="nav-icon" aria-hidden="true">
          <svg v-if="item.icon === 'pulse'" viewBox="0 0 24 24"><path d="M3 12h4l2.2-7 4.1 14 2.2-7H21" /></svg>
          <svg v-else-if="item.icon === 'crosshair'" viewBox="0 0 24 24"><circle cx="12" cy="12" r="6" /><path d="M12 2v4m0 12v4M2 12h4m12 0h4" /></svg>
          <svg v-else-if="item.icon === 'layers'" viewBox="0 0 24 24"><path d="m12 3 9 5-9 5-9-5 9-5Z" /><path d="m3 12 9 5 9-5M3 16l9 5 9-5" /></svg>
          <svg v-else-if="item.icon === 'list'" viewBox="0 0 24 24"><path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01" /></svg>
          <svg v-else viewBox="0 0 24 24"><path d="M4 7h16M7 4v6m10-6v6M4 17h16M7 14v6m10-6v6" /></svg>
        </span>
        <span class="nav-label">{{ item.label }}</span>
        <span class="nav-hint">{{ item.hint }}</span>
        <span v-if="isActive(item.to)" class="nav-signal"></span>
      </button>
    </nav>

    <div class="sidebar-divider"></div>
    <div class="sidebar-footer">
      <div v-if="authStore.user" class="user-info">
        <div class="user-avatar">{{ authStore.user.username.charAt(0).toUpperCase() }}</div>
        <div class="user-details">
          <div class="user-name">{{ authStore.user.username }}</div>
          <div class="user-role"><span class="online-dot"></span>{{ authStore.user.role === 'admin' ? '管理员' : '操作员' }}</div>
        </div>
      </div>
      <button class="logout-btn" type="button" title="退出登录" @click="handleLogout">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M10 4H5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h5m5-5 4-3-4-3m4 3H9" /></svg>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 248px;
  height: 100vh;
  position: sticky;
  top: 0;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
  background: var(--bg-alt);
  border-right: 1px solid var(--border-color);
  transition: width var(--transition-base), transform var(--transition-base);
}

.sidebar::before {
  position: absolute;
  inset: 0;
  z-index: 0;
  background-image: linear-gradient(rgba(97, 231, 210, 0.035) 1px, transparent 1px), linear-gradient(90deg, rgba(97, 231, 210, 0.035) 1px, transparent 1px);
  background-size: 32px 32px;
  content: '';
  pointer-events: none;
}

.sidebar > * { position: relative; z-index: 1; }
.sidebar-header { display: flex; align-items: center; justify-content: space-between; padding: 25px 20px 24px; }
.brand { display: flex; align-items: center; gap: 11px; border: 0; background: transparent; color: var(--text-primary); cursor: pointer; text-align: left; }
.brand-mark { display: flex; align-items: flex-end; gap: 3px; width: 25px; height: 25px; padding: 4px; border: 1px solid var(--border-bright); border-radius: 7px; background: rgba(97, 231, 210, 0.08); }
.brand-mark i { display: block; width: 3px; border-radius: 3px; background: var(--color-primary); box-shadow: 0 0 8px rgba(97, 231, 210, .7); }
.brand-mark i:nth-child(1) { height: 7px; opacity: .6; }.brand-mark i:nth-child(2) { height: 13px; }.brand-mark i:nth-child(3) { height: 10px; opacity: .8; }
.brand-copy { display: flex; flex-direction: column; line-height: 1.05; }.brand-copy strong { font-family: var(--font-display); font-size: 17px; letter-spacing: -.04em; }.brand-copy small { margin-top: 4px; color: var(--text-tertiary); font-family: var(--font-mono); font-size: 8px; letter-spacing: .13em; text-transform: uppercase; }
.brand-status { width: 7px; height: 7px; border-radius: 50%; background: var(--color-primary); box-shadow: 0 0 0 4px rgba(97, 231, 210, .1), 0 0 14px rgba(97, 231, 210, .8); }
.nav-kicker { padding: 0 20px 10px; color: var(--text-tertiary); font-family: var(--font-mono); font-size: 9px; letter-spacing: .18em; }
.sidebar-nav { display: flex; flex-direction: column; gap: 4px; padding: 0 12px; }
.nav-item { position: relative; display: flex; align-items: center; gap: 11px; width: 100%; min-height: 45px; padding: 0 11px; border: 1px solid transparent; border-radius: 8px; background: transparent; color: var(--text-secondary); cursor: pointer; text-align: left; transition: all var(--transition-fast); }
.nav-item:hover { border-color: var(--border-color); background: var(--bg-hover); color: var(--text-primary); }.nav-item.active { border-color: var(--border-bright); background: linear-gradient(90deg, rgba(97,231,210,.12), rgba(97,231,210,.035)); color: var(--color-primary); box-shadow: inset 3px 0 0 var(--color-primary); }
.nav-icon { display: flex; width: 18px; height: 18px; flex-shrink: 0; }.nav-icon svg { width: 100%; height: 100%; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.6; }.nav-label { font-size: 13px; font-weight: 600; }.nav-hint { margin-left: auto; color: var(--text-tertiary); font-family: var(--font-mono); font-size: 9px; letter-spacing: .04em; }.nav-item.active .nav-hint { color: var(--color-primary-text); }.nav-signal { width: 5px; height: 5px; margin-left: 1px; border-radius: 50%; background: currentColor; box-shadow: 0 0 8px currentColor; }
.sidebar-divider { height: 1px; margin: auto 20px 16px; background: var(--border-color); }.sidebar-footer { display: flex; align-items: center; gap: 8px; padding: 0 16px 18px; }.user-info { display: flex; align-items: center; gap: 10px; min-width: 0; flex: 1; }.user-avatar { display: flex; align-items: center; justify-content: center; width: 30px; height: 30px; flex-shrink: 0; border: 1px solid var(--border-bright); border-radius: 8px; background: var(--color-primary-bg); color: var(--color-primary); font-family: var(--font-mono); font-size: 12px; font-weight: 600; }.user-details { min-width: 0; }.user-name { overflow: hidden; color: var(--text-primary); font-size: 12px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }.user-role { display: flex; align-items: center; gap: 5px; margin-top: 2px; color: var(--text-tertiary); font-family: var(--font-mono); font-size: 9px; }.online-dot { width: 4px; height: 4px; border-radius: 50%; background: var(--color-primary); }.logout-btn { display: flex; align-items: center; justify-content: center; width: 30px; height: 30px; border: 1px solid var(--border-color); border-radius: 7px; background: transparent; color: var(--text-tertiary); cursor: pointer; transition: all var(--transition-fast); }.logout-btn:hover { border-color: var(--color-danger); color: var(--color-danger); }.logout-btn svg { width: 15px; height: 15px; fill: none; stroke: currentColor; stroke-linecap: round; stroke-linejoin: round; stroke-width: 1.6; }

@media (max-width: 760px) { .sidebar { position: fixed; z-index: 20; width: min(248px, 86vw); transform: translateX(-100%); box-shadow: 24px 0 60px rgba(0,0,0,.3); }.sidebar.mobile-open { transform: translateX(0); } }
</style>
