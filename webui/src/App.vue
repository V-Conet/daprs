<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import {
  createPeering,
  fetchMe,
  fetchNodes,
  fetchPeers,
  loginUrl,
  logout as logoutApi,
  modifyPeering,
  removePeering,
} from './lib/api'
import type { MeResponse, NodeActionRequest, NodeAgentConfig, PeeringPayload, RoutingPolicy } from './types'

type ViewMode = 'overview' | 'nodes' | 'peers' | 'compose'
type MutationMode = 'create' | 'modify' | 'remove'

const viewModes: Array<{ key: ViewMode; label: string }> = [
  { key: 'overview', label: '概览' },
  { key: 'nodes', label: '节点' },
  { key: 'peers', label: '队列' },
  { key: 'compose', label: '编辑' },
]

const policies: RoutingPolicy[] = ['FullTable', 'Transit', 'PeeringOnly', 'Downstream']

const activeView = ref<ViewMode>('overview')
const loading = ref(true)
const busy = ref(false)
const error = ref('')
const me = ref<MeResponse | null>(null)
const nodes = ref<Record<string, NodeAgentConfig>>({})
const peers = ref<Array<NodeActionRequest<PeeringPayload>>>([])
const selectedNode = ref('')
const mutationMode = ref<MutationMode>('create')
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

const selectedNodeInfo = computed(() => nodes.value[selectedNode.value] ?? null)
const nodeEntries = computed(() => Object.entries(nodes.value))
const onlineCount = computed(() => nodeEntries.value.filter(([, node]) => node.online).length)
const offlineCount = computed(() => nodeEntries.value.length - onlineCount.value)
const sessionText = computed(() => {
  if (!me.value) {
    return '未登录'
  }

  return formatDuration(Math.max(0, me.value.expires_at * 1000 - Date.now()))
})

onMounted(async () => {
  await refreshAll()
})

async function refreshAll() {
  loading.value = true
  error.value = ''

  try {
    const [session, nodeData, peerData] = await Promise.all([
      fetchMe(),
      fetchNodes(),
      fetchPeers(),
    ])

    me.value = session
    nodes.value = nodeData
    peers.value = peerData

    if (!selectedNode.value) {
      selectedNode.value = Object.keys(nodeData)[0] ?? ''
    }

    if (!form.node) {
      form.node = selectedNode.value
    }

    if (!form.payload.endpoint && selectedNodeInfo.value?.address) {
      form.payload.endpoint = selectedNodeInfo.value.address
    }
  } catch (err) {
    const reason = err instanceof Error ? err.message : '请求失败'
    if (reason.includes('401')) {
      me.value = null
      nodes.value = {}
      peers.value = []
    } else {
      error.value = reason
    }
  } finally {
    loading.value = false
  }
}

function loadNodeIntoForm(name: string) {
  const node = nodes.value[name]
  if (!node) {
    return
  }

  selectedNode.value = name
  form.node = name
  form.payload.v4 = node.conf.dn42.ipv4 || ''
  form.payload.v6 = node.conf.dn42.ipv6 || ''
  form.payload.lla = node.conf.dn42.lla || ''
  form.payload.endpoint = node.address
  form.payload.pubkey = node.conf.dn42.wgkey || ''
}

function applyPeer(peer: NodeActionRequest<PeeringPayload>) {
  mutationMode.value = 'modify'
  form.node = peer.node
  selectedNode.value = peer.node
  Object.assign(form.payload, peer.payload)
}

async function submit(mode: MutationMode) {
  busy.value = true
  error.value = ''
  message.value = ''

  try {
    if (!form.node) {
      throw new Error('请选择一个节点')
    }

    if (!form.payload.endpoint.trim()) {
      throw new Error('请输入对端地址')
    }

    if (!form.payload.pubkey.trim()) {
      throw new Error('请输入对端公钥')
    }

    const payload: NodeActionRequest<PeeringPayload> = {
      node: form.node,
      payload: {
        ...form.payload,
        v4: normalizeOptionalText(form.payload.v4),
        v6: normalizeOptionalText(form.payload.v6),
        lla: normalizeOptionalText(form.payload.lla),
        psk: normalizeOptionalText(form.payload.psk),
        custom_port: normalizeNumber(form.payload.custom_port),
        mtu: normalizeNumber(form.payload.mtu),
        endpoint: form.payload.endpoint.trim(),
        pubkey: form.payload.pubkey.trim(),
      },
    }

    if (mode === 'create') {
      await createPeering(payload)
      message.value = '已提交创建请求'
    } else if (mode === 'modify') {
      await modifyPeering(payload)
      message.value = '已提交修改请求'
    } else {
      await removePeering({ node: form.node })
      message.value = '已提交删除请求'
    }

    await refreshAll()
  } catch (err) {
    error.value = err instanceof Error ? err.message : '操作失败'
  } finally {
    busy.value = false
  }
}

