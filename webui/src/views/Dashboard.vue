<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { getNodes, getMyPendingRequests, type NodeAgentConfig, type PendingRequest } from '../api'

const router = useRouter()
const nodes = ref<Record<string, NodeAgentConfig>>({})
const pendingRequests = ref<PendingRequest[]>([])
const loading = ref(true)
const error = ref<string | null>(null)

onMounted(async () => {
  await loadNodes()
  await loadPendingRequests()
})

async function loadNodes() {
  loading.value = true
  error.value = null
  try {
    const response = await getNodes()
    nodes.value = response.data
  } catch (e: any) {
    if (e.response?.status === 401) {
      window.location.href = '/api/login'
      return
    }
    error.value = 'Failed to load nodes. Please check if server is running.'
    console.error(e)
  } finally {
    loading.value = false
  }
}

async function loadPendingRequests() {
  try {
    const response = await getMyPendingRequests()
    pendingRequests.value = response.data
  } catch {
    // Ignore errors for pending requests
  }
}

function goToPeering(nodeName: string) {
  router.push(`/peering/${nodeName}`)
}

function goToPeerInfo(nodeName: string) {
  router.push(`/peer/${nodeName}`)
}

function formatTime(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString()
}
</script>

<template>
  <div class="dashboard">
    <div class="page-header">
      <h2>Nodes</h2>
      <button @click="loadNodes" class="btn-refresh" :disabled="loading">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 2v6h-6"></path>
          <path d="M3 12a9 9 0 0 1 15-6.7L21 8"></path>
          <path d="M3 22v-6h6"></path>
          <path d="M21 12a9 9 0 0 1-15 6.7L3 16"></path>
        </svg>
        Refresh
      </button>
    </div>

    <!-- Error -->
    <div v-if="error" class="error-block">
      <span>{{ error }}</span>
      <button @click="loadNodes" class="btn-retry">Retry</button>
    </div>

    <!-- Pending Requests -->
    <div v-if="pendingRequests.length > 0" class="pending-block">
      <div class="pending-header">
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"></circle>
          <polyline points="12 6 12 12 16 14"></polyline>
        </svg>
        Pending Peering Requests
      </div>
      <div class="pending-list">
        <div v-for="req in pendingRequests" :key="req.id" class="pending-item">
          <span class="pending-node">{{ req.node }}</span>
          <span class="pending-time">{{ formatTime(req.created_at) }}</span>
        </div>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>

    <!-- Node Grid -->
    <div v-else class="node-grid">
      <div v-for="(node, name) in nodes" :key="name" class="node-card">
        <div class="node-header">
          <span class="node-name">{{ name }}</span>
          <span class="node-status" :class="node.online ? 'online' : 'offline'">
            {{ node.online ? 'Online' : 'Offline' }}
          </span>
        </div>

        <div class="node-body">
          <div class="node-info">
            <div class="info-row">
              <span class="info-label">ASN</span>
              <span class="info-value">{{ node.conf.dn42.asn || 'N/A' }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">IPv4</span>
              <span class="info-value mono">{{ node.conf.dn42.ipv4 || 'N/A' }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">IPv6</span>
              <span class="info-value mono truncate">{{ node.conf.dn42.ipv6 || 'N/A' }}</span>
            </div>
          </div>

          <div class="node-tags">
            <span v-if="node.conf.net.ipv4" class="tag">IPv4</span>
            <span v-if="node.conf.net.ipv6" class="tag">IPv6</span>
            <span v-if="node.conf.net.cn" class="tag warn">CN</span>
            <span v-if="!node.conf.net.accept_nat" class="tag">No NAT</span>
          </div>

          <div class="node-flags">
            <span class="flag" :class="node.conf.is_open ? 'open' : 'closed'">
              {{ node.conf.is_open ? 'Open' : 'Closed' }}
            </span>
            <span v-if="node.conf.is_verify" class="flag verify">Verify</span>
          </div>

          <div v-if="node.error" class="node-error">{{ node.error }}</div>
          <div v-else-if="node.conf.extra_msg" class="node-extra">{{ node.conf.extra_msg }}</div>
        </div>

        <div class="node-actions">
          <button
            @click="goToPeering(name)"
            class="btn-action primary"
            :disabled="!node.online || !node.conf.is_open"
            :title="!node.online ? 'Node offline' : !node.conf.is_open ? 'Peering closed' : 'Add new peer'"
          >
            Add Peer
          </button>
          <button
            @click="goToPeerInfo(name)"
            class="btn-action"
            :disabled="!node.online"
          >
            Info
          </button>
        </div>
      </div>
    </div>

    <!-- Empty State -->
    <div v-if="!loading && Object.keys(nodes).length === 0" class="empty-state">
      <div class="empty-icon">📡</div>
      <h3>No nodes configured</h3>
      <p>Add nodes to server.toml configuration</p>
    </div>
  </div>
</template>

<style scoped>
.dashboard {
  padding: var(--space-xl) 0;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--space-xl);
}

.page-header h2 {
  font-size: 1.5rem;
  font-weight: 600;
}

.btn-refresh {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  padding: var(--space-sm) var(--space-md);
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 0.875rem;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-refresh:hover:not(:disabled) {
  border-color: var(--text-tertiary);
  color: var(--text-primary);
}

.btn-refresh:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.error-block {
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

.btn-retry {
  padding: var(--space-xs) var(--space-sm);
  background: transparent;
  border: 1px solid var(--danger);
  border-radius: var(--radius-sm);
  color: var(--danger);
  font-size: 0.75rem;
  cursor: pointer;
}

.pending-block {
  padding: var(--space-md);
  background: var(--warning-light);
  border: 1px solid var(--warning);
  border-radius: var(--radius-md);
  margin-bottom: var(--space-lg);
}

.pending-header {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  font-weight: 600;
  color: var(--warning);
  margin-bottom: var(--space-md);
}

.pending-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}

.pending-item {
  display: flex;
  justify-content: space-between;
  font-size: 0.875rem;
}

.pending-node {
  font-weight: 500;
}

.pending-time {
  color: var(--text-tertiary);
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: var(--space-3xl);
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--border-color);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.node-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-lg);
}

