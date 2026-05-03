<script setup lang="ts">
import { onMounted, ref } from 'vue'

import { fetchMe, fetchNodes } from '../lib/api'
import type { NodeAgentConfig } from '../types'
import { stageLinks, unauthorized, useThemeToggle } from '../lib/page'

const nodes = ref<Record<string, NodeAgentConfig>>({})
const error = ref('')
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
    nodes.value = await fetchNodes()
  } catch (err) {
    const text = err instanceof Error ? err.message : '请求失败'
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
      <a v-for="item in stageLinks" :key="item.key" :href="item.href" class="tab" :class="{ active: item.key === 'nodes' }">{{ item.label }}</a>
    </nav>

    <section class="card pane wide-table">
      <div class="pane-header">
        <div>
          <p class="eyebrow">Stage 3</p>
          <h3>BGP 网络节点</h3>
        </div>
        <button class="button button-ghost" type="button" @click="refresh">刷新</button>
      </div>
      <p v-if="error" class="notice notice-error">{{ error }}</p>
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>节点</th>
              <th>地址</th>
              <th>状态</th>
              <th>ASN</th>
              <th>动作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(node, name) in nodes" :key="name">
              <td><strong>{{ name }}</strong></td>
              <td>{{ node.address }}</td>
              <td><span class="pill" :class="node.online ? 'pill-ok' : 'pill-warn'">{{ node.online ? '在线' : '离线' }}</span></td>
              <td class="mono">{{ node.conf.dn42.asn }}</td>
              <td><a class="inline-link" :href="`/compose.html?node=${encodeURIComponent(name)}`">进入节点互联</a></td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <footer class="site-footer">Powered by PeerAPI</footer>
  </div>
</template>