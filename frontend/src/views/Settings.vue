<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useDialog, useMessage } from 'naive-ui'
import { useAuthStore } from '@/stores/auth'
import { useDark } from '@/utils/theme'
import http, { getErrorMessage } from '@/api/index'
import { videoCookieApi, type VideoCookieBackup, type VideoCookieStatus } from '@/api/video-cookie'

const router = useRouter()
const message = useMessage()
const authStore = useAuthStore()
const { isDark, toggleDark } = useDark()

interface User {
  id: string
  username: string
  role: string
}

const users = ref<User[]>([])
const loading = ref(false)
const isAdmin = computed(() => authStore.isAdmin)
const cookiePlatforms = ref<string[]>([])
const cookieStatuses = ref<VideoCookieStatus[]>([])
const cookieLoading = ref(false)
const cookieImporting = ref(false)
const cookieBackupImporting = ref(false)
const cookieStatusMap = computed(() => new Map(cookieStatuses.value.map(item => [item.platform, item])))

const platformLabels: Record<string, string> = {
  youtube: 'YouTube',
  bilibili: 'Bilibili',
  sohu: '搜狐视频',
  youku: '优酷',
  mgtv: '芒果TV',
  pptv: 'PPTV',
  qq: '腾讯视频',
}

async function fetchUsers() {
  loading.value = true
  try {
    const res = await http.get('/admin/users')
    users.value = res.data || []
  } catch (e: any) {
    message.error(e.message || '加载用户列表失败')
  } finally {
    loading.value = false
  }
}

async function fetchCookieStatus() {
  if (!isAdmin.value) return
  cookieLoading.value = true
  try {
    const res = await videoCookieApi.list()
    cookiePlatforms.value = res.data.platforms
    cookieStatuses.value = res.data.cookies
  } catch (e: unknown) {
    message.error(getErrorMessage(e, '加载视频 Cookie 状态失败'))
  } finally {
    cookieLoading.value = false
  }
}

async function handleCookieImport(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  cookieImporting.value = true
  try {
    const parsed = JSON.parse(await file.text())
    const cookies = extractCookieArray(parsed)
    if (!cookies) throw new Error('Cookie JSON 必须是数组，或包含 cookies 数组字段')
    const platform = detectCookiePlatform(cookies)
    if (!platform) throw new Error('无法从 Cookie JSON 推断平台，请确认文件来自受支持的视频站点')
    await videoCookieApi.importJson(platform, cookies)
    message.success(`${platformLabels[platform] || platform} Cookie 已加密保存`)
    await fetchCookieStatus()
  } catch (e: unknown) {
    message.error(getErrorMessage(e, 'Cookie 导入失败'))
  } finally {
    cookieImporting.value = false
  }
}

async function handleBackupImport(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  cookieBackupImporting.value = true
  try {
    const backup = JSON.parse(await file.text()) as VideoCookieBackup
    if (!backup.encrypted_payload || !backup.platform) throw new Error('不是有效的视频 Cookie 加密备份')
    await videoCookieApi.importBackup(backup)
    message.success('视频 Cookie 加密备份已恢复')
    await fetchCookieStatus()
  } catch (e: unknown) {
    message.error(getErrorMessage(e, '加密备份恢复失败'))
  } finally {
    cookieBackupImporting.value = false
  }
}

function extractCookieArray(value: unknown): unknown[] | null {
  if (Array.isArray(value)) return value
  if (value && typeof value === 'object' && Array.isArray((value as { cookies?: unknown }).cookies)) {
    return (value as { cookies: unknown[] }).cookies
  }
  return null
}

function detectCookiePlatform(value: unknown[]): string | null {
  if (value.length === 0) return null
  const domains = value
    .map(item => item && typeof item === 'object' && typeof (item as { domain?: unknown }).domain === 'string'
      ? (item as { domain: string }).domain.toLowerCase()
      : '')
    .join(' ')
  if (domains.includes('youtube.com') || domains.includes('google.com')) return 'youtube'
  if (domains.includes('bilibili.com') || domains.includes('b23.tv')) return 'bilibili'
  if (domains.includes('sohu.com')) return 'sohu'
  if (domains.includes('youku.com') || domains.includes('tudou.com')) return 'youku'
  if (domains.includes('mgtv.com') || domains.includes('hunantv.com')) return 'mgtv'
  if (domains.includes('pptv.com') || domains.includes('pps.tv')) return 'pptv'
  if (domains.includes('qq.com')) return 'qq'
  return null
}

