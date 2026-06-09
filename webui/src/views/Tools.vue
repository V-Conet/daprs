<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { getNodes, executeCmd, type NodeAgentConfig } from '../api'

const nodes = ref<Record<string, NodeAgentConfig>>({})
const selectedNode = ref<string>('')
const cmdType = ref('ping')
const loading = ref(false)
const output = ref('')
const error = ref<string | null>(null)

// Ping
const pingCount = ref(4)
const pingProtocol = ref<string | null>(null)

// Traceroute
const tracerouteProtocol = ref<string | null>(null)

// Dig
const digType = ref('A')
const digServer = ref('')

// TcPing
const tcpingPort = ref(443)
const tcpingCount = ref(5)
const tcpingTimeout = ref(3)
const tcpingProtocol = ref<string | null>(null)

// Target
const target = ref('')

const onlineNodes = computed(() => {
  return Object.entries(nodes.value)
    .filter(([, n]) => n.online)
    .map(([name]) => name)
})

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
            target: target.value.trim()
          }
        }
        break
      case 'path':
        cmd = {
          op: 'path',
          args: {
            target: target.value.trim()
          }
        }
        break
    }

    const response = await executeCmd(selectedNode.value, cmd)
    output.value = response.data
  } catch (e: any) {
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

const commandOptions = [
  { value: 'ping', label: 'Ping' },
  { value: 'traceroute', label: 'Traceroute' },
  { value: 'tcping', label: 'TCP Ping' },
  { value: 'dig', label: 'Dig' },
  { value: 'route', label: 'Route' },
  { value: 'path', label: 'AS Path' }
]

const queryTypes = ['A', 'AAAA', 'MX', 'TXT', 'NS', 'SOA', 'CNAME', 'PTR']
</script>

<template>
  <div class="tools-page">
    <h2 class="page-title">Network Tools</h2>

    <!-- Command Bar -->
    <div class="command-bar">
      <div class="command-row">
        <div class="field-group">
          <label class="field-label">Node</label>
          <select v-model="selectedNode" class="field-select">
            <option v-for="name in onlineNodes" :key="name" :value="name">
              {{ name }}
            </option>
          </select>
        </div>

        <div class="field-group">
          <label class="field-label">Command</label>
          <select v-model="cmdType" class="field-select">
            <option v-for="opt in commandOptions" :key="opt.value" :value="opt.value">
              {{ opt.label }}
            </option>
          </select>
        </div>

        <div class="field-group grow">
          <label class="field-label">Target</label>
          <input
            v-model="target"
            type="text"
            class="field-input"
            placeholder="IP address or hostname"
            @keyup.enter="executeCommand"
          />
        </div>

        <button
          @click="executeCommand"
          class="btn-run"
          :disabled="loading || !selectedNode || !target.trim()"
        >
          <span v-if="loading" class="spinner-small"></span>
          <span v-else>Run</span>
        </button>
      </div>

      <!-- Ping Options -->
      <div v-if="cmdType === 'ping'" class="options-row">
        <div class="field-group small">
          <label class="field-label">Count</label>
          <input v-model.number="pingCount" type="number" class="field-input" min="1" max="20" />
        </div>
        <div class="field-group small">
          <label class="field-label">Protocol</label>
          <select v-model="pingProtocol" class="field-select">
            <option :value="null">Auto</option>
            <option value="v4">IPv4</option>
            <option value="v6">IPv6</option>
          </select>
        </div>
      </div>

      <!-- Traceroute Options -->
      <div v-if="cmdType === 'traceroute'" class="options-row">
        <div class="field-group small">
          <label class="field-label">Protocol</label>
          <select v-model="tracerouteProtocol" class="field-select">
            <option :value="null">Auto</option>
            <option value="v4">IPv4</option>
            <option value="v6">IPv6</option>
          </select>
        </div>
      </div>

      <!-- Dig Options -->
      <div v-if="cmdType === 'dig'" class="options-row">
        <div class="field-group small">
          <label class="field-label">Type</label>
          <select v-model="digType" class="field-select">
            <option v-for="t in queryTypes" :key="t" :value="t">{{ t }}</option>
          </select>
        </div>
        <div class="field-group">
          <label class="field-label">DNS Server</label>
          <input v-model="digServer" type="text" class="field-input" placeholder="e.g., 172.20.0.53" />
        </div>
      </div>

      <!-- TcPing Options -->
      <div v-if="cmdType === 'tcping'" class="options-row">
        <div class="field-group small">
          <label class="field-label">Port</label>
          <input v-model.number="tcpingPort" type="number" class="field-input" min="1" max="65535" />
        </div>
        <div class="field-group small">
          <label class="field-label">Count</label>
          <input v-model.number="tcpingCount" type="number" class="field-input" min="1" max="20" />
        </div>
        <div class="field-group small">
          <label class="field-label">Timeout</label>
          <input v-model.number="tcpingTimeout" type="number" class="field-input" min="1" max="30" />
        </div>
        <div class="field-group small">
          <label class="field-label">Protocol</label>
          <select v-model="tcpingProtocol" class="field-select">
            <option :value="null">Auto</option>
            <option value="v4">IPv4</option>
            <option value="v6">IPv6</option>
          </select>
        </div>
      </div>

      <!-- Route/Path Info -->
      <div v-if="cmdType === 'route' || cmdType === 'path'" class="options-row info">
        BIRD will automatically select the correct routing table.
      </div>

      <div v-if="onlineNodes.length === 0" class="no-nodes">
        No online nodes available
      </div>
    </div>

    <!-- Error -->
    <div v-if="error" class="error-banner">
      <span>{{ error }}</span>
      <button @click="clearOutput" class="btn-dismiss">×</button>
    </div>

    <!-- Output -->
    <div v-if="output" class="output-container">
      <div class="output-header">
        <span>Output</span>
        <button @click="clearOutput" class="btn-clear">Clear</button>
      </div>
      <pre class="output-content">{{ output }}</pre>
    </div>

    <!-- Empty State -->
    <div v-if="!output && !error" class="empty-state">
      <div class="empty-icon">⌨</div>
      <p>Select a node, enter a target, and run a command</p>
    </div>
  </div>
</template>

<style scoped>
.tools-page {
  padding: var(--space-xl) 0;
}

.page-title {
  font-size: 1.5rem;
  font-weight: 600;
  margin-bottom: var(--space-xl);
}

.command-bar {
  padding: var(--space-lg);
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  margin-bottom: var(--space-lg);
}

.command-row {
  display: flex;
  gap: var(--space-md);
  align-items: flex-end;
}

.field-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}

