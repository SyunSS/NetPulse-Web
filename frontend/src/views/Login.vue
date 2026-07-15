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
  background: var(--n-color-body);
}
.login-card {
  width: 420px;
  padding: 40px;
  border-radius: 12px;
  background: var(--n-color-card);
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.08);
}
.login-header {
  text-align: center;
  margin-bottom: 32px;
}
.logo-text {
  font-size: 28px;
  font-weight: 700;
  background: linear-gradient(135deg, #2080f0 0%, #18a058 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}
.subtitle {
  margin-top: 8px;
  color: var(--n-text-color-3);
  font-size: 14px;
}
.form-group {
  margin-bottom: 16px;
}
.form-label {
  display: block;
  margin-bottom: 6px;
  font-size: 14px;
  color: var(--n-text-color-2);
}
.form-input {
  width: 100%;
  height: 40px;
  padding: 0 12px;
  font-size: 14px;
  border: 1px solid var(--n-border-color);
  border-radius: 4px;
  background: var(--n-color-input);
  color: var(--n-text-color);
  transition: border-color 0.2s;
  box-sizing: border-box;
}
.form-input:focus {
  outline: none;
  border-color: #2080f0;
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
  border-radius: 4px;
  background: #2080f0;
  color: white;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: opacity 0.2s;
}
.submit-btn:hover:not(:disabled) {
  opacity: 0.9;
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
  color: #2080f0;
  cursor: pointer;
  font-size: 14px;
  padding: 4px 8px;
}
.link-btn:hover {
  text-decoration: underline;
}
</style>
