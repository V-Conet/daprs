<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { getPeerInfo, removePeering } from '../api'

const route = useRoute()
const router = useRouter()

const nodeName = route.params.node as string
const peerInfo = ref<any>(null)
const loading = ref(true)
const deleting = ref(false)
const error = ref<string | null>(null)
const success = ref<string | null>(null)

const hasPeer = computed(() => {
  return peerInfo.value?.peer != null
})

onMounted(async () => {
  await loadPeerInfo()
})

async function loadPeerInfo() {
  loading.value = true
  error.value = null

  try {
    const response = await getPeerInfo(nodeName)
    peerInfo.value = response.data
  } catch (e: any) {
    if (e.response?.status === 404) {
      error.value = 'No peer configuration found on this node'
    } else if (e.response?.status === 502) {
      error.value = 'Agent is not reachable. Please check if the agent is running.'
    } else {
      error.value = e.response?.data?.error || 'Failed to load peer info'
    }
    console.error(e)
  } finally {
    loading.value = false
  }
}

async function deletePeer() {
  if (!confirm(`Are you sure you want to delete the peer on node "${nodeName}"?\n\nThis will remove the WireGuard interface and BGP configuration.`)) {
    return
  }

  deleting.value = true
  error.value = null

  try {
    await removePeering(nodeName)
    success.value = 'Peer deleted successfully! Redirecting...'
    setTimeout(() => {
      router.push('/dashboard')
    }, 1500)
  } catch (e: any) {
    error.value = e.response?.data?.error || 'Failed to delete peer'
    console.error(e)
  } finally {
    deleting.value = false
  }
}

function goBack() {
  router.push('/dashboard')
}

function refreshInfo() {
  loadPeerInfo()
}
</script>

