<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { authApi } from '@/api/auth'
import { useAuthStore } from '@/stores/auth'

const router = useRouter()
const message = useMessage()
const authStore = useAuthStore()

const isLogin = ref(true)
const username = ref('')
const password = ref('')
const loading = ref(false)

async function handleSubmit() {
  if (!username.value || !password.value) {
    message.warning('请填写用户名和密码')
    return
  }

  loading.value = true
  try {
    if (isLogin.value) {
      const res = await authApi.login({
        username: username.value,
        password: password.value,
      })
      authStore.setAuth(res.data.token, res.data.user)
      message.success('登录成功')
      router.push('/')
    } else {
      await authApi.register({
        username: username.value,
        password: password.value,
      })
      message.success('注册成功，请登录')
      isLogin.value = true
      password.value = ''
    }
  } catch (e: unknown) {
    const err = e as Error
    message.error(err.message || '操作失败')
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="login-page">
    <div class="login-card">
      <div class="login-header">
        <div class="logo-text">NetPulse Web</div>
        <p class="subtitle">网络质量测试平台</p>
      </div>

      <div class="form-group">
        <label class="form-label">用户名</label>
        <input
          v-model="username"
          type="text"
          class="form-input"
          placeholder="请输入用户名"
          :disabled="loading"
          autocomplete="username"
        />
      </div>

      <div class="form-group">
        <label class="form-label">密码</label>
        <input
          v-model="password"
          type="password"
          class="form-input"
          placeholder="请输入密码"
          :disabled="loading"
          autocomplete="current-password"
          @keyup.enter="handleSubmit"
        />
      </div>

      <button
        type="button"
        class="submit-btn"
        :disabled="loading"
        @click="handleSubmit"
      >
        {{ loading ? '处理中...' : (isLogin ? '登录' : '注册') }}
      </button>

      <div class="login-footer">
        <button
          type="button"
          class="link-btn"
          @click="isLogin = !isLogin; password = ''"
        >
          {{ isLogin ? '没有账号？去注册' : '已有账号？去登录' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.login-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-alt);
}
.login-card {
  width: 420px;
  padding: 40px;
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  box-shadow: var(--shadow-card);
}
.login-header {
  text-align: center;
  margin-bottom: 32px;
}
.logo-text {
  font-size: 24px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -0.5px;
}
.subtitle {
  margin-top: 8px;
  color: var(--text-secondary);
  font-size: 14px;
}
.form-group {
  margin-bottom: 16px;
}
.form-label {
  display: block;
  margin-bottom: 6px;
  font-size: 14px;
  color: var(--text-secondary);
}
.form-input {
  width: 100%;
  height: 40px;
  padding: 0 12px;
  font-size: 14px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text-primary);
  transition: border-color 0.2s;
  box-sizing: border-box;
}
.form-input:focus {
  outline: none;
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-bg);
}
.form-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.submit-btn {
  width: 100%;
  height: 40px;
  margin-top: 8px;
  border: none;
  border-radius: var(--radius-sm);
  background: var(--color-primary);
  color: white;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s;
}
.submit-btn:hover:not(:disabled) {
  background: var(--color-primary-active);
}
.submit-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.login-footer {
  text-align: center;
  margin-top: 16px;
}
.link-btn {
  background: none;
  border: none;
  color: var(--color-primary);
  cursor: pointer;
  font-size: 14px;
  padding: 4px 8px;
}
.link-btn:hover {
  text-decoration: underline;
}
</style>
