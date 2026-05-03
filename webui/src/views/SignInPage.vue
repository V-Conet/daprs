<script setup lang="ts">
import { onMounted, ref } from 'vue'

import { fetchMe, loginUrl } from '../lib/api'
import { unauthorized, useThemeToggle } from '../lib/page'

const error = ref('')
const loggedIn = ref(false)
const { themeLabel, toggleTheme } = useThemeToggle()

onMounted(async () => {
  try {
    await fetchMe()
    loggedIn.value = true
  } catch (err) {
    const message = err instanceof Error ? err.message : '请求失败'
    if (!unauthorized(message)) {
      error.value = message
    }
  }
})
</script>

<template>
  <div class="shell signin-shell">
    <header class="site-header card">
      <div class="site-header-inner">
        <a class="logo" href="/signin.html">dn42</a>
        <div class="header-right">
          <button class="button button-ghost" type="button" @click="toggleTheme">{{ themeLabel }}</button>
        </div>
      </div>
    </header>

    <section class="hero card">
      <div class="hero-copy">
        <p class="eyebrow">Stage 1</p>
        <h2>登录</h2>
        <p>使用后端现有 OIDC 登录流程。该页只负责触发认证，不承载其它业务逻辑。</p>
      </div>
      <div class="hero-panel">
        <div class="session-card" v-if="loggedIn">
          <span class="status-dot status-dot-ok"></span>
          <div>
            <p>已检测到有效会话</p>
            <strong><a href="/session.html">进入我的会话</a></strong>
          </div>
        </div>
        <div class="session-card session-card-muted" v-else>
          <span class="status-dot"></span>
          <div>
            <p>当前未登录</p>
            <strong>点击按钮进行认证</strong>
          </div>
        </div>
        <p v-if="error" class="notice notice-error">{{ error }}</p>
        <div class="session-actions">
          <a class="button button-primary" :href="loginUrl()">前往登录</a>
          <a class="button button-ghost" href="/session.html">我的会话</a>
        </div>
      </div>
    </section>

    <footer class="site-footer">Powered by PeerAPI</footer>
  </div>
</template>