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
  if (!username.value || !password.value) { message.warning('请填写用户名和密码'); return }
  loading.value = true
  try {
    if (isLogin.value) {
      const res = await authApi.login({ username: username.value, password: password.value })
      authStore.setAuth(res.data.token, res.data.user)
      message.success('登录成功')
      router.push('/')
    } else {
      await authApi.register({ username: username.value, password: password.value })
      message.success('注册成功，请登录')
      isLogin.value = true
      password.value = ''
    }
  } catch (e: unknown) {
    message.error((e as Error).message || '操作失败')
  } finally { loading.value = false }
}
</script>

<template>
  <div class="login-page">
    <div class="login-grid" aria-hidden="true"></div>
    <div class="login-orbit orbit-one"></div><div class="login-orbit orbit-two"></div>
    <section class="login-signal">
      <div class="login-brand"><span class="brand-mark"><i></i><i></i><i></i></span><strong>NetPulse</strong></div>
      <div class="signal-kicker"><span></span> NETWORK QUALITY OBSERVATORY</div>
      <h1>Read the<br /><em>signal.</em></h1>
      <p>每一次连接都有一条轨迹。用可读的遥测数据，判断网络真正发生了什么。</p>
      <div class="terminal-card">
        <div class="terminal-top"><span><i></i><i></i><i></i></span><small>probe://signal-room</small><b>LIVE</b></div>
        <div class="terminal-line"><span>01</span><i>resolve</i><b>42ms</b></div>
        <div class="terminal-line"><span>02</span><i>connect</i><b>18ms</b></div>
        <div class="terminal-line"><span>03</span><i>first byte</i><b class="warm">86ms</b></div>
        <div class="terminal-trace"><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span><span></span></div>
      </div>
    </section>
    <section class="login-card">
      <div class="login-card-header"><span class="section-eyebrow">{{ isLogin ? 'operator access' : 'new operator' }}</span><div class="login-status"><span></span> SECURE</div></div>
      <h2>{{ isLogin ? '进入信号室' : '创建操作员' }}</h2>
      <p class="login-subtitle">{{ isLogin ? '登录以查看你的网络质量态势。' : '创建一个新的 NetPulse 操作员账号。' }}</p>
      <form @submit.prevent="handleSubmit">
        <div class="form-group"><label class="form-label" for="username">操作员名称</label><div class="input-wrap"><span>@</span><input id="username" v-model="username" type="text" placeholder="输入用户名" :disabled="loading" autocomplete="username" /></div></div>
        <div class="form-group"><label class="form-label" for="password">访问密钥</label><div class="input-wrap"><span>⌁</span><input id="password" v-model="password" type="password" placeholder="输入密码" :disabled="loading" autocomplete="current-password" /></div></div>
        <button class="submit-btn" type="submit" :disabled="loading"><span>{{ loading ? '验证中...' : (isLogin ? '进入信号室' : '创建账号') }}</span><b>↗</b></button>
      </form>
      <button class="switch-btn" type="button" @click="isLogin = !isLogin; password = ''">{{ isLogin ? '没有操作员账号？创建一个' : '已有账号？返回登录' }} <span>→</span></button>
      <div class="login-footer"><span>NETPULSE WEB</span><span>AUTH / 01</span><span>ENCRYPTED SESSION</span></div>
    </section>
  </div>
</template>

