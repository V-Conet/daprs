<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'

import { createPeering, fetchNodes, modifyPeering, removePeering } from '../lib/api'
import { stageLinks, unauthorized } from '../lib/page'
import type { NodeActionRequest, NodeAgentConfig, PeeringPayload, RoutingPolicy } from '../types'

const policies: RoutingPolicy[] = ['FullTable', 'Transit', 'PeeringOnly', 'Downstream']
const nodes = ref<Record<string, NodeAgentConfig>>({})
const busy = ref(false)
const error = ref('')
const message = ref('')

const form = reactive<NodeActionRequest<PeeringPayload>>({
  node: '',
  payload: {
    is_mhp: true,
    is_nhp: true,
    policy: 'Transit',
    v4: '',
    v6: '',
    lla: '',
    is_prefer_lla: true,
    endpoint: '',
    pubkey: '',
    custom_port: null,
    psk: '',
    mtu: 1420,
  },
})

onMounted(async () => {
  await refreshNodes()
})

async function refreshNodes() {
  try {
    error.value = ''
    const fetched = await fetchNodes()
    nodes.value = fetched

    const queryNode = new URLSearchParams(window.location.search).get('node') || ''
    if (queryNode && fetched[queryNode]) {
      applyNode(queryNode)
    } else if (!form.node) {
      const first = Object.keys(fetched)[0]
      if (first) {
        applyNode(first)
      }
    }
  } catch (err) {
    const text = err instanceof Error ? err.message : '请求失败'
    error.value = unauthorized(text) ? '未登录，请先前往登录页。' : text
  }
}

function applyNode(name: string) {
  const node = nodes.value[name]
  if (!node) {
    return
  }

  form.node = name
  form.payload.v4 = node.conf.dn42.ipv4 || ''
  form.payload.v6 = node.conf.dn42.ipv6 || ''
  form.payload.lla = node.conf.dn42.lla || ''
  form.payload.endpoint = node.address
  form.payload.pubkey = node.conf.dn42.wgkey || ''
}

function normalizeText(value: string | null) {
  if (value === null) {
    return null
  }

  const trimmed = value.trim()
  return trimmed === '' ? null : trimmed
}

function normalizeNumber(value: number | null) {
  if (value === null || Number.isNaN(value)) {
    return null
  }
  return value
}

function getPayload(): NodeActionRequest<PeeringPayload> {
  return {
    node: form.node,
    payload: {
      ...form.payload,
      endpoint: form.payload.endpoint.trim(),
      pubkey: form.payload.pubkey.trim(),
      v4: normalizeText(form.payload.v4),
      v6: normalizeText(form.payload.v6),
      lla: normalizeText(form.payload.lla),
      psk: normalizeText(form.payload.psk),
      custom_port: normalizeNumber(form.payload.custom_port),
      mtu: normalizeNumber(form.payload.mtu),
    },
  }
}

async function submit(action: 'create' | 'modify' | 'remove') {
  busy.value = true
  error.value = ''
  message.value = ''

  try {
    if (!form.node) {
      throw new Error('请选择一个节点')
    }

    if (action === 'remove') {
      await removePeering({ node: form.node })
      message.value = '已提交删除请求'
    } else {
      const payload = getPayload()
      if (!payload.payload.endpoint) {
        throw new Error('请输入对端地址')
      }
      if (!payload.payload.pubkey) {
        throw new Error('请输入对端公钥')
      }

      if (action === 'create') {
        await createPeering(payload)
        message.value = '已提交创建请求'
      } else {
        await modifyPeering(payload)
        message.value = '已提交修改请求'
      }
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : '提交失败'
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="shell">
    <nav class="tabs card">
      <a v-for="item in stageLinks" :key="item.key" :href="item.href" class="tab" :class="{ active: item.key === 'compose' }">{{ item.label }}</a>
    </nav>

    <section class="card pane">
      <div class="pane-header">
        <div>
          <p class="eyebrow">Stage 4</p>
          <h3>节点互联</h3>
        </div>
        <button class="button button-ghost" type="button" @click="refreshNodes">刷新节点</button>
      </div>

      <p v-if="error" class="notice notice-error">{{ error }}</p>
      <p v-if="message" class="notice notice-ok">{{ message }}</p>

      <div class="form-grid">
        <label>
          <span>目标节点</span>
          <select v-model="form.node" @change="applyNode(form.node)">
            <option value="" disabled>选择节点</option>
            <option v-for="(node, name) in nodes" :key="name" :value="name">{{ name }} · {{ node.address }}</option>
          </select>
        </label>

        <label>
          <span>路由策略</span>
          <select v-model="form.payload.policy">
            <option v-for="policy in policies" :key="policy" :value="policy">{{ policy }}</option>
          </select>
        </label>

        <label>
          <span>对端地址</span>
          <input v-model="form.payload.endpoint" placeholder="198.51.100.1:31213" />
        </label>

        <label>
          <span>对端公钥</span>
          <input v-model="form.payload.pubkey" placeholder="WireGuard public key" />
        </label>

        <label>
          <span>本地 IPv4</span>
          <input v-model="form.payload.v4" placeholder="172.20.x.x" />
        </label>

        <label>
          <span>本地 IPv6</span>
          <input v-model="form.payload.v6" placeholder="fd42:..." />
        </label>

        <label>
          <span>链路本地地址</span>
          <input v-model="form.payload.lla" placeholder="fe80::1" />
        </label>

        <label>
          <span>开放端口</span>
          <input v-model.number="form.payload.custom_port" type="number" min="0" max="65535" placeholder="默认" />
        </label>

        <label>
          <span>PSK</span>
          <input v-model="form.payload.psk" placeholder="可选" />
        </label>

        <label>
          <span>MTU</span>
          <input v-model.number="form.payload.mtu" type="number" min="1280" max="9000" placeholder="1420" />
        </label>

        <label class="toggle-row">
          <input v-model="form.payload.is_mhp" type="checkbox" />
          <span>MultiHop</span>
        </label>

        <label class="toggle-row">
          <input v-model="form.payload.is_nhp" type="checkbox" />
          <span>Extended NextHop</span>
        </label>

        <label class="toggle-row">
          <input v-model="form.payload.is_prefer_lla" type="checkbox" />
          <span>优先链路本地地址</span>
        </label>
      </div>

      <div class="actions-row" style="margin-top: 18px;">
        <button class="button button-primary" type="button" :disabled="busy" @click="submit('create')">创建</button>
        <button class="button button-ghost" type="button" :disabled="busy" @click="submit('modify')">修改</button>
        <button class="button button-danger" type="button" :disabled="busy" @click="submit('remove')">删除</button>
      </div>
    </section>
  </div>
</template>