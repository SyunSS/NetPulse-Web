<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { useAuthStore } from '@/stores/auth'
import { useDark } from '@/utils/theme'

const router = useRouter()
const message = useMessage()
const authStore = useAuthStore()
const { isDark, toggleDark } = useDark()
import http from '@/api/index'

interface User {
  id: string
  username: string
  role: string
}

const users = ref<User[]>([])
const loading = ref(false)
const isAdmin = authStore.isAdmin

async function fetchUsers() {
  loading.value = true
  try {
    const res = await http.get('/admin/users')
    users.value = res.data.data || []
  } catch (e: any) {
    message.error(e.message || '加载用户列表失败')
  } finally {
    loading.value = false
  }
}

async function updateRole(userId: string, role: string) {
  try {
    await http.post('/admin/users/role', { user_id: userId, role })
    message.success('权限已更新')
    fetchUsers()
  } catch (e: any) {
    message.error(e.message || '更新失败')
  }
}

onMounted(() => {
  if (isAdmin) fetchUsers()
})
</script>

<template>
  <div class="settings-page">
    <div class="settings-section">
      <h2 class="section-title">👤 个人信息</h2>
      <div class="info-grid">
        <div class="info-item">
          <span class="info-label">用户名</span>
          <strong>{{ authStore.user?.username }}</strong>
        </div>
        <div class="info-item">
          <span class="info-label">角色</span>
          <span class="status-tag" :class="isAdmin ? 'status-completed' : 'status-pending'">
            {{ isAdmin ? '管理员' : '普通用户' }}
          </span>
        </div>
      </div>
    </div>

    <div class="settings-section">
      <h2 class="section-title">🎨 外观</h2>
      <div class="setting-row">
        <div>
          <div class="setting-label">深色模式</div>
          <div class="setting-desc">切换浅色/深色主题</div>
        </div>
        <label class="switch">
          <input type="checkbox" :checked="isDark" @change="toggleDark" />
          <span class="slider"></span>
        </label>
      </div>
    </div>

    <div v-if="isAdmin" class="settings-section">
      <div class="section-header">
        <h2 class="section-title">👥 用户管理</h2>
        <button class="refresh-btn" :disabled="loading" @click="fetchUsers">↻ 刷新</button>
      </div>
      <div v-if="loading" class="loading-text">加载中...</div>
      <div v-else-if="users.length === 0" class="empty-text">暂无用户</div>
      <div v-else class="user-list">
        <div v-for="u in users" :key="u.id" class="user-row">
          <div class="user-info">
            <div class="user-avatar">{{ u.username.charAt(0).toUpperCase() }}</div>
            <div>
              <div class="user-name">{{ u.username }}</div>
              <div class="user-role-text">{{ u.role === 'admin' ? '管理员' : '普通用户' }}</div>
            </div>
          </div>
          <select
            class="role-select"
            :value="u.role"
            @change="(e: any) => updateRole(u.id, e.target.value)"
          >
            <option value="user">普通用户</option>
            <option value="admin">管理员</option>
          </select>
        </div>
      </div>
    </div>

    <div class="settings-section">
      <h2 class="section-title">🔐 账户</h2>
      <button class="danger-btn" @click="authStore.logout(); router.push('/login')">
        退出登录
      </button>
    </div>
  </div>
</template>

<style scoped>
.settings-page { max-width: 800px; margin: 0 auto; }
.settings-section {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  padding: 24px;
  margin-bottom: 16px;
}
.section-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 16px; }
.section-title { font-size: 16px; font-weight: 600; margin: 0 0 16px 0; }
.section-header .section-title { margin-bottom: 0; }
.info-grid { display: flex; flex-direction: column; gap: 12px; }
.info-item { display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; background: var(--bg-body); border-radius: var(--radius-md); }
.info-label { color: var(--text-secondary); font-size: 13px; }
.setting-row { display: flex; align-items: center; justify-content: space-between; }
.setting-label { font-weight: 500; }
.setting-desc { font-size: 12px; color: var(--text-tertiary); margin-top: 2px; }
.refresh-btn {
  height: 32px; padding: 0 14px;
  border: 1px solid var(--border-color); background: var(--bg-card); color: var(--text-secondary);
  border-radius: var(--radius-sm); cursor: pointer; font-size: 13px;
}
.loading-text, .empty-text { text-align: center; padding: 20px; color: var(--text-tertiary); font-size: 13px; }
.user-list { display: flex; flex-direction: column; gap: 8px; }
.user-row { display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; background: var(--bg-body); border-radius: var(--radius-md); }
.user-info { display: flex; align-items: center; gap: 10px; }
.user-avatar { width: 36px; height: 36px; border-radius: 50%; background: var(--gradient-primary); color: white; display: flex; align-items: center; justify-content: center; font-weight: 600; font-size: 14px; }
.user-name { font-size: 13px; font-weight: 500; }
.user-role-text { font-size: 11px; color: var(--text-tertiary); }
.role-select { height: 32px; padding: 0 10px; border: 1px solid var(--border-color); border-radius: var(--radius-sm); background: var(--bg-card); color: var(--text-primary); font-size: 13px; cursor: pointer; }
.danger-btn { height: 36px; padding: 0 16px; border: 1px solid var(--color-danger); background: var(--bg-card); color: var(--color-danger); border-radius: var(--radius-md); cursor: pointer; font-size: 13px; }
.danger-btn:hover { background: var(--color-danger); color: white; }
.switch { position: relative; display: inline-block; width: 44px; height: 24px; flex-shrink: 0; }
.switch input { opacity: 0; width: 0; height: 0; }
.slider { position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background: var(--border-color); border-radius: 24px; transition: var(--transition-fast); }
.slider::before { content: ''; position: absolute; height: 18px; width: 18px; left: 3px; bottom: 3px; background: white; border-radius: 50%; transition: var(--transition-fast); }
.switch input:checked + .slider { background: var(--color-primary); }
.switch input:checked + .slider::before { transform: translateX(20px); }
</style>