.node-card {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.node-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-md) var(--space-lg);
  border-bottom: 1px solid var(--border-color);
}

.node-name {
  font-weight: 600;
}

.node-status {
  font-size: 0.75rem;
  font-weight: 500;
  padding: 2px 8px;
  border-radius: var(--radius-full);
}

.node-status.online {
  background: var(--success-light);
  color: var(--success);
}

.node-status.offline {
  background: var(--danger-light);
  color: var(--danger);
}

.node-body {
  padding: var(--space-md) var(--space-lg);
}

.node-info {
  margin-bottom: var(--space-md);
}

.info-row {
  display: flex;
  justify-content: space-between;
  padding: var(--space-xs) 0;
}

.info-label {
  font-size: 0.75rem;
  color: var(--text-tertiary);
}

.info-value {
  font-size: 0.875rem;
  font-weight: 500;
}

.info-value.mono {
  font-family: var(--font-mono);
  font-size: 0.8rem;
}

.info-value.truncate {
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-tags {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-xs);
  margin-bottom: var(--space-sm);
}

.tag {
  font-size: 0.7rem;
  padding: 2px 6px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
}

.tag.warn {
  background: var(--warning-light);
  color: var(--warning);
}

.node-flags {
  display: flex;
  gap: var(--space-xs);
  margin-bottom: var(--space-sm);
}

.flag {
  font-size: 0.7rem;
  padding: 2px 6px;
  border-radius: var(--radius-sm);
}

.flag.open {
  background: var(--success-light);
  color: var(--success);
}

.flag.closed {
  background: var(--bg-tertiary);
  color: var(--text-tertiary);
}

.flag.verify {
  background: var(--accent-light);
  color: var(--accent);
}

.node-error {
  font-size: 0.75rem;
  color: var(--warning);
}

.node-extra {
  font-size: 0.75rem;
  color: var(--text-tertiary);
}

.node-actions {
  display: flex;
  gap: var(--space-sm);
  padding: var(--space-md) var(--space-lg);
  border-top: 1px solid var(--border-color);
}

.btn-action {
  flex: 1;
  padding: var(--space-sm);
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-action:hover:not(:disabled) {
  border-color: var(--text-tertiary);
}

.btn-action:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.btn-action.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: var(--text-inverse);
}

.btn-action.primary:hover:not(:disabled) {
  background: var(--accent-hover);
  border-color: var(--accent-hover);
}

.empty-state {
  text-align: center;
  padding: var(--space-3xl);
}

.empty-icon {
  font-size: 3rem;
  margin-bottom: var(--space-md);
}

.empty-state h3 {
  margin-bottom: var(--space-sm);
  color: var(--text-secondary);
}

.empty-state p {
  color: var(--text-tertiary);
}

@media (max-width: 1024px) {
  .node-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 640px) {
  .node-grid {
    grid-template-columns: 1fr;
  }
}
</style>
