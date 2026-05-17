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
  type PendingRequest,
  type PeerInfoResponse
} from '../api'

const router = useRouter()

const isAdmin = ref(false)
const loading = ref(true)
const activeTab = ref<'pending' | 'peers'>('pending')

// Pending requests
const pendingRequests = ref<PendingRequest[]>([])
const pendingLoading = ref(false)

// All peers
const allPeers = ref<Record<string, PeerInfoResponse[]>>({})
const peersLoading = ref(false)

// Delete confirmation
const deleteModal = ref(false)
const deleteTarget = ref<{ node: string; asn: number } | null>(null)
const deleteLoading = ref(false)

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

// Watch tab changes
function onTabChange(tab: 'pending' | 'peers') {
  activeTab.value = tab
  if (tab === 'peers' && Object.keys(allPeers.value).length === 0) {
    loadAllPeers()
  }
}

// Check if node has error
function isNodeError(peerList: PeerInfoResponse[]): boolean {
  return peerList.length === 1 && (peerList[0] as any).error
}
</script>

<template>
  <div>
    <div class="d-flex justify-content-between align-items-center mb-4">
      <h2 class="mb-0">
        <i class="bi bi-shield-check me-2"></i>
        Admin Panel
      </h2>
      <span class="badge bg-primary">{{ totalPending }} pending requests</span>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="text-center py-5">
      <div class="spinner-border text-primary" role="status"></div>
    </div>

    <template v-else-if="isAdmin">
      <!-- Tabs -->
      <ul class="nav nav-tabs mb-4">
        <li class="nav-item">
          <button
            class="nav-link"
            :class="{ active: activeTab === 'pending' }"
            @click="onTabChange('pending')"
          >
            <i class="bi bi-clock-history me-1"></i>
            Pending Requests
            <span v-if="totalPending > 0" class="badge bg-warning text-dark ms-1">{{ totalPending }}</span>
          </button>
        </li>
        <li class="nav-item">
          <button
            class="nav-link"
            :class="{ active: activeTab === 'peers' }"
            @click="onTabChange('peers')"
          >
            <i class="bi bi-diagram-3 me-1"></i>
            All Peers
          </button>
        </li>
      </ul>

      <!-- Pending Requests Tab -->
      <div v-if="activeTab === 'pending'">
        <div v-if="pendingLoading" class="text-center py-4">
          <div class="spinner-border spinner-border-sm"></div>
        </div>

        <div v-else-if="pendingRequests.length === 0" class="text-center py-5 text-muted">
          <i class="bi bi-check-circle display-4"></i>
          <p class="mt-3">No pending requests</p>
        </div>

        <div v-else class="table-responsive">
          <table class="table table-hover">
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
                <td>
                  <span class="badge bg-secondary">AS{{ req.asn }}</span>
                </td>
                <td>{{ req.node }}</td>
                <td class="font-monospace small">{{ req.payload.endpoint }}</td>
                <td class="small text-muted">{{ formatTime(req.created_at) }}</td>
                <td>
                  <div class="btn-group btn-group-sm">
                    <button @click="handleApprove(req.id)" class="btn btn-success">
                      <i class="bi bi-check-lg"></i> Approve
                    </button>
                    <button @click="handleReject(req.id)" class="btn btn-outline-danger">
                      <i class="bi bi-x-lg"></i> Reject
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- All Peers Tab -->
      <div v-if="activeTab === 'peers'">
        <div class="d-flex justify-content-end mb-3">
          <button @click="loadAllPeers" class="btn btn-outline-primary btn-sm" :disabled="peersLoading">
            <i class="bi bi-arrow-clockwise me-1"></i> Refresh
          </button>
        </div>

        <div v-if="peersLoading" class="text-center py-4">
          <div class="spinner-border spinner-border-sm"></div>
        </div>

        <div v-else class="row g-4">
          <div v-for="(peerList, nodeName) in allPeers" :key="nodeName" class="col-12">
            <div class="card">
              <div class="card-header d-flex justify-content-between align-items-center">
                <h5 class="mb-0">
                  <i class="bi bi-hdd-network me-2"></i>{{ nodeName }}
                </h5>
                <span class="badge bg-secondary">{{ peerList.length }} peer(s)</span>
              </div>
              <div class="card-body p-0">
                <!-- 节点错误 -->
                <div v-if="isNodeError(peerList)" class="p-3">
                  <div class="alert alert-warning small mb-0">
                    <i class="bi bi-exclamation-triangle me-1"></i>
                    {{ (peerList[0] as any).error }}
                  </div>
                </div>
                <!-- 无 peer -->
                <div v-else-if="peerList.length === 0" class="p-3 text-center text-muted">
                  <i class="bi bi-dash-circle me-1"></i>
                  No peers configured
                </div>
                <!-- Peer 列表表格 -->
                <div v-else class="table-responsive">
                  <table class="table table-hover mb-0">
                    <thead>
                      <tr>
                        <th style="width: 120px">ASN</th>
                        <th style="width: 80px">Status</th>
                        <th>Endpoint</th>
                        <th>IPv4</th>
                        <th>IPv6</th>
                        <th style="width: 100px">BGP</th>
                        <th style="width: 150px">Actions</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr v-for="(peerInfo, idx) in peerList" :key="idx">
                        <template v-if="(peerInfo as any).error">
                          <td colspan="7" class="text-warning small">
                            AS{{ (peerInfo as any).asn }}: {{ (peerInfo as any).error }}
                          </td>
                        </template>
                        <template v-else>
                          <td>
                            <span class="badge bg-info">AS{{ peerInfo.asn }}</span>
                          </td>
                          <td>
                            <span :class="['badge', peerInfo.interface_up ? 'bg-success' : 'bg-secondary']">
                              {{ peerInfo.interface_up ? 'Up' : 'Down' }}
                            </span>
                          </td>
                          <td class="font-monospace small text-truncate" style="max-width: 200px">
                            {{ peerInfo.wg?.endpoint || 'N/A' }}
                          </td>
                          <td class="font-monospace small">{{ peerInfo.wg?.peer_v4 || 'N/A' }}</td>
                          <td class="font-monospace small text-truncate" style="max-width: 150px">
                            {{ peerInfo.wg?.peer_v6 || 'N/A' }}
                          </td>
                          <td class="small">{{ peerInfo.bird?.session_type || 'N/A' }}</td>
                          <td>
                            <div class="btn-group btn-group-sm">
                              <button
                                @click="goToModify(nodeName, peerInfo.asn)"
                                class="btn btn-outline-primary"
                                title="Modify"
                              >
                                <i class="bi bi-pencil"></i>
                              </button>
                              <button
                                @click="showDeleteModal(nodeName, peerInfo.asn)"
                                class="btn btn-outline-danger"
                                title="Delete"
                              >
                                <i class="bi bi-trash"></i>
                              </button>
                            </div>
                          </td>
                        </template>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div v-if="Object.keys(allPeers).length === 0" class="text-center py-5 text-muted">
          <i class="bi bi-inbox display-4"></i>
          <p class="mt-3">No nodes found</p>
        </div>
      </div>
    </template>

    <!-- Delete Modal -->
    <div v-if="deleteModal" class="modal fade show d-block" style="background: rgba(0,0,0,0.5)">
      <div class="modal-dialog modal-dialog-centered">
        <div class="modal-content">
          <div class="modal-header">
            <h5 class="modal-title">Confirm Delete</h5>
            <button type="button" class="btn-close" @click="deleteModal = false"></button>
          </div>
          <div class="modal-body">
            <p>Are you sure you want to delete the peer?</p>
            <p class="mb-0">
              <strong>Node:</strong> {{ deleteTarget?.node }}<br>
              <strong>ASN:</strong> AS{{ deleteTarget?.asn }}
            </p>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn btn-secondary" @click="deleteModal = false">Cancel</button>
            <button type="button" class="btn btn-danger" @click="confirmDelete" :disabled="deleteLoading">
              <span v-if="deleteLoading" class="spinner-border spinner-border-sm me-1"></span>
              Delete
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
