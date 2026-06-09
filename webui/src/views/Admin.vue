<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import {
  checkAdmin,
  getPendingRequests,
  approveRequest,
  rejectRequest,
  getAllPeers,
  adminDeletePeer,
  getAuditLogs,
  type PendingRequest,
  type PeerInfoResponse,
  type AuditLog
} from '../api'

const router = useRouter()

const isAdmin = ref(false)
const loading = ref(true)
const activeTab = ref<'pending' | 'peers' | 'audit'>('pending')

const pendingRequests = ref<PendingRequest[]>([])
const pendingLoading = ref(false)

const allPeers = ref<Record<string, PeerInfoResponse[]>>({})
const peersLoading = ref(false)

const auditLogs = ref<AuditLog[]>([])
const auditLoading = ref(false)

const deleteModal = ref(false)
const deleteTarget = ref<{ node: string; asn: number } | null>(null)
const deleteLoading = ref(false)

const expandedPeers = ref<Set<string>>(new Set())

onMounted(async () => {
  try {
    const response = await checkAdmin()
    isAdmin.value = response.data
    if (!isAdmin.value) {
      router.push('/dashboard')
      return
    }
    await loadPendingRequests()
  } catch {
    router.push('/dashboard')
  } finally {
    loading.value = false
  }
})

async function loadPendingRequests() {
  pendingLoading.value = true
  try {
    const response = await getPendingRequests()
    pendingRequests.value = response.data
  } catch (e) {
    console.error('Failed to load pending requests:', e)
  } finally {
    pendingLoading.value = false
  }
}

async function loadAllPeers() {
  peersLoading.value = true
  try {
    const response = await getAllPeers()
    allPeers.value = response.data
  } catch (e) {
    console.error('Failed to load all peers:', e)
  } finally {
    peersLoading.value = false
  }
}

async function loadAuditLogs() {
  auditLoading.value = true
  try {
    const response = await getAuditLogs()
    auditLogs.value = response.data
  } catch (e) {
    console.error('Failed to load audit logs:', e)
  } finally {
    auditLoading.value = false
  }
}

async function handleApprove(id: string) {
  try {
    await approveRequest(id)
    pendingRequests.value = pendingRequests.value.filter(r => r.id !== id)
  } catch (e: any) {
    alert(e.response?.data?.error || 'Failed to approve request')
  }
}

async function handleReject(id: string) {
  if (!confirm('Are you sure you want to reject this request?')) return
  try {
    await rejectRequest(id)
    pendingRequests.value = pendingRequests.value.filter(r => r.id !== id)
  } catch (e: any) {
    alert(e.response?.data?.error || 'Failed to reject request')
  }
}

function showDeleteModal(node: string, asn: number) {
  deleteTarget.value = { node, asn }
  deleteModal.value = true
}

function goToModify(node: string, asn: number) {
  router.push(`/admin/modify/${node}/${asn}`)
}

async function confirmDelete() {
  if (!deleteTarget.value) return
  deleteLoading.value = true
  try {
    await adminDeletePeer(deleteTarget.value)
    await loadAllPeers()
    deleteModal.value = false
    deleteTarget.value = null
  } catch (e: any) {
    alert(e.response?.data?.error || 'Failed to delete peer')
  } finally {
    deleteLoading.value = false
  }
}

function formatTime(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString()
}

const totalPending = computed(() => pendingRequests.value.length)

function onTabChange(tab: 'pending' | 'peers' | 'audit') {
  activeTab.value = tab
  if (tab === 'peers' && Object.keys(allPeers.value).length === 0) {
    loadAllPeers()
  }
  if (tab === 'audit' && auditLogs.value.length === 0) {
    loadAuditLogs()
  }
}

function isNodeError(peerList: PeerInfoResponse[]): boolean {
  return peerList.length === 1 && (peerList[0] as any).error
}

function togglePeerDetails(node: string, asn: number) {
  const key = `${node}-${asn}`
  if (expandedPeers.value.has(key)) {
    expandedPeers.value.delete(key)
  } else {
    expandedPeers.value.add(key)
  }
}

