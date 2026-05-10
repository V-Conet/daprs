<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { getNodes, type NodeAgentConfig } from '../api'

const router = useRouter()
const nodes = ref<Record<string, NodeAgentConfig>>({})
const loading = ref(true)
const error = ref<string | null>(null)

onMounted(async () => {
  await loadNodes()
})

async function loadNodes() {
  loading.value = true
  error.value = null
  try {
    const response = await getNodes()
    nodes.value = response.data
  } catch (e: any) {
    if (e.response?.status === 401) {
      // 未登录，跳转登录
      window.location.href = '/api/login'
      return
    }
    error.value = 'Failed to load nodes. Please check if server is running.'
    console.error(e)
  } finally {
    loading.value = false
  }
}

function goToPeering(nodeName: string) {
  router.push(`/peering/${nodeName}`)
}

function goToPeerInfo(nodeName: string) {
  router.push(`/peer/${nodeName}`)
}

function getStatusBadge(online: boolean) {
  return online ? 'bg-success' : 'bg-danger'
}

function getStatusText(online: boolean) {
  return online ? 'Online' : 'Offline'
}

// 获取限制标签
function getRestrictions(node: NodeAgentConfig): string[] {
  const restrictions: string[] = []
  if (!node.conf.net.accept_nat) {
    restrictions.push('No NAT')
  }
  if (!node.conf.net.cn) {
    restrictions.push('No CN')
  }
  return restrictions
}
</script>

<template>
  <div>
    <div class="d-flex justify-content-between align-items-center mb-4">
      <h2 class="mb-0">Available Nodes</h2>
      <button @click="loadNodes" class="btn btn-outline-primary btn-sm" :disabled="loading">
        Refresh
      </button>
    </div>

    <!-- 错误提示 -->
    <div v-if="error" class="alert alert-danger">
      {{ error }}
      <button @click="loadNodes" class="btn btn-sm btn-outline-danger ms-2">Retry</button>
    </div>

    <!-- 加载中 -->
    <div v-if="loading" class="text-center py-5">
      <div class="spinner-border text-primary" role="status">
        <span class="visually-hidden">Loading...</span>
      </div>
    </div>

    <!-- 节点列表 -->
    <div v-else class="row g-4">
      <div v-for="(node, name) in nodes" :key="name" class="col-md-6 col-lg-4">
        <div class="card h-100 shadow-sm">
          <div class="card-header d-flex justify-content-between align-items-center">
            <h5 class="mb-0">{{ name }}</h5>
            <span :class="['badge', getStatusBadge(node.online)]">
              {{ getStatusText(node.online) }}
            </span>
          </div>

          <div class="card-body">
            <!-- 节点信息 -->
            <dl class="row mb-3">
              <dt class="col-sm-4 text-muted small">ASN</dt>
              <dd class="col-sm-8">{{ node.conf.dn42.asn || 'N/A' }}</dd>

              <dt class="col-sm-4 text-muted small">IPv4</dt>
              <dd class="col-sm-8 font-monospace small">{{ node.conf.dn42.ipv4 || 'N/A' }}</dd>

              <dt class="col-sm-4 text-muted small">IPv6</dt>
              <dd class="col-sm-8 font-monospace small text-truncate">{{ node.conf.dn42.ipv6 || 'N/A' }}</dd>

              <dt class="col-sm-4 text-muted small">Network</dt>
              <dd class="col-sm-8">
                <span v-if="node.conf.net.ipv4" class="badge bg-light text-dark me-1">IPv4</span>
                <span v-if="node.conf.net.ipv6" class="badge bg-light text-dark me-1">IPv6</span>
                <span v-if="node.conf.net.cn" class="badge bg-warning text-dark">CN</span>
              </dd>

              <dt class="col-sm-4 text-muted small">Open</dt>
              <dd class="col-sm-8">
                <span :class="['badge', node.conf.is_open ? 'bg-success' : 'bg-secondary']">
                  {{ node.conf.is_open ? 'Yes' : 'No' }}
                </span>
                <span v-if="node.conf.is_verify" class="badge bg-info ms-1">Verify</span>
              </dd>

              <!-- 限制条件 -->
              <template v-if="getRestrictions(node).length > 0">
                <dt class="col-sm-4 text-muted small">Limits</dt>
                <dd class="col-sm-8">
                  <span v-for="r in getRestrictions(node)" :key="r" class="badge bg-secondary me-1">{{ r }}</span>
                </dd>
              </template>
            </dl>

            <!-- 错误信息 -->
            <div v-if="node.error" class="alert alert-warning small mb-0">
              {{ node.error }}
            </div>

            <!-- 额外信息 -->
            <div v-else-if="node.conf.extra_msg" class="alert alert-info small mb-0">
              {{ node.conf.extra_msg }}
            </div>
          </div>

          <div class="card-footer">
            <div class="btn-group w-100">
              <!-- 添加 Peer -->
              <button
                @click="goToPeering(name)"
                class="btn btn-primary btn-sm"
                :disabled="!node.online || !node.conf.is_open"
                :title="!node.online ? 'Node offline' : !node.conf.is_open ? 'Peering closed' : 'Add new peer'"
              >
                + Add Peer
              </button>

              <!-- 查看 Peer 信息 -->
              <button
                @click="goToPeerInfo(name)"
                class="btn btn-outline-secondary btn-sm"
                :disabled="!node.online"
                title="View peer info"
              >
                Info
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 空状态 -->
    <div v-if="!loading && Object.keys(nodes).length === 0" class="text-center py-5 text-muted">
      <div class="display-4 mb-3">📡</div>
      <h5>No nodes configured</h5>
      <p>Add nodes to server.toml configuration</p>
    </div>
  </div>
</template>

<style scoped>
.font-monospace {
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
}
</style>
