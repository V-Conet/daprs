<script setup lang="ts">
import { onMounted, ref } from 'vue'

import { fetchMe, fetchNodes, fetchPeers, logout } from '../lib/api'
import type { MeResponse } from '../types'
import { stageLinks, unauthorized } from '../lib/page'

const me = ref<MeResponse | null>(null)
const nodeCount = ref(0)
const peerCount = ref(0)
const error = ref('')
const message = ref('')

onMounted(async () => {
  await refresh()
})

async function refresh() {
  error.value = ''
  const [session, nodes, peers] = await Promise.all([fetchMe(), fetchNodes(), fetchPeers()])
  me.value = session
  nodeCount.value = Object.keys(nodes).length
  peerCount.value = peers.length
}

async function handleRefresh() {
  try {
    await refresh()
  } catch (err) {
    const text = err instanceof Error ? err.message : '请求失败'
    error.value = unauthorized(text) ? '未登录，请先前往登录页。' : text
  }
}

async function handleLogout() {
  try {
    await logout()
    message.value = '已退出登录'
    me.value = null
  } catch (err) {
    error.value = err instanceof Error ? err.message : '退出失败'
  }
}

function formatTime(value: number) {
  return new Intl.DateTimeFormat('zh-CN', { dateStyle: 'short', timeStyle: 'medium' }).format(new Date(value * 1000))
}
</script>

<template>
  <div class="shell">
    <nav class="tabs card">
      <a v-for="item in stageLinks" :key="item.key" :href="item.href" class="tab" :class="{ active: item.key === 'session' }">{{ item.label }}</a>
    </nav>

    <section class="hero card">
      <div class="hero-copy">
        <p class="eyebrow">Stage 2</p>
        <h2>我的会话</h2>
        <p>验证登录状态、查看会话有效期，并作为后续页面的分流入口。</p>
        <div class="hero-stats">
          <div><strong>{{ nodeCount }}</strong><span>节点总数</span></div>
          <div><strong>{{ peerCount }}</strong><span>互联队列</span></div>
          <div><strong>{{ me ? '有效' : '失效' }}</strong><span>会话状态</span></div>
        </div>
      </div>
      <div class="hero-panel">
        <div class="session-card" v-if="me">
          <span class="status-dot status-dot-ok"></span>
          <div>
            <p>当前会话</p>
            <strong>{{ formatTime(me.expires_at) }} 过期</strong>
          </div>
        </div>
        <div class="session-card session-card-muted" v-else>
          <span class="status-dot"></span>
          <div>
            <p>暂无有效会话</p>
            <strong><a href="/signin.html">前往登录页</a></strong>
          </div>
        </div>
        <p v-if="error" class="notice notice-error">{{ error }}</p>
        <p v-if="message" class="notice notice-ok">{{ message }}</p>
        <div class="session-actions">
          <button class="button button-ghost" type="button" @click="handleRefresh">刷新</button>
          <button class="button button-primary" type="button" @click="handleLogout">退出</button>
        </div>
      </div>
    </section>

    <section class="card pane" v-if="me">
      <div class="pane-header">
        <div>
          <p class="eyebrow">UserInfo</p>
          <h3>会话信息</h3>
        </div>
      </div>
      <div class="kv-list">
        <div><span>Issued</span><strong>{{ formatTime(me.issued_at) }}</strong></div>
        <div><span>Expires</span><strong>{{ formatTime(me.expires_at) }}</strong></div>
        <div v-for="(value, key) in me.userinfo" :key="key">
          <span>{{ key }}</span>
          <strong>{{ typeof value === 'string' ? value : JSON.stringify(value) }}</strong>
        </div>
      </div>
    </section>
  </div>
</template>