<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { getNodes, executeCmd, type NodeAgentConfig } from '../api'

const nodes = ref<Record<string, NodeAgentConfig>>({})
const selectedNode = ref<string>('')
const cmdType = ref('ping')
const loading = ref(false)
const output = ref('')
const error = ref<string | null>(null)

// Ping 参数
const pingCount = ref(4)
const pingProtocol = ref<number | null>(null)

// Traceroute 参数
const tracerouteProtocol = ref<number | null>(null)

// Dig 参数
const digType = ref('A')
const digServer = ref('')

// TcPing 参数
const tcpingPort = ref(443)
const tcpingCount = ref(5)
const tcpingTimeout = ref(3)
const tcpingProtocol = ref<number | null>(null)

// Route/Path 参数
const routeProtocol = ref<number | null>(null)

// 目标地址
const target = ref('')

// 计算在线节点
const onlineNodes = computed(() => {
  return Object.entries(nodes.value)
    .filter(([, n]) => n.online)
    .map(([name]) => name)
})

// 加载节点列表
async function loadNodes() {
  try {
    const response = await getNodes()
    nodes.value = response.data
    const online = Object.entries(response.data).filter(([, n]) => n.online)
    if (online.length > 0) {
      selectedNode.value = online[0][0]
    }
  } catch (e) {
    console.error('Failed to load nodes:', e)
    error.value = 'Failed to load nodes. Please check if server is running.'
  }
}

onMounted(loadNodes)

async function executeCommand() {
  if (!selectedNode.value) {
    error.value = 'Please select a node'
    return
  }
  if (!target.value.trim()) {
    error.value = 'Please enter a target'
    return
  }

  loading.value = true
  error.value = null
  output.value = ''

  try {
    let cmd: Record<string, unknown> = {}

    switch (cmdType.value) {
      case 'ping':
        cmd = {
          op: 'ping',
          args: {
            target: target.value.trim(),
            count: pingCount.value || 4,
            protocol: pingProtocol.value
          }
        }
        break
      case 'traceroute':
        cmd = {
          op: 'traceroute',
          args: {
            target: target.value.trim(),
            protocol: tracerouteProtocol.value
          }
        }
        break
      case 'dig':
        cmd = {
          op: 'dig',
          args: {
            target: target.value.trim(),
            qtype: digType.value || 'A',
            server: digServer.value || null
          }
        }
        break
      case 'tcping':
        cmd = {
          op: 'tcping',
          args: {
            target: target.value.trim(),
            port: tcpingPort.value || 443,
            count: tcpingCount.value || 5,
            timeout: tcpingTimeout.value || 3,
            protocol: tcpingProtocol.value
          }
        }
        break
      case 'route':
        cmd = {
          op: 'route',
          args: {
            target: target.value.trim(),
            protocol: routeProtocol.value
          }
        }
        break
      case 'path':
        cmd = {
          op: 'path',
          args: {
            target: target.value.trim(),
            protocol: routeProtocol.value
          }
        }
        break
    }

    const response = await executeCmd(selectedNode.value, cmd)
    output.value = response.data
  } catch (e: any) {
    // 处理超时错误
    if (e.code === 'ECONNABORTED' || e.message?.includes('timeout')) {
      error.value = 'Command timed out. The operation took too long to complete.'
    } else {
      const msg = e.response?.data?.error || e.message || 'Command failed'
      error.value = msg
    }
    output.value = ''
  } finally {
    loading.value = false
  }
}

function clearOutput() {
  output.value = ''
  error.value = null
}
</script>