.field-group.grow {
  flex: 1;
}

.field-group.small {
  width: 100px;
}

.field-label {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-tertiary);
}

.field-select, .field-input {
  padding: var(--space-sm) var(--space-md);
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 0.875rem;
  min-width: 120px;
}

.field-input {
  min-width: auto;
  width: 100%;
}

.field-select:focus, .field-input:focus {
  outline: none;
  border-color: var(--accent);
}

.btn-run {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 80px;
  padding: var(--space-sm) var(--space-lg);
  background: var(--accent);
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-inverse);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-run:hover:not(:disabled) {
  background: var(--accent-hover);
}

.btn-run:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.spinner-small {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.options-row {
  display: flex;
  gap: var(--space-md);
  margin-top: var(--space-md);
  padding-top: var(--space-md);
  border-top: 1px solid var(--border-color);
}

.options-row.info {
  font-size: 0.875rem;
  color: var(--text-tertiary);
  padding: var(--space-md);
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
}

.no-nodes {
  margin-top: var(--space-md);
  font-size: 0.875rem;
  color: var(--danger);
}

.error-banner {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-md);
  background: var(--danger-light);
  border: 1px solid var(--danger);
  border-radius: var(--radius-md);
  color: var(--danger);
  margin-bottom: var(--space-lg);
}

.btn-dismiss {
  background: none;
  border: none;
  color: var(--danger);
  font-size: 1.25rem;
  cursor: pointer;
  opacity: 0.7;
}

.btn-dismiss:hover {
  opacity: 1;
}

.output-container {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.output-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-sm) var(--space-md);
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-color);
  font-size: 0.875rem;
  font-weight: 500;
}

.btn-clear {
  padding: var(--space-xs) var(--space-sm);
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: 0.75rem;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-clear:hover {
  border-color: var(--text-tertiary);
  color: var(--text-primary);
}

.output-content {
  padding: var(--space-md);
  background: var(--code-bg);
  color: #e0e0e0;
  font-family: var(--font-mono);
  font-size: 0.8rem;
  line-height: 1.5;
  max-height: 500px;
  overflow: auto;
  white-space: pre;
}

.empty-state {
  text-align: center;
  padding: var(--space-3xl);
  color: var(--text-tertiary);
}

.empty-icon {
  font-size: 3rem;
  margin-bottom: var(--space-md);
  opacity: 0.5;
}

@media (max-width: 768px) {
  .command-row {
    flex-wrap: wrap;
  }

  .field-group.grow {
    flex: 1 1 100%;
  }

  .btn-run {
    width: 100%;
  }

  .options-row {
    flex-wrap: wrap;
  }

  .field-group.small {
    width: calc(50% - var(--space-sm));
  }
}
</style>
