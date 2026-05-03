<script setup lang="ts">
import { onMounted, ref } from 'vue'

import { deletePeeringQueue, fetchMe, fetchPeers } from '../lib/api'
import type { NodeActionRequest, PeeringPayload } from '../types'
import { stageLinks, unauthorized, useThemeToggle } from '../lib/page'

const peers = ref<Array<NodeActionRequest<PeeringPayload>>>([])
const error = ref('')
const message = ref('')
const { themeLabel, toggleTheme } = useThemeToggle()

onMounted(async () => {
  try {
    await fetchMe()
  } catch (err) {
    const text = err instanceof Error ? err.message : '请求失败'
    if (unauthorized(text)) {
      window.location.href = '/signin.html'
      return
    }
  }
  await refresh()
})

async function refresh() {
  try {
    error.value = ''
    message.value = ''
    peers.value = await fetchPeers()
  } catch (err) {
    const text = err instanceof Error ? err.message : '请求失败'
    error.value = unauthorized(text) ? '未登录，请先前往登录页。' : text
  }
}

async function removeQueue(node: string) {
  try {
    error.value = ''
    await deletePeeringQueue(node)
    message.value = `已从队列删除 ${node}`
    peers.value = peers.value.filter((item) => item.node !== node)
  } catch (err) {
    const text = err instanceof Error ? err.message : '删除失败'
    error.value = unauthorized(text) ? '未登录，请先前往登录页。' : text
  }
}
</script>

<template>
  <div class="shell">
    <header class="site-header card">
      <div class="site-header-inner">
        <a class="logo" href="/signin.html">dn42</a>
        <div class="header-right">
          <button class="button button-ghost" type="button" @click="toggleTheme">{{ themeLabel }}</button>
        </div>
      </div>
    </header>

    <nav class="tabs card">
      <a v-for="item in stageLinks" :key="item.key" :href="item.href" class="tab" :class="{ active: item.key === 'peers' }">{{ item.label }}</a>
    </nav>

    <section class="card pane wide-table">
      <div class="pane-header">
        <div>
          <p class="eyebrow">Stage 5</p>
          <h3>互联队列</h3>
        </div>
        <button class="button button-ghost" type="button" @click="refresh">刷新</button>
      </div>
      <p v-if="error" class="notice notice-error">{{ error }}</p>
      <p v-if="message" class="notice notice-ok">{{ message }}</p>
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>节点</th>
              <th>策略</th>
              <th>端点</th>
              <th>端口</th>
              <th>动作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="peer in peers" :key="`${peer.node}-${peer.payload.endpoint}`">
              <td>{{ peer.node }}</td>
              <td>{{ peer.payload.policy }}</td>
              <td>{{ peer.payload.endpoint }}</td>
              <td>{{ peer.payload.custom_port ?? '默认' }}</td>
              <td>
                <div class="button-row">
                  <a class="inline-link" :href="`/compose.html?node=${encodeURIComponent(peer.node)}`">去编辑</a>
                  <button class="inline-link" type="button" @click="removeQueue(peer.node)">删队列</button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <footer class="site-footer">Powered by PeerAPI</footer>
  </div>
</template>