function isPeerExpanded(node: string, asn: number): boolean {
  return expandedPeers.value.has(`${node}-${asn}`)
}

function getActionLabel(action: string): string {
  const labels: Record<string, string> = {
    create: 'Create',
    approve: 'Approve',
    reject: 'Reject',
    modify: 'Modify',
    delete: 'Delete'
  }
  return labels[action] || action
}

function getActionClass(action: string): string {
  const classes: Record<string, string> = {
    create: 'primary',
    approve: 'success',
    reject: 'danger',
    modify: 'warning',
    delete: 'secondary'
  }
  return classes[action] || 'secondary'
}
</script>

<template>
  <div class="admin-page">
    <div class="page-header">
      <h2>Admin Panel</h2>
      <span class="pending-count" v-if="totalPending > 0">{{ totalPending }} pending</span>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>

    <template v-else-if="isAdmin">
      <!-- Tabs -->
      <div class="tabs">
        <button
          :class="['tab', { active: activeTab === 'pending' }]"
          @click="onTabChange('pending')"
        >
          Pending Requests
          <span v-if="totalPending > 0" class="tab-badge">{{ totalPending }}</span>
        </button>
        <button
          :class="['tab', { active: activeTab === 'peers' }]"
          @click="onTabChange('peers')"
        >
          All Peers
        </button>
        <button
          :class="['tab', { active: activeTab === 'audit' }]"
          @click="onTabChange('audit')"
        >
          Recent Operations
        </button>
      </div>

      <!-- Pending Requests -->
      <div v-if="activeTab === 'pending'">
        <div v-if="pendingLoading" class="loading-state small">
          <div class="spinner"></div>
        </div>

        <div v-else-if="pendingRequests.length === 0" class="empty-state">
          <div class="empty-icon">✓</div>
          <p>No pending requests</p>
        </div>

        <div v-else class="table-container">
          <table class="data-table">
            <thead>
              <tr>
                <th>ASN</th>
                <th>Node</th>
                <th>Endpoint</th>
                <th>Created</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="req in pendingRequests" :key="req.id">
                <td><span class="tag">AS{{ req.asn }}</span></td>
                <td>{{ req.node }}</td>
                <td class="mono">{{ req.payload.endpoint }}</td>
                <td class="muted">{{ formatTime(req.created_at) }}</td>
                <td>
                  <div class="action-btns">
                    <button @click="handleApprove(req.id)" class="btn-sm success">Approve</button>
                    <button @click="handleReject(req.id)" class="btn-sm danger">Reject</button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- All Peers -->
      <div v-if="activeTab === 'peers'">
        <div class="toolbar">
          <button @click="loadAllPeers" class="btn-refresh" :disabled="peersLoading">
            Refresh
          </button>
        </div>

        <div v-if="peersLoading" class="loading-state small">
          <div class="spinner"></div>
        </div>

        <div v-else class="peer-list">
          <div v-for="(peerList, nodeName) in allPeers" :key="nodeName" class="node-block">
            <div class="node-header">
              <h3>{{ nodeName }}</h3>
              <span class="peer-count">{{ peerList.length }} peer(s)</span>
            </div>

            <div v-if="isNodeError(peerList)" class="node-error">
              {{ (peerList[0] as any).error }}
            </div>
            <div v-else-if="peerList.length === 0" class="no-peers">
              No peers configured
            </div>
            <div v-else class="table-container">
              <table class="data-table">
                <thead>
                  <tr>
                    <th style="width: 30px"></th>
                    <th>ASN</th>
                    <th>Status</th>
                    <th>Endpoint</th>
                    <th>IPv4</th>
                    <th>IPv6</th>
                    <th>BGP</th>
                    <th>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  <template v-for="(peerInfo, idx) in peerList" :key="idx">
                    <tr v-if="(peerInfo as any).error">
                      <td colspan="8" class="error-text">
                        AS{{ (peerInfo as any).asn }}: {{ (peerInfo as any).error }}
                      </td>
                    </tr>
                    <template v-else>
                      <tr @click="togglePeerDetails(nodeName, peerInfo.asn)" class="clickable">
                        <td>
                          <span class="expand-icon">{{ isPeerExpanded(nodeName, peerInfo.asn) ? '▼' : '▶' }}</span>
                        </td>
                        <td><span class="tag info">AS{{ peerInfo.asn }}</span></td>
                        <td>
                          <span :class="['status', peerInfo.interface_up ? 'up' : 'down']">
                            {{ peerInfo.interface_up ? 'Up' : 'Down' }}
                          </span>
                        </td>
                        <td class="mono truncate">{{ peerInfo.wg?.endpoint || 'N/A' }}</td>
                        <td class="mono">{{ peerInfo.wg?.peer_v4 || 'N/A' }}</td>
                        <td class="mono truncate">{{ peerInfo.wg?.peer_v6 || 'N/A' }}</td>
                        <td>{{ peerInfo.bird?.session_type || 'N/A' }}</td>
                        <td @click.stop>
                          <div class="action-btns">
                            <button @click="goToModify(nodeName, peerInfo.asn)" class="btn-icon" title="Modify">✎</button>
                            <button @click="showDeleteModal(nodeName, peerInfo.asn)" class="btn-icon danger" title="Delete">✕</button>
                          </div>
                        </td>
                      </tr>
                      <tr v-if="isPeerExpanded(nodeName, peerInfo.asn)">
                        <td colspan="8" class="expanded-content">
                          <div class="expanded-grid">
                            <div class="expanded-section">
                              <h4>WireGuard</h4>
                              <div class="detail-row"><span>Port</span><span>{{ peerInfo.wg?.port || 'N/A' }}</span></div>
                              <div class="detail-row"><span>MTU</span><span>{{ peerInfo.wg?.mtu || 1420 }}</span></div>
                              <div class="detail-row"><span>PubKey</span><span class="mono key">{{ peerInfo.wg?.pubkey || 'N/A' }}</span></div>
                              <div class="detail-row"><span>PSK</span><span>{{ peerInfo.wg?.psk ? 'Set' : 'None' }}</span></div>
                            </div>
                            <div class="expanded-section">
                              <h4>BGP</h4>
                              <div class="detail-row"><span>Session</span><span>{{ peerInfo.bird?.session_type || 'N/A' }}</span></div>
                              <div class="detail-row"><span>MP-BGP</span><span>{{ peerInfo.bird?.is_mhp ? 'Yes' : 'No' }}</span></div>
                              <div class="detail-row"><span>Ext NH</span><span>{{ peerInfo.bird?.is_nhp ? 'Yes' : 'No' }}</span></div>
                            </div>
                          </div>
                          <div v-if="peerInfo.interface_up && peerInfo.wg_show" class="output-section">
                            <h4>wg show</h4>
                            <pre class="output-block">{{ peerInfo.wg_show.output }}</pre>
                          </div>
                        </td>
                      </tr>
                    </template>
                  </template>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <div v-if="Object.keys(allPeers).length === 0" class="empty-state">
          <div class="empty-icon">📭</div>
          <p>No nodes found</p>
        </div>
      </div>

      <!-- Audit Logs -->
      <div v-if="activeTab === 'audit'">
        <div class="toolbar">
          <button @click="loadAuditLogs" class="btn-refresh" :disabled="auditLoading">
            Refresh
          </button>
        </div>

        <div v-if="auditLoading" class="loading-state small">
          <div class="spinner"></div>
        </div>

        <div v-else-if="auditLogs.length === 0" class="empty-state">
          <div class="empty-icon">📋</div>
          <p>No operations recorded</p>
        </div>

        <div v-else class="table-container">
          <table class="data-table">
            <thead>
              <tr>
                <th>Time</th>
                <th>Actor</th>
                <th>Action</th>
                <th>Target</th>
                <th>Node</th>
                <th>Result</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="log in auditLogs" :key="log.id">
                <td class="muted">{{ formatTime(log.timestamp) }}</td>
                <td><span class="tag">AS{{ log.actor_asn }}</span></td>
                <td>
                  <span :class="['tag', getActionClass(log.action)]">{{ getActionLabel(log.action) }}</span>
                </td>
                <td><span class="tag info">AS{{ log.target_asn }}</span></td>
                <td>{{ log.node }}</td>
                <td>
                  <span :class="['status', log.result === 'success' ? 'up' : 'down']">
                    {{ log.result === 'success' ? 'Success' : 'Failed' }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </template>

    <!-- Delete Modal -->
    <div v-if="deleteModal" class="modal-overlay" @click="deleteModal = false">
      <div class="modal" @click.stop>
        <h3>Confirm Delete</h3>
        <p>Are you sure you want to delete this peer?</p>
        <div class="modal-info">
          <span>Node: <strong>{{ deleteTarget?.node }}</strong></span>
          <span>ASN: <strong>AS{{ deleteTarget?.asn }}</strong></span>
        </div>
        <div class="modal-actions">
          <button @click="deleteModal = false" class="btn-cancel">Cancel</button>
          <button @click="confirmDelete" class="btn-danger" :disabled="deleteLoading">
            <span v-if="deleteLoading" class="spinner-small"></span>
            Delete
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.admin-page {
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

.pending-count {
  font-size: 0.75rem;
  font-weight: 500;
  padding: var(--space-xs) var(--space-sm);
  background: var(--accent-light);
  color: var(--accent);
  border-radius: var(--radius-full);
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: var(--space-3xl);
}

.loading-state.small {
  padding: var(--space-xl);
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

.empty-state {
  text-align: center;
  padding: var(--space-3xl);
  color: var(--text-tertiary);
}

.empty-icon {
  font-size: 3rem;
  margin-bottom: var(--space-md);
}

.tabs {
  display: flex;
  gap: var(--space-xs);
  margin-bottom: var(--space-xl);
  border-bottom: 1px solid var(--border-color);
  padding-bottom: var(--space-sm);
}

.tab {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  padding: var(--space-sm) var(--space-md);
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 0.875rem;
  cursor: pointer;
  position: relative;
  transition: color var(--transition-fast);
}

.tab:hover {
  color: var(--text-primary);
}

.tab.active {
  color: var(--text-primary);
}

.tab.active::after {
  content: '';
  position: absolute;
  bottom: calc(-1 * var(--space-sm) - 1px);
  left: 0;
  right: 0;
  height: 2px;
  background: var(--accent);
}

.tab-badge {
  font-size: 0.7rem;
  padding: 2px 6px;
  background: var(--warning);
  color: var(--text-inverse);
  border-radius: var(--radius-full);
}

.toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: var(--space-lg);
}

.btn-refresh {
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

.table-container {
  overflow-x: auto;
}

.data-table {
  width: 100%;
  border-collapse: collapse;
}

.data-table th {
  text-align: left;
  padding: var(--space-sm) var(--space-md);
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-tertiary);
  border-bottom: 1px solid var(--border-color);
}

.data-table td {
  padding: var(--space-sm) var(--space-md);
  font-size: 0.875rem;
  border-bottom: 1px solid var(--border-light);
}

.data-table tr.clickable {
  cursor: pointer;
}

.data-table tr.clickable:hover {
  background: var(--bg-hover);
}

.mono {
  font-family: var(--font-mono);
  font-size: 0.8rem;
}

.muted {
  color: var(--text-tertiary);
  font-size: 0.8rem;
}

.truncate {
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tag {
  display: inline-block;
  padding: 2px 8px;
  font-size: 0.7rem;
  font-weight: 500;
  border-radius: var(--radius-full);
  background: var(--bg-tertiary);
  color: var(--text-secondary);
}

.tag.info {
  background: var(--accent-light);
  color: var(--accent);
}

.tag.primary {
  background: var(--accent-light);
  color: var(--accent);
}

.tag.success {
  background: var(--success-light);
  color: var(--success);
}

.tag.warning {
  background: var(--warning-light);
  color: var(--warning);
}

.tag.danger {
  background: var(--danger-light);
  color: var(--danger);
}

.tag.secondary {
  background: var(--bg-tertiary);
  color: var(--text-tertiary);
}

.status {
  font-size: 0.75rem;
  font-weight: 500;
  padding: 2px 8px;
  border-radius: var(--radius-full);
}

.status.up {
  background: var(--success-light);
  color: var(--success);
}

.status.down {
  background: var(--danger-light);
  color: var(--danger);
}

.action-btns {
  display: flex;
  gap: var(--space-xs);
}

.btn-sm {
  padding: var(--space-xs) var(--space-sm);
  font-size: 0.75rem;
  font-weight: 500;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-sm.success {
  background: var(--success);
  color: var(--text-inverse);
}

.btn-sm.success:hover {
  opacity: 0.9;
}

.btn-sm.danger {
  background: transparent;
  border: 1px solid var(--danger);
  color: var(--danger);
}

.btn-sm.danger:hover {
  background: var(--danger-light);
}

.btn-icon {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-icon:hover {
  border-color: var(--text-tertiary);
  color: var(--text-primary);
}

.btn-icon.danger {
  color: var(--danger);
  border-color: var(--danger);
}

.btn-icon.danger:hover {
  background: var(--danger-light);
}

.expand-icon {
  font-size: 0.7rem;
  color: var(--text-tertiary);
}

.peer-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}

.node-block {
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
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-color);
}

.node-header h3 {
  font-size: 1rem;
  font-weight: 600;
}

.peer-count {
  font-size: 0.75rem;
  color: var(--text-tertiary);
}

.node-error, .no-peers {
  padding: var(--space-lg);
  text-align: center;
  color: var(--text-tertiary);
}

.node-error {
  color: var(--warning);
}

.error-text {
  color: var(--warning);
}

.expanded-content {
  padding: var(--space-lg);
  background: var(--bg-secondary);
}

.expanded-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--space-lg);
  margin-bottom: var(--space-lg);
}

.expanded-section h4 {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-tertiary);
  margin-bottom: var(--space-sm);
}

