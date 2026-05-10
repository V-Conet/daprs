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

// 判断是否已建立 peer（检查是否有对端配置）
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

    // 短暂延迟后跳转
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
  <div>
    <!-- 标题栏 -->
    <div class="d-flex align-items-center mb-4">
      <button @click="goBack" class="btn btn-outline-secondary btn-sm me-3">
        ← Back
      </button>
      <h2 class="mb-0 flex-grow-1">
        Peer Info: <span class="text-primary">{{ nodeName }}</span>
      </h2>
      <button
        @click="refreshInfo"
        class="btn btn-outline-primary btn-sm me-2"
        :disabled="loading"
      >
        Refresh
      </button>
    </div>

    <!-- 加载中 -->
    <div v-if="loading" class="text-center py-5">
      <div class="spinner-border text-primary" role="status">
        <span class="visually-hidden">Loading...</span>
      </div>
    </div>

    <!-- 成功提示 -->
    <div v-else-if="success" class="alert alert-success text-center py-5">
      <div class="display-4 mb-3">✓</div>
      <h4>{{ success }}</h4>
    </div>

    <!-- 错误提示 -->
    <div v-else-if="error" class="alert alert-danger">
      <h5 class="alert-heading">Error</h5>
      <p class="mb-0">{{ error }}</p>
      <hr>
      <button @click="goBack" class="btn btn-outline-danger">Back to Dashboard</button>
    </div>

    <!-- Peer 信息 -->
    <template v-else-if="peerInfo">
      <!-- 状态卡片 -->
      <div class="row g-4 mb-4">
        <!-- 对端信息 -->
        <div class="col-md-6">
          <div class="card h-100">
            <div class="card-header">
              <h5 class="mb-0">Peer Configuration</h5>
            </div>
            <div class="card-body">
              <template v-if="peerInfo.peer">
                <dl class="row mb-0">
                  <dt class="col-sm-4 text-muted">ASN</dt>
                  <dd class="col-sm-8"><strong>AS{{ peerInfo.asn }}</strong></dd>

                  <dt class="col-sm-4 text-muted">Endpoint</dt>
                  <dd class="col-sm-8 font-monospace small">{{ peerInfo.peer.endpoint || 'N/A' }}</dd>

                  <dt class="col-sm-4 text-muted">IPv4</dt>
                  <dd class="col-sm-8 font-monospace small">{{ peerInfo.peer.v4 || 'N/A' }}</dd>

                  <dt class="col-sm-4 text-muted">IPv6</dt>
                  <dd class="col-sm-8 font-monospace small">{{ peerInfo.peer.v6 || 'N/A' }}</dd>

                  <dt class="col-sm-4 text-muted">PubKey</dt>
                  <dd class="col-sm-8 font-monospace small text-break" style="font-size: 0.7rem;">{{ peerInfo.peer.pubkey || 'N/A' }}</dd>
                </dl>
              </template>
              <div v-else class="text-muted">
                No peer configuration found
              </div>
            </div>
          </div>
        </div>

        <!-- 我的配置 -->
        <div class="col-md-6">
          <div class="card h-100">
            <div class="card-header">
              <h5 class="mb-0">My Configuration</h5>
            </div>
            <div class="card-body">
              <dl class="row mb-0">
                <dt class="col-sm-4 text-muted">IPv4</dt>
                <dd class="col-sm-8 font-monospace small">{{ peerInfo.my_v4 || 'N/A' }}</dd>

                <dt class="col-sm-4 text-muted">IPv6</dt>
                <dd class="col-sm-8 font-monospace small">{{ peerInfo.my_v6 || 'N/A' }}</dd>

                <dt class="col-sm-4 text-muted">LLA</dt>
                <dd class="col-sm-8 font-monospace small">{{ peerInfo.my_lla || 'N/A' }}</dd>

                <dt class="col-sm-4 text-muted">PubKey</dt>
                <dd class="col-sm-8 font-monospace small text-break" style="font-size: 0.7rem;">{{ peerInfo.my_pubkey || 'N/A' }}</dd>
              </dl>
            </div>
          </div>
        </div>
      </div>

      <!-- 操作按钮 -->
      <div class="card mb-4">
        <div class="card-body">
          <div class="d-flex gap-2">
            <router-link :to="`/peering/${nodeName}`" class="btn btn-primary">
              {{ hasPeer ? 'Modify Peer' : 'Add Peer' }}
            </router-link>
            <button
              v-if="hasPeer"
              @click="deletePeer"
              class="btn btn-outline-danger"
              :disabled="deleting"
            >
              <span v-if="deleting" class="spinner-border spinner-border-sm me-2"></span>
              Delete This Peer
            </button>
          </div>
        </div>
      </div>

      <!-- WireGuard 状态 - 仅在有配置时显示 -->
      <div v-if="hasPeer" class="card mb-4">
        <div class="card-header d-flex justify-content-between align-items-center">
          <h5 class="mb-0">WireGuard Status</h5>
          <span :class="peerInfo.interface_up ? 'bg-success' : 'bg-danger'" class="badge">
            Interface {{ peerInfo.interface_up ? 'Up' : 'Down' }}
          </span>
        </div>
        <div class="card-body p-0">
          <template v-if="peerInfo.interface_up && peerInfo.wg_show">
            <pre class="bg-dark text-light p-3 mb-0" style="max-height: 250px; overflow-y: auto; font-size: 0.8rem;">{{ peerInfo.wg_show.output || 'No output' }}</pre>
          </template>
          <template v-else>
            <div class="p-4 text-center">
              <div class="text-warning mb-2">
                <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" fill="currentColor" class="bi bi-exclamation-triangle" viewBox="0 0 16 16">
                  <path d="M7.938 2.016A.13.13 0 0 1 8.002 2a.13.13 0 0 1 .063.016.146.146 0 0 1 .054.057l6.857 11.667c.036.06.035.124.002.183a.163.163 0 0 1-.054.06.116.116 0 0 1-.066.017H1.146a.115.115 0 0 1-.066-.017.163.163 0 0 1-.054-.06.176.176 0 0 1 .002-.183L7.884 2.073a.147.147 0 0 1 .054-.057zm1.044-.45a1.13 1.13 0 0 0-1.96 0L.165 13.233c-.457.778.091 1.767.98 1.767h13.713c.889 0 1.438-.99.98-1.767L8.982 1.566z"/>
                  <path d="M7.002 12a1 1 0 1 1 2 0 1 1 0 0 1-2 0zM7.1 5.995a.905.905 0 1 1 1.8 0l-.35 3.507a.552.552 0 0 1-1.1 0L7.1 5.995z"/>
                </svg>
              </div>
              <p class="text-muted mb-1">WireGuard interface is not running</p>
              <small class="text-muted">Try refreshing or check if <code class="text-info">wg-quick up dn42-{{ peerInfo.asn }}</code> was executed</small>
            </div>
          </template>
        </div>
      </div>

      <!-- BGP 状态 - 仅在有配置且接口在线时显示 -->
      <div v-if="hasPeer && peerInfo.interface_up" class="card">
        <div class="card-header">
          <h5 class="mb-0">BGP Status</h5>
        </div>
        <div class="card-body p-0">
          <div v-if="peerInfo.bird_protocols?.length > 0">
            <div v-for="protocol in peerInfo.bird_protocols" :key="protocol.command" class="border-bottom">
              <div class="px-3 py-2 small text-muted border-bottom">
                <code>{{ protocol.command }}</code>
              </div>
              <pre class="bg-dark text-light p-3 mb-0" style="max-height: 200px; overflow-y: auto; font-size: 0.8rem;">{{ protocol.output || 'No output' }}</pre>
            </div>
          </div>
          <div v-else class="p-4 text-center text-muted">
            No BGP protocol information available
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
