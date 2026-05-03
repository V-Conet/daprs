<script setup lang="ts">
import { onMounted, ref } from 'vue'

import { fetchPeers } from '../lib/api'
import type { NodeActionRequest, PeeringPayload } from '../types'
import { stageLinks, unauthorized } from '../lib/page'

const peers = ref<Array<NodeActionRequest<PeeringPayload>>>([])
const error = ref('')

onMounted(async () => {
  await refresh()
})

async function refresh() {
  try {
    error.value = ''
    peers.value = await fetchPeers()
  } catch (err) {
    const text = err instanceof Error ? err.message : '请求失败'
    error.value = unauthorized(text) ? '未登录，请先前往登录页。' : text
  }
}
</script>

<template>
  <div class="shell">
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
              <td><a class="inline-link" :href="`/compose.html?node=${encodeURIComponent(peer.node)}`">去编辑</a></td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>