.detail-row {
  display: flex;
  justify-content: space-between;
  padding: var(--space-xs) 0;
  font-size: 0.875rem;
}

.detail-row span:first-child {
  color: var(--text-tertiary);
}

.detail-row .key {
  font-size: 0.7rem;
  word-break: break-all;
}

.output-section h4 {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-tertiary);
  margin-bottom: var(--space-sm);
}

.output-block {
  padding: var(--space-md);
  background: var(--code-bg);
  border-radius: var(--radius-md);
  color: #e0e0e0;
  font-family: var(--font-mono);
  font-size: 0.75rem;
  line-height: 1.5;
  overflow-x: auto;
  white-space: pre;
  max-height: 150px;
}

/* Modal */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: var(--bg-primary);
  border-radius: var(--radius-md);
  padding: var(--space-xl);
  max-width: 400px;
  width: 90%;
}

.modal h3 {
  font-size: 1.125rem;
  margin-bottom: var(--space-md);
}

.modal p {
  color: var(--text-secondary);
  margin-bottom: var(--space-md);
}

.modal-info {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
  padding: var(--space-md);
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
  margin-bottom: var(--space-lg);
  font-size: 0.875rem;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-md);
}

.btn-cancel {
  padding: var(--space-sm) var(--space-lg);
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 0.875rem;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-cancel:hover {
  border-color: var(--text-tertiary);
  color: var(--text-primary);
}

.btn-danger {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-lg);
  background: var(--danger);
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-inverse);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-danger:hover:not(:disabled) {
  opacity: 0.9;
}

.btn-danger:disabled {
  opacity: 0.6;
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

@media (max-width: 768px) {
  .expanded-grid {
    grid-template-columns: 1fr;
  }

  .tabs {
    flex-wrap: wrap;
  }

  .data-table {
    font-size: 0.8rem;
  }
}
</style>