async function exportCookieBackup(platform: string) {
  try {
    const res = await videoCookieApi.exportBackup(platform)
    const blob = new Blob([JSON.stringify(res.data, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = `netpulse-${platform}-cookie-backup.json`
    anchor.click()
    URL.revokeObjectURL(url)
    message.success('加密备份已导出，文件不能脱离原加密密钥使用')
  } catch (e: unknown) {
    message.error(getErrorMessage(e, '加密备份导出失败'))
  }
}

async function removeCookies(platform: string) {
  try {
    await videoCookieApi.remove(platform)
    message.success('视频 Cookie 已删除')
    await fetchCookieStatus()
  } catch (e: unknown) {
    message.error(getErrorMessage(e, '删除 Cookie 失败'))
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

async function deleteUser(userId: string, username: string) {
  const dialog = useDialog()
  dialog.warning({
    title: '删除用户',
    content: `确认删除用户「${username}」？此操作不可撤销。`,
    positiveText: '确认删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await http.delete(`/admin/users/${userId}`)
        message.success(`用户「${username}」已删除`)
        fetchUsers()
      } catch (e: any) {
        message.error(e.message || '删除失败')
      }
    },
  })
}

onMounted(() => {
  if (isAdmin.value) {
    fetchUsers()
    fetchCookieStatus()
  }
})
</script>

<template>
  <div class="settings-page">
    <div class="settings-section">
      <h2 class="section-title">个人信息</h2>
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
      <h2 class="section-title">外观</h2>
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
        <h2 class="section-title">用户管理</h2>
        <button class="refresh-btn" :disabled="loading" @click="fetchUsers">刷新</button>
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
          <div class="user-actions">
            <select
              class="role-select"
              :value="u.role"
              @change="(e: any) => updateRole(u.id, e.target.value)"
            >
              <option value="user">普通用户</option>
              <option value="admin">管理员</option>
            </select>
            <button class="delete-btn" @click="deleteUser(u.id, u.username)" title="删除用户">
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none"><path d="M2 4h10M5 4V2.5A.5.5 0 0 1 5.5 2h3a.5.5 0 0 1 .5.5V4m1 0v7.5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            </button>
          </div>
        </div>
      </div>
    </div>

    <div v-if="isAdmin" class="settings-section">
      <div class="section-header">
        <div>
          <h2 class="section-title">视频登录态</h2>
          <p class="section-desc">导入浏览器扩展导出的 Cookie JSON。系统按平台白名单校验后加密保存，只在对应视频站点注入。</p>
        </div>
        <button class="refresh-btn" :disabled="cookieLoading" @click="fetchCookieStatus">刷新</button>
      </div>
      <div class="cookie-actions">
        <label class="file-btn" :class="{ disabled: cookieImporting }">
          {{ cookieImporting ? '导入中...' : '导入浏览器 Cookie JSON' }}
          <input type="file" accept=".json,application/json" :disabled="cookieImporting" @change="handleCookieImport" />
        </label>
        <label class="file-btn secondary" :class="{ disabled: cookieBackupImporting }">
          {{ cookieBackupImporting ? '恢复中...' : '恢复加密备份' }}
          <input type="file" accept=".json,application/json" :disabled="cookieBackupImporting" @change="handleBackupImport" />
        </label>
      </div>
      <div v-if="cookieLoading" class="loading-text">加载中...</div>
      <div v-else class="cookie-list">
        <div v-for="platform in cookiePlatforms" :key="platform" class="cookie-row">
          <div>
            <div class="cookie-platform">{{ platformLabels[platform] || platform }}</div>
            <div v-if="cookieStatusMap.get(platform)" class="cookie-meta">
              {{ cookieStatusMap.get(platform)?.cookie_count }} 个 Cookie · {{ cookieStatusMap.get(platform)?.domains.join(', ') }}
            </div>
            <div v-else class="cookie-meta">未配置登录态，任务将匿名播放</div>
          </div>
          <div class="cookie-row-actions" v-if="cookieStatusMap.get(platform)">
            <button class="refresh-btn" @click="exportCookieBackup(platform)">导出加密备份</button>
            <button class="delete-btn" title="删除登录态" @click="removeCookies(platform)">删除</button>
          </div>
        </div>
      </div>
      <p class="security-note">原始 Cookie 不会回传展示，也不会写入任务日志。加密密钥保存在容器的 `/app/storage`，请和数据库一起备份并限制目录权限。</p>
    </div>

    <div class="settings-section">
      <h2 class="section-title">账户</h2>
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
.section-desc { margin: -8px 0 0; color: var(--text-tertiary); font-size: 12px; line-height: 1.6; max-width: 560px; }
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
.user-avatar { width: 36px; height: 36px; border-radius: 50%; background: var(--color-primary); color: white; display: flex; align-items: center; justify-content: center; font-weight: 600; font-size: 14px; }
.user-name { font-size: 13px; font-weight: 500; }
.user-role-text { font-size: 11px; color: var(--text-tertiary); }
.role-select { height: 32px; padding: 0 10px; border: 1px solid var(--border-color); border-radius: var(--radius-sm); background: var(--bg-card); color: var(--text-primary); font-size: 13px; cursor: pointer; }
.user-actions { display: flex; align-items: center; gap: 8px; }
.delete-btn {
  width: 32px; height: 32px; display: flex; align-items: center; justify-content: center;
  border: 1px solid transparent; background: transparent; border-radius: var(--radius-sm);
  cursor: pointer; transition: all var(--transition-fast);
}
.delete-btn:hover { background: rgba(208,48,80,0.1); color: var(--color-danger); }
.danger-btn { height: 36px; padding: 0 16px; border: 1px solid var(--color-danger); background: var(--bg-card); color: var(--color-danger); border-radius: var(--radius-sm); cursor: pointer; font-size: 13px; }
.danger-btn:hover { background: var(--color-danger); color: white; }
.cookie-actions { display: flex; flex-wrap: wrap; gap: 10px; margin-bottom: 16px; }
.file-btn { display: inline-flex; align-items: center; height: 34px; padding: 0 14px; border-radius: var(--radius-sm); background: var(--color-primary); color: #fff; cursor: pointer; font-size: 13px; }
.file-btn.secondary { background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-color); }
.file-btn.disabled { opacity: .6; pointer-events: none; }
.file-btn input { display: none; }
.cookie-list { display: flex; flex-direction: column; gap: 8px; }
.cookie-row { display: flex; align-items: center; justify-content: space-between; gap: 14px; padding: 12px 14px; background: var(--bg-body); border-radius: var(--radius-md); }
.cookie-platform { font-size: 13px; font-weight: 600; }
.cookie-meta { color: var(--text-tertiary); font-size: 11px; margin-top: 4px; word-break: break-word; }
.cookie-row-actions { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
.cookie-row-actions .delete-btn { width: auto; padding: 0 10px; color: var(--color-danger); border: 1px solid rgba(208,48,80,.35); font-size: 12px; }
.security-note { margin: 16px 0 0; color: var(--text-tertiary); font-size: 11px; line-height: 1.6; }
.switch { position: relative; display: inline-block; width: 44px; height: 24px; flex-shrink: 0; }
.switch input { opacity: 0; width: 0; height: 0; }
.slider { position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background: var(--border-color); border-radius: 24px; transition: var(--transition-fast); }
.slider::before { content: ''; position: absolute; height: 18px; width: 18px; left: 3px; bottom: 3px; background: white; border-radius: 50%; transition: var(--transition-fast); }
.switch input:checked + .slider { background: var(--color-primary); }
.switch input:checked + .slider::before { transform: translateX(20px); }
@media (max-width: 640px) {
  .cookie-row { align-items: flex-start; flex-direction: column; }
  .cookie-row-actions { width: 100%; }
  .cookie-row-actions .refresh-btn, .cookie-row-actions .delete-btn { flex: 1; }
}
</style>

<style scoped>
.settings-page { max-width:900px; }.settings-section { position:relative; overflow:hidden; border-color:var(--border-color); border-radius:10px; background:linear-gradient(145deg,var(--bg-card),rgba(15,32,50,.76)); box-shadow:var(--shadow-card); }.settings-section::before { position:absolute; top:0; left:0; width:2px; height:36px; background:var(--color-primary); content:''; }.section-title { color:var(--text-primary); font-family:var(--font-display); letter-spacing:-.03em; }.section-desc,.setting-desc,.cookie-meta,.security-note { color:var(--text-secondary); }.info-item,.user-row,.cookie-row { border:1px solid var(--border-color); background:var(--bg-alt); }.info-label { color:var(--text-tertiary); font-family:var(--font-mono); font-size:10px; }.user-avatar { border:1px solid var(--border-bright); background:var(--color-primary-bg); color:var(--color-primary); }.user-role-text { color:var(--text-tertiary); font-family:var(--font-mono); font-size:9px; }.refresh-btn,.role-select { border-color:var(--border-color); border-radius:5px; background:var(--bg-input); color:var(--text-secondary); font-family:var(--font-mono); font-size:10px; }.refresh-btn:hover { border-color:var(--border-color-hover); background:var(--bg-hover); }.file-btn { border:1px solid var(--color-primary); border-radius:5px; background:var(--color-primary); color:#06201f; font-family:var(--font-mono); font-size:10px; }.file-btn.secondary { border-color:var(--border-color); background:transparent; color:var(--text-secondary); }.file-btn.secondary:hover { border-color:var(--border-color-hover); }.danger-btn { border-color:var(--color-danger); border-radius:5px; background:transparent; color:var(--color-danger); font-family:var(--font-mono); font-size:10px; }.danger-btn:hover { background:var(--color-danger); color:#27120d; }.switch input:checked + .slider { background:var(--color-primary); }
</style>