<style scoped>
.login-page { position: relative; display: grid; grid-template-columns: minmax(320px, 1fr) minmax(380px, 470px); align-items: center; gap: clamp(40px, 9vw, 140px); min-height: 100vh; overflow: hidden; padding: 54px clamp(28px, 8vw, 140px); background: #08111f; color: #edf6ff; }.login-grid { position: absolute; inset: 0; opacity: .7; background-image: linear-gradient(rgba(97,231,210,.045) 1px, transparent 1px), linear-gradient(90deg, rgba(97,231,210,.045) 1px, transparent 1px); background-size: 42px 42px; mask-image: linear-gradient(90deg, black, transparent 82%); }.login-orbit { position: absolute; border: 1px solid rgba(97,231,210,.12); border-radius: 50%; pointer-events: none; }.orbit-one { top: -20%; left: 30%; width: 720px; height: 720px; }.orbit-two { top: -6%; left: 37%; width: 500px; height: 500px; border-color: rgba(136,169,255,.1); }.login-signal, .login-card { position: relative; z-index: 1; }.login-signal { max-width: 570px; justify-self: end; }.login-brand { display: flex; align-items: center; gap: 11px; color: #edf6ff; font-family: var(--font-display); font-size: 18px; }.brand-mark { display: flex; align-items: flex-end; gap: 3px; width: 26px; height: 26px; padding: 4px; border: 1px solid rgba(97,231,210,.45); border-radius: 7px; background: rgba(97,231,210,.08); }.brand-mark i { display: block; width: 3px; border-radius: 2px; background: #61e7d2; box-shadow: 0 0 8px rgba(97,231,210,.7); }.brand-mark i:nth-child(1) { height: 7px; opacity: .6; }.brand-mark i:nth-child(2) { height: 14px; }.brand-mark i:nth-child(3) { height: 10px; opacity: .8; }.signal-kicker { display: flex; align-items: center; gap: 8px; margin-top: clamp(70px, 12vh, 130px); color: #61e7d2; font-family: var(--font-mono); font-size: 9px; letter-spacing: .15em; }.signal-kicker span { width: 18px; height: 1px; background: #61e7d2; box-shadow: 0 0 8px #61e7d2; }.login-signal h1 { margin-top: 17px; font-family: var(--font-display); font-size: clamp(58px, 7vw, 92px); font-weight: 500; letter-spacing: -.085em; line-height: .87; }.login-signal h1 em { color: #61e7d2; font-style: normal; }.login-signal > p { max-width: 370px; margin-top: 24px; color: #9bb0c2; font-size: 14px; line-height: 1.8; }.terminal-card { width: min(100%, 420px); margin-top: 48px; padding: 15px 17px 17px; border: 1px solid rgba(136,169,201,.2); border-radius: 10px; background: rgba(15,32,50,.76); box-shadow: 0 20px 60px rgba(0,0,0,.25); }.terminal-top, .terminal-line { display: flex; align-items: center; }.terminal-top { justify-content: space-between; padding-bottom: 12px; border-bottom: 1px solid rgba(136,169,201,.14); }.terminal-top > span { display: flex; gap: 4px; }.terminal-top i { width: 5px; height: 5px; border-radius: 50%; background: #61788e; }.terminal-top i:first-child { background: #ff8a65; }.terminal-top small, .terminal-top b { color: #61788e; font-family: var(--font-mono); font-size: 8px; font-weight: 400; }.terminal-top b { color: #61e7d2; }.terminal-line { gap: 13px; padding-top: 11px; font-family: var(--font-mono); font-size: 10px; }.terminal-line span { color: #61788e; }.terminal-line i { flex: 1; color: #9bb0c2; font-style: normal; }.terminal-line b { color: #61e7d2; font-weight: 500; }.terminal-line b.warm { color: #ffb454; }.terminal-trace { display: flex; align-items: flex-end; gap: 4px; height: 24px; margin-top: 15px; padding-top: 6px; border-top: 1px solid rgba(136,169,201,.12); }.terminal-trace span { flex: 1; height: 7px; border-radius: 2px; background: #61e7d2; opacity: .34; }.terminal-trace span:nth-child(2) { height: 13px; }.terminal-trace span:nth-child(3) { height: 9px; }.terminal-trace span:nth-child(4) { height: 18px; opacity: .72; }.terminal-trace span:nth-child(5) { height: 11px; }.terminal-trace span:nth-child(6) { height: 21px; }.terminal-trace span:nth-child(7) { height: 14px; opacity: .6; }.terminal-trace span:nth-child(8) { height: 9px; }.terminal-trace span:nth-child(9) { height: 16px; opacity: .8; }
.login-card { width: 100%; padding: clamp(27px, 4vw, 42px); border: 1px solid rgba(136,169,201,.23); border-radius: 15px; background: rgba(15,32,50,.86); box-shadow: 0 30px 80px rgba(0,0,0,.28), 0 0 0 1px rgba(97,231,210,.04); backdrop-filter: blur(18px); }.login-card-header { display: flex; align-items: center; justify-content: space-between; }.section-eyebrow { color: #61e7d2; font-family: var(--font-mono); font-size: 9px; letter-spacing: .14em; text-transform: uppercase; }.login-status { display: flex; align-items: center; gap: 6px; color: #61788e; font-family: var(--font-mono); font-size: 8px; letter-spacing: .08em; }.login-status span { width: 5px; height: 5px; border-radius: 50%; background: #61e7d2; box-shadow: 0 0 8px #61e7d2; }.login-card h2 { margin-top: 39px; color: #edf6ff; font-family: var(--font-display); font-size: 28px; font-weight: 600; letter-spacing: -.05em; }.login-subtitle { margin-top: 8px; color: #9bb0c2; font-size: 12px; }.login-card form { margin-top: 31px; }.form-group { margin-bottom: 17px; }.form-label { display: block; margin-bottom: 7px; color: #9bb0c2; font-family: var(--font-mono); font-size: 10px; }.input-wrap { display: flex; align-items: center; height: 44px; padding: 0 13px; border: 1px solid rgba(136,169,201,.22); border-radius: 6px; background: #0a1828; transition: border-color var(--transition-fast), box-shadow var(--transition-fast); }.input-wrap:focus-within { border-color: #61e7d2; box-shadow: 0 0 0 3px rgba(97,231,210,.1), 0 0 20px rgba(97,231,210,.08); }.input-wrap > span { width: 23px; color: #61e7d2; font-family: var(--font-mono); font-size: 14px; }.input-wrap input { width: 100%; height: 100%; border: 0; outline: 0; background: transparent; color: #edf6ff; font-size: 13px; }.input-wrap input::placeholder { color: #61788e; }.submit-btn { display: flex; align-items: center; justify-content: space-between; width: 100%; height: 45px; margin-top: 25px; padding: 0 15px 0 17px; border: 1px solid #61e7d2; border-radius: 6px; background: #61e7d2; color: #06201f; cursor: pointer; font-size: 12px; font-weight: 700; transition: all var(--transition-fast); }.submit-btn:hover:not(:disabled) { background: #9af5e5; box-shadow: 0 0 28px rgba(97,231,210,.2); }.submit-btn b { font-family: var(--font-mono); font-size: 16px; }.submit-btn:disabled { opacity: .55; cursor: wait; }.switch-btn { margin-top: 21px; border: 0; background: transparent; color: #9bb0c2; cursor: pointer; font-size: 11px; }.switch-btn:hover { color: #61e7d2; }.switch-btn span { margin-left: 5px; color: #61e7d2; }.login-footer { display: flex; justify-content: space-between; gap: 8px; margin-top: 41px; padding-top: 14px; border-top: 1px solid rgba(136,169,201,.14); color: #61788e; font-family: var(--font-mono); font-size: 7px; letter-spacing: .06em; }
@media (max-width: 850px) { .login-page { grid-template-columns: 1fr; max-width: 600px; margin: 0 auto; padding: 32px 22px 42px; }.login-signal { justify-self: start; }.signal-kicker { margin-top: 65px; }.terminal-card { margin-top: 30px; }.login-card { margin-top: 12px; } }
@media (max-width: 520px) { .login-page { display: block; min-height: 100vh; padding: 28px 16px 32px; }.login-signal h1 { font-size: 62px; }.signal-kicker { margin-top: 56px; }.login-signal > p { font-size: 13px; }.terminal-card { margin-top: 28px; }.login-card { margin-top: 22px; padding: 25px 20px; }.login-card h2 { margin-top: 31px; }.login-footer { flex-wrap: wrap; }.login-footer span:last-child { width: 100%; } }
</style>