<template>
  <div>
    <h2 class="mb-4">
      Network Tools
    </h2>

    <!-- 节点选择 -->
    <div class="card shadow-sm mb-4">
      <div class="card-body">
        <div class="row g-3 align-items-end">
          <div class="col-md-3">
            <label class="form-label">Node</label>
            <select v-model="selectedNode" class="form-select">
              <option v-for="name in onlineNodes" :key="name" :value="name">
                {{ name }}
              </option>
            </select>
            <div v-if="onlineNodes.length === 0" class="form-text text-danger">
              No online nodes available
            </div>
          </div>

          <div class="col-md-3">
            <label class="form-label">Command</label>
            <select v-model="cmdType" class="form-select">
              <option value="ping">Ping</option>
              <option value="traceroute">Traceroute</option>
              <option value="tcping">TCP Ping</option>
              <option value="dig">Dig</option>
              <option value="route">Route</option>
              <option value="path">AS Path</option>
            </select>
          </div>

          <div class="col-md-4">
            <label class="form-label">Target</label>
            <input
              v-model="target"
              type="text"
              class="form-control"
              placeholder="IP address or hostname"
              @keyup.enter="executeCommand"
            />
          </div>

          <div class="col-md-2">
            <button
              @click="executeCommand"
              class="btn btn-primary w-100"
              :disabled="loading || !selectedNode || !target.trim()"
            >
              <span v-if="loading" class="spinner-border spinner-border-sm me-1"></span>
              Run
            </button>
          </div>
        </div>

        <!-- Ping 参数 -->
        <div v-if="cmdType === 'ping'" class="row g-3 mt-2 pt-3 border-top">
          <div class="col-md-3">
            <label class="form-label small">Count</label>
            <input v-model.number="pingCount" type="number" class="form-control form-control-sm" min="1" max="20" />
          </div>
          <div class="col-md-3">
            <label class="form-label small">Protocol</label>
            <select v-model="pingProtocol" class="form-select form-select-sm">
              <option :value="null">Auto</option>
              <option :value="4">IPv4</option>
              <option :value="6">IPv6</option>
            </select>
          </div>
        </div>

        <!-- Traceroute 参数 -->
        <div v-if="cmdType === 'traceroute'" class="row g-3 mt-2 pt-3 border-top">
          <div class="col-md-3">
            <label class="form-label small">Protocol</label>
            <select v-model="tracerouteProtocol" class="form-select form-select-sm">
              <option :value="null">Auto</option>
              <option :value="4">IPv4</option>
              <option :value="6">IPv6</option>
            </select>
          </div>
        </div>

        <!-- Dig 参数 -->
        <div v-if="cmdType === 'dig'" class="row g-3 mt-2 pt-3 border-top">
          <div class="col-md-3">
            <label class="form-label small">Query Type</label>
            <select v-model="digType" class="form-select form-select-sm">
              <option>A</option>
              <option>AAAA</option>
              <option>MX</option>
              <option>TXT</option>
              <option>NS</option>
              <option>SOA</option>
              <option>CNAME</option>
              <option>PTR</option>
            </select>
          </div>
          <div class="col-md-4">
            <label class="form-label small">DNS Server (optional)</label>
            <input v-model="digServer" type="text" class="form-control form-control-sm" placeholder="e.g., 172.20.0.53" />
          </div>
        </div>

        <!-- TcPing 参数 -->
        <div v-if="cmdType === 'tcping'" class="row g-3 mt-2 pt-3 border-top">
          <div class="col-md-2">
            <label class="form-label small">Port</label>
            <input v-model.number="tcpingPort" type="number" class="form-control form-control-sm" min="1" max="65535" />
          </div>
          <div class="col-md-2">
            <label class="form-label small">Count</label>
            <input v-model.number="tcpingCount" type="number" class="form-control form-control-sm" min="1" max="20" />
          </div>
          <div class="col-md-2">
            <label class="form-label small">Timeout (s)</label>
            <input v-model.number="tcpingTimeout" type="number" class="form-control form-control-sm" min="1" max="30" />
          </div>
          <div class="col-md-2">
            <label class="form-label small">Protocol</label>
            <select v-model="tcpingProtocol" class="form-select form-select-sm">
              <option :value="null">Auto</option>
              <option :value="4">IPv4</option>
              <option :value="6">IPv6</option>
            </select>
          </div>
        </div>

        <!-- Route/Path 参数 -->
        <div v-if="cmdType === 'route' || cmdType === 'path'" class="row g-3 mt-2 pt-3 border-top">
          <div class="col-md-3">
            <label class="form-label small">Protocol</label>
            <select v-model="routeProtocol" class="form-select form-select-sm">
              <option :value="null">Auto</option>
              <option :value="4">IPv4</option>
              <option :value="6">IPv6</option>
            </select>
          </div>
        </div>
      </div>
    </div>

    <!-- 错误提示 -->
    <div v-if="error" class="alert alert-danger d-flex justify-content-between align-items-center">
      <span>{{ error }}</span>
      <button @click="clearOutput" class="btn btn-sm btn-outline-danger">Dismiss</button>
    </div>

    <!-- 输出结果 -->
    <div v-if="output" class="card shadow-sm">
      <div class="card-header d-flex justify-content-between align-items-center">
        <span class="fw-bold">Output</span>
        <button @click="clearOutput" class="btn btn-sm btn-outline-secondary">Clear</button>
      </div>
      <div class="card-body p-0">
        <pre class="bg-dark text-light p-3 mb-0" style="max-height: 500px; overflow-y: auto; font-size: 0.85rem;">{{ output }}</pre>
      </div>
    </div>

    <!-- 空状态 -->
    <div v-if="!output && !error" class="text-center py-5 text-muted">
      <div class="display-4 mb-3">⌨</div>
      <p>Select a node, enter a target, and run a command</p>
    </div>
  </div>
</template>