async function handleLogout() {
  busy.value = true
  error.value = ''

  try {
    await logoutApi()
    me.value = null
    message.value = '已退出登录'
  } catch (err) {
    error.value = err instanceof Error ? err.message : '退出失败'
  } finally {
    busy.value = false
  }
}

function useLogin() {
  window.location.href = loginUrl()
}

function shortId(value: string) {
  return value.length > 12 ? `${value.slice(0, 6)}…${value.slice(-4)}` : value
}

function formatTime(value: number) {
  return new Intl.DateTimeFormat('zh-CN', {
    dateStyle: 'short',
    timeStyle: 'medium',
  }).format(new Date(value * 1000))
}

function formatDuration(ms: number) {
  const totalMinutes = Math.floor(ms / 60000)
  const hours = Math.floor(totalMinutes / 60)
  const minutes = totalMinutes % 60
  return `${hours} 小时 ${minutes} 分钟后过期`
}

function normalizeOptionalText(value: string | null) {
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
</script>

<template>
  <div class="shell">
    <header class="topbar">
      <div>
        <p class="eyebrow">DAPRS / WebUI</p>
        <h1>极简互联控制台</h1>
        <p class="lead">面向 DN42 自动互联流程的轻量管理界面。</p>
      </div>
      <div class="topbar-actions">
        <a class="ghost-link" href="/api/login" target="_self">直接登录</a>
        <button class="button button-ghost" type="button" @click="refreshAll" :disabled="busy || loading">刷新</button>
      </div>
    </header>

    <main class="layout">
      <section class="hero card">
        <div class="hero-copy">
          <p class="eyebrow">Session</p>
          <h2>{{ me ? '已连接到控制面板' : '需要登录后才能继续' }}</h2>
          <p>{{ me ? '当前会话可直接操作节点、队列和互联申请。' : '点击登录后，回到这里继续查看节点和提交申请。' }}</p>
          <div class="hero-stats">
            <div>
              <strong>{{ nodeEntries.length }}</strong>
              <span>节点总数</span>
            </div>
            <div>
              <strong>{{ onlineCount }}</strong>
              <span>在线</span>
            </div>
            <div>
              <strong>{{ peers.length }}</strong>
              <span>待处理队列</span>
            </div>
          </div>
        </div>

        <div class="hero-panel">
          <div class="session-card" v-if="me">
            <span class="status-dot status-dot-ok"></span>
            <div>
              <p>会话有效</p>
              <strong>{{ sessionText }}</strong>
            </div>
          </div>

          <div class="session-card session-card-muted" v-else>
            <span class="status-dot"></span>
            <div>
              <p>尚未认证</p>
              <strong>登录后获取用户信息</strong>
            </div>
          </div>

          <div class="session-actions">
            <button class="button button-primary" type="button" @click="useLogin" v-if="!me">前往登录</button>
            <button class="button button-primary" type="button" @click="handleLogout" v-else :disabled="busy">退出登录</button>
            <button class="button button-ghost" type="button" @click="refreshAll" :disabled="busy || loading">重新拉取</button>
          </div>
        </div>
      </section>

      <nav class="tabs card">
        <button
          v-for="item in viewModes"
          :key="item.key"
          type="button"
          class="tab"
          :class="{ active: activeView === item.key }"
          @click="activeView = item.key"
        >
          {{ item.label }}
        </button>
      </nav>

      <p v-if="error" class="notice notice-error">{{ error }}</p>
      <p v-if="message" class="notice notice-ok">{{ message }}</p>

      <section v-if="activeView === 'overview'" class="content-grid">
        <article class="card pane">
          <div class="pane-header">
            <div>
              <p class="eyebrow">Account</p>
              <h3>当前会话</h3>
            </div>
            <span class="pill" :class="me ? 'pill-ok' : 'pill-warn'">{{ me ? '已登录' : '未登录' }}</span>
          </div>
          <template v-if="me">
            <div class="kv-list">
              <div>
                <span>Issued</span>
                <strong>{{ formatTime(me.issued_at) }}</strong>
              </div>
              <div>
                <span>Expires</span>
                <strong>{{ formatTime(me.expires_at) }}</strong>
              </div>
              <div v-for="(value, key) in me.userinfo" :key="key">
                <span>{{ key }}</span>
                <strong>{{ typeof value === 'string' ? shortId(value) : JSON.stringify(value) }}</strong>
              </div>
            </div>
          </template>
          <template v-else>
            <p class="muted">后端通过 OIDC 维护会话，前端只负责调用现有 API。</p>
          </template>
        </article>

        <article class="card pane">
          <div class="pane-header">
            <div>
              <p class="eyebrow">Nodes</p>
              <h3>节点概览</h3>
            </div>
          </div>
          <div class="stacked-list">
            <div v-for="[name, node] in nodeEntries" :key="name" class="list-row" @click="loadNodeIntoForm(name)">
              <div>
                <strong>{{ name }}</strong>
                <p>{{ node.address }}</p>
              </div>
              <span class="pill" :class="node.online ? 'pill-ok' : 'pill-warn'">{{ node.online ? '在线' : '离线' }}</span>
            </div>
          </div>
        </article>

        <article class="card pane">
          <div class="pane-header">
            <div>
              <p class="eyebrow">Queue</p>
              <h3>互联待办</h3>
            </div>
          </div>
          <div class="stacked-list">
            <div v-for="peer in peers" :key="`${peer.node}-${peer.payload.endpoint}`" class="list-row list-row-clickable" @click="applyPeer(peer)">
              <div>
                <strong>{{ peer.node }}</strong>
                <p>{{ peer.payload.endpoint }}</p>
              </div>
              <span class="pill pill-neutral">{{ peer.payload.policy }}</span>
            </div>
            <p v-if="!peers.length" class="empty-state">暂无待处理互联请求。</p>
          </div>
        </article>
      </section>

      <section v-else-if="activeView === 'nodes'" class="card pane wide-table">
        <div class="pane-header">
          <div>
            <p class="eyebrow">Nodes</p>
            <h3>节点详情</h3>
          </div>
          <span class="muted">{{ offlineCount }} 个离线</span>
        </div>
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>节点</th>
                <th>地址</th>
                <th>状态</th>
                <th>配置</th>
                <th>备注</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="[name, node] in nodeEntries" :key="name">
                <td>
                  <button class="inline-link" type="button" @click="loadNodeIntoForm(name)">{{ name }}</button>
                </td>
                <td>{{ node.address }}</td>
                <td>
                  <span class="pill" :class="node.online ? 'pill-ok' : 'pill-warn'">{{ node.online ? '在线' : '离线' }}</span>
                </td>
                <td>
                  <span class="mono">ASN {{ node.conf.dn42.asn }}</span>
                </td>
                <td>{{ node.error || node.conf.extra_msg }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section v-else-if="activeView === 'peers'" class="card pane wide-table">
        <div class="pane-header">
          <div>
            <p class="eyebrow">Peers</p>
            <h3>队列详情</h3>
          </div>
        </div>
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>节点</th>
                <th>策略</th>
                <th>端点</th>
                <th>端口</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="peer in peers" :key="`${peer.node}-${peer.payload.endpoint}`">
                <td>{{ peer.node }}</td>
                <td>{{ peer.payload.policy }}</td>
                <td>{{ peer.payload.endpoint }}</td>
                <td>{{ peer.payload.custom_port ?? '默认' }}</td>
                <td>
                  <button class="inline-link" type="button" @click="applyPeer(peer)">载入表单</button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section v-else class="editor-grid">
        <article class="card pane">
          <div class="pane-header">
            <div>
              <p class="eyebrow">Compose</p>
              <h3>互联表单</h3>
            </div>
            <span class="pill pill-neutral">{{ mutationMode }}</span>
          </div>

          <div class="form-grid">
            <label>
              <span>目标节点</span>
              <select v-model="form.node" @change="selectedNode = form.node || selectedNode">
                <option value="" disabled>选择节点</option>
                <option v-for="[name] in nodeEntries" :key="name" :value="name">{{ name }}</option>
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

          <div class="actions-row">
            <button class="button button-primary" type="button" @click="submit('create')" :disabled="busy">创建</button>
            <button class="button button-ghost" type="button" @click="submit('modify')" :disabled="busy">修改</button>
            <button class="button button-danger" type="button" @click="submit('remove')" :disabled="busy">删除</button>
          </div>
        </article>

        <article class="card pane side-panel">
          <div class="pane-header">
            <div>
              <p class="eyebrow">Preview</p>
              <h3>当前表单摘要</h3>
            </div>
          </div>

          <div class="preview-list">
            <div><span>节点</span><strong>{{ form.node || '未选择' }}</strong></div>
            <div><span>策略</span><strong>{{ form.payload.policy }}</strong></div>
            <div><span>端点</span><strong>{{ form.payload.endpoint || '未填写' }}</strong></div>
            <div><span>公钥</span><strong>{{ form.payload.pubkey ? shortId(form.payload.pubkey) : '未填写' }}</strong></div>
            <div><span>MTU</span><strong>{{ form.payload.mtu ?? '默认' }}</strong></div>
          </div>

          <div v-if="selectedNodeInfo" class="node-note">
            <p>所选节点</p>
            <strong>{{ selectedNode }} · {{ selectedNodeInfo.address }}</strong>
            <span>{{ selectedNodeInfo.conf.extra_msg }}</span>
          </div>
        </article>
      </section>
    </main>
  </div>
</template>