<template>
  <div class="peer-info-page">
    <div class="page-header">
      <button @click="goBack" class="btn-back">
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="19" y1="12" x2="5" y2="12"></line>
          <polyline points="12 19 5 12 12 5"></polyline>
        </svg>
      </button>
      <h2>Peer Info · {{ nodeName }}</h2>
      <button @click="refreshInfo" class="btn-refresh" :disabled="loading">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 2v6h-6"></path>
          <path d="M3 12a9 9 0 0 1 15-6.7L21 8"></path>
          <path d="M3 22v-6h6"></path>
          <path d="M21 12a9 9 0 0 1-15 6.7L3 16"></path>
        </svg>
        Refresh
      </button>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>

    <div v-else-if="success" class="success-state">
      <div class="success-icon">✓</div>
      <p>{{ success }}</p>
    </div>

    <div v-else-if="error" class="error-state">
      <div class="error-icon">⚠</div>
      <h3>Error</h3>
      <p>{{ error }}</p>
      <button @click="goBack" class="btn-back-dashboard">Back to Dashboard</button>
    </div>

    <template v-else-if="peerInfo">
      <!-- Status Cards -->
      <div class="info-cards">
        <div class="info-card">
          <div class="card-title">Peer Configuration</div>
          <template v-if="peerInfo.peer">
            <div class="info-list">
              <div class="info-row">
                <span class="info-label">ASN</span>
                <span class="info-value">AS{{ peerInfo.asn }}</span>
              </div>
              <div class="info-row">
                <span class="info-label">Endpoint</span>
                <span class="info-value mono">{{ peerInfo.peer.endpoint || 'N/A' }}</span>
              </div>
              <div class="info-row">
                <span class="info-label">IPv4</span>
                <span class="info-value mono">{{ peerInfo.peer.v4 || 'N/A' }}</span>
              </div>
              <div class="info-row">
                <span class="info-label">IPv6</span>
                <span class="info-value mono">{{ peerInfo.peer.v6 || 'N/A' }}</span>
              </div>
              <div class="info-row">
                <span class="info-label">PubKey</span>
                <span class="info-value mono key">{{ peerInfo.peer.pubkey || 'N/A' }}</span>
              </div>
            </div>
          </template>
          <div v-else class="no-data">No peer configuration found</div>
        </div>

        <div class="info-card">
          <div class="card-title">My Configuration</div>
          <div class="info-list">
            <div class="info-row">
              <span class="info-label">IPv4</span>
              <span class="info-value mono">{{ peerInfo.my_v4 || 'N/A' }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">IPv6</span>
              <span class="info-value mono">{{ peerInfo.my_v6 || 'N/A' }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">LLA</span>
              <span class="info-value mono">{{ peerInfo.my_lla || 'N/A' }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">PubKey</span>
              <span class="info-value mono key">{{ peerInfo.my_pubkey || 'N/A' }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Actions -->
      <div class="action-bar">
        <router-link :to="`/peering/${nodeName}`" class="btn-primary">
          {{ hasPeer ? 'Modify Peer' : 'Add Peer' }}
        </router-link>
        <button v-if="hasPeer" @click="deletePeer" class="btn-danger" :disabled="deleting">
          <span v-if="deleting" class="spinner-small"></span>
          Delete Peer
        </button>
      </div>

      <!-- WireGuard Status -->
      <div v-if="hasPeer" class="section">
        <div class="section-header">
          <h3>WireGuard Status</h3>
          <span class="status-badge" :class="peerInfo.interface_up ? 'up' : 'down'">
            {{ peerInfo.interface_up ? 'Up' : 'Down' }}
          </span>
        </div>
        <template v-if="peerInfo.interface_up && peerInfo.wg_show">
          <pre class="output-block">{{ peerInfo.wg_show.output || 'No output' }}</pre>
        </template>
        <template v-else>
          <div class="warning-block">
            <p>WireGuard interface is not running</p>
            <small>Try refreshing or check if <code>wg-quick up dn42-{{ peerInfo.asn }}</code> was executed</small>
          </div>
        </template>
      </div>

      <!-- BGP Status -->
      <div v-if="hasPeer && peerInfo.interface_up" class="section">
        <div class="section-header">
          <h3>BGP Status</h3>
        </div>
        <div v-if="peerInfo.bird_protocols?.length > 0">
          <div v-for="protocol in peerInfo.bird_protocols" :key="protocol.command" class="protocol-block">
            <div class="protocol-cmd">{{ protocol.command }}</div>
            <pre class="output-block">{{ protocol.output || 'No output' }}</pre>
          </div>
        </div>
        <div v-else class="no-data">No BGP protocol information available</div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.peer-info-page {
  padding: var(--space-xl) 0;
}

.page-header {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin-bottom: var(--space-xl);
}

.page-header h2 {
  flex: 1;
  font-size: 1.25rem;
  font-weight: 600;
}

.btn-back {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-back:hover {
  border-color: var(--text-tertiary);
  color: var(--text-primary);
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

.loading-state, .error-state, .success-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-3xl);
  text-align: center;
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

.success-icon, .error-icon {
  font-size: 3rem;
  margin-bottom: var(--space-md);
}

.success-icon {
  color: var(--success);
}

.error-icon {
  color: var(--warning);
}

.error-state h3 {
  margin-bottom: var(--space-sm);
  color: var(--text-primary);
}

.error-state p {
  color: var(--text-secondary);
  margin-bottom: var(--space-lg);
}

.btn-back-dashboard {
  padding: var(--space-sm) var(--space-lg);
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-back-dashboard:hover {
  border-color: var(--text-tertiary);
  color: var(--text-primary);
}

.info-cards {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--space-lg);
  margin-bottom: var(--space-xl);
}

.info-card {
  padding: var(--space-lg);
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
}

.card-title {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--text-tertiary);
  margin-bottom: var(--space-md);
  padding-bottom: var(--space-sm);
  border-bottom: 1px solid var(--border-color);
}

.info-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
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

.info-value.key {
  font-size: 0.7rem;
  word-break: break-all;
}

.no-data {
  color: var(--text-tertiary);
  font-size: 0.875rem;
}

.action-bar {
  display: flex;
  gap: var(--space-md);
  margin-bottom: var(--space-xl);
}

.btn-primary {
  display: inline-flex;
  align-items: center;
  padding: var(--space-sm) var(--space-lg);
  background: var(--accent);
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-inverse);
  font-size: 0.875rem;
  font-weight: 500;
  text-decoration: none;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-primary:hover {
  background: var(--accent-hover);
}

.btn-danger {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-lg);
  background: transparent;
  border: 1px solid var(--danger);
  border-radius: var(--radius-sm);
  color: var(--danger);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-danger:hover:not(:disabled) {
  background: var(--danger-light);
}

.btn-danger:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.spinner-small {
  width: 14px;
  height: 14px;
  border: 2px solid var(--danger);
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

.section {
  margin-bottom: var(--space-xl);
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--space-md);
}

.section-header h3 {
  font-size: 1rem;
  font-weight: 600;
}

.status-badge {
  font-size: 0.75rem;
  font-weight: 500;
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-full);
}

.status-badge.up {
  background: var(--success-light);
  color: var(--success);
}

.status-badge.down {
  background: var(--danger-light);
  color: var(--danger);
}

.output-block {
  padding: var(--space-md);
  background: var(--code-bg);
  border-radius: var(--radius-md);
  color: #e0e0e0;
  font-family: var(--font-mono);
  font-size: 0.8rem;
  line-height: 1.5;
  overflow-x: auto;
  white-space: pre;
}

.warning-block {
  padding: var(--space-lg);
  background: var(--warning-light);
  border: 1px solid var(--warning);
  border-radius: var(--radius-md);
  text-align: center;
}

.warning-block p {
  color: var(--warning);
  margin-bottom: var(--space-xs);
}

.warning-block small {
  color: var(--text-tertiary);
}

.warning-block code {
  background: var(--bg-tertiary);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
}

.protocol-block {
  margin-bottom: var(--space-md);
}

.protocol-block:last-child {
  margin-bottom: 0;
}

.protocol-cmd {
  padding: var(--space-sm) var(--space-md);
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-bottom: none;
  border-radius: var(--radius-md) var(--radius-md) 0 0;
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: var(--text-tertiary);
}

.protocol-block .output-block {
  border-radius: 0 0 var(--radius-md) var(--radius-md);
}

@media (max-width: 768px) {
  .info-cards {
    grid-template-columns: 1fr;
  }

  .page-header h2 {
    font-size: 1rem;
  }
}
</style>
