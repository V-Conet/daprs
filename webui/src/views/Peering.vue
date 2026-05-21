<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { getNodes, getPeerInfo, createPeering, modifyPeering, type NodeAgentConfig, type PeeringPayload, type PeerInfoResponse } from '../api'
import { useAuthStore } from '../stores/auth'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()

const nodeName = route.params.node as string
const node = ref<NodeAgentConfig | null>(null)
const loading = ref(true)
const submitting = ref(false)
const error = ref<string | null>(null)
const success = ref<string | null>(null)
const isModify = ref(false)
const existingConfig = ref<PeerInfoResponse | null>(null)

// 表单数据 - BGP Extensions 默认启用
const form = ref<PeeringPayload>({
  is_mhp: true,
  is_nhp: true,
  v4: null,
  v6: null,
  lla: null,
  is_prefer_lla: false,
  endpoint: '',
  pubkey: '',
  custom_port: null,
  psk: null,
  mtu: 1420
})

// 监听 MP-BGP 开关，关闭时自动关闭 ENH
function onMhpChange(value: boolean) {
  if (!value) {
    form.value.is_nhp = false
  }
}

// 计算默认端口
const defaultPort = computed(() => {
  const asn = authStore.asn
  if (!asn) return null
  if (asn >= 4242420000) {
    return asn % 100000
  } else {
    return 40000 + (asn % 10000)
  }
})

// 计算实际使用的端口
const actualPort = computed(() => {
  return form.value.custom_port || defaultPort.value
})

// 判断是否为纯 IPv6 地址（包含冒号）
const isIpv6Address = (addr: string): boolean => {
  return addr.includes(':') && !addr.startsWith('[')
}

// 计算 IPv4 Endpoint
const endpointV4 = computed(() => {
  if (!node.value) return null
  const dn42 = node.value.conf.dn42
  const port = actualPort.value
  if (dn42.ipv4_addr) {
    return `${dn42.ipv4_addr}:${port}`
  }
  return null
})

// 计算 IPv6 Endpoint
const endpointV6 = computed(() => {
  if (!node.value) return null
  const dn42 = node.value.conf.dn42
  const port = actualPort.value
  if (dn42.ipv6_addr) {
    // 纯 IPv6 地址需要加方括号，域名不需要
    if (isIpv6Address(dn42.ipv6_addr)) {
      return `[${dn42.ipv6_addr}]:${port}`
    }
    return `${dn42.ipv6_addr}:${port}`
  }
  return null
})

// 节点是否需要验证
const needsVerification = computed(() => node.value?.conf.is_verify === true)

onMounted(async () => {
  try {
    // 加载节点信息
    const response = await getNodes()
    node.value = response.data[nodeName] || null

    if (!node.value) {
      error.value = `Node "${nodeName}" not found`
      loading.value = false
      return
    }

    // 尝试获取现有配置
    try {
      const configResponse = await getPeerInfo(nodeName)
      const data = configResponse.data

      // 检查是否有现有配置
      if (data.wg && data.bird) {
        existingConfig.value = data
        isModify.value = true

        // 填充表单
        form.value = {
          is_mhp: data.bird.is_mhp,
          is_nhp: data.bird.is_nhp,
          v4: data.wg.peer_v4,
          v6: data.wg.peer_v6,
          lla: null,
          is_prefer_lla: false,
          endpoint: data.wg.endpoint || '',
          pubkey: data.wg.pubkey,
          custom_port: data.wg.port || null,
          psk: data.wg.psk,
          mtu: data.wg.mtu || 1420
        }
      }
    } catch {
      // 没有现有配置，是新建模式
      isModify.value = false
    }
  } catch (e) {
    error.value = 'Failed to load node info'
    console.error(e)
  } finally {
    loading.value = false
  }
})

async function submitForm() {
  submitting.value = true
  error.value = null

  // 验证数值字段
  if (form.value.custom_port !== null) {
    if (isNaN(form.value.custom_port) || form.value.custom_port < 1024 || form.value.custom_port > 65535) {
      error.value = 'Port must be between 1024 and 65535'
      submitting.value = false
      return
    }
  }
  if (form.value.mtu !== null) {
    if (isNaN(form.value.mtu) || form.value.mtu < 576 || form.value.mtu > 9000) {
      error.value = 'MTU must be between 576 and 9000'
      submitting.value = false
      return
    }
  }

  // 构建干净的 payload，确保 null 值正确处理
  const payload: PeeringPayload = {
    is_mhp: form.value.is_mhp,
    is_nhp: form.value.is_nhp,
    v4: form.value.v4 || null,
    v6: form.value.v6 || null,
    lla: form.value.lla || null,
    is_prefer_lla: form.value.is_prefer_lla,
    endpoint: form.value.endpoint,
    pubkey: form.value.pubkey,
    custom_port: form.value.custom_port ? Math.floor(form.value.custom_port) : null,
    psk: form.value.psk || null,
    mtu: form.value.mtu ? Math.floor(form.value.mtu) : null
  }

  try {
    if (isModify.value) {
      await modifyPeering({
        node: nodeName,
        payload
      })
      success.value = 'Peering updated successfully!'
    } else {
      await createPeering({
        node: nodeName,
        payload
      })
      if (needsVerification.value) {
        success.value = 'Request submitted! Waiting for admin approval. You can check the status on the Dashboard.'
      } else {
        success.value = 'Peering created successfully!'
      }
    }
    if (!needsVerification.value || isModify.value) {
      setTimeout(() => router.push('/dashboard'), 1500)
    }
  } catch (e: any) {
    error.value = e.response?.data?.error || `Failed to ${isModify.value ? 'modify' : 'create'} peering`
  } finally {
    submitting.value = false
  }
}

function goBack() {
  router.back()
}
</script>

<template>
  <div>
    <div class="d-flex align-items-center mb-4">
      <button @click="goBack" class="btn btn-outline-secondary btn-sm me-3">
        <i class="bi bi-arrow-left"></i>
      </button>
      <h2 class="mb-0">
        <i class="bi me-2" :class="isModify ? 'bi-pencil' : 'bi-plus-circle'"></i>
        {{ isModify ? 'Modify' : 'Add' }} Peering: {{ nodeName }}
      </h2>
    </div>

    <div v-if="loading" class="text-center py-5">
      <div class="spinner-border text-primary" role="status">
        <span class="visually-hidden">Loading...</span>
      </div>
    </div>

    <div v-else-if="error && !node" class="alert alert-danger">
      {{ error }}
    </div>

    <template v-else-if="node">
      <div class="card shadow-sm">
        <div class="card-body">
          <form @submit.prevent="submitForm">
            <!-- 节点信息 -->
            <div class="alert alert-info">
              <strong>Your ASN:</strong> {{ authStore.asn }} |
              <strong>Target ASN:</strong> {{ node.conf.dn42.asn }}
              <span v-if="isModify" class="badge bg-success ms-2">Existing Peer</span>
              <span v-else-if="needsVerification" class="badge bg-warning text-dark ms-2">Requires Approval</span>
            </div>

            <!-- 验证提示 -->
            <div v-if="needsVerification && !isModify" class="alert alert-warning mb-3">
              <i class="bi bi-exclamation-triangle me-2"></i>
              <strong>Verification Required:</strong> Your peering request will be submitted for admin approval.
              You can check the status on the Dashboard after submission.
            </div>

            <!-- 成功提示 -->
            <div v-if="success" class="alert alert-success mb-3">
              <i class="bi bi-check-circle me-2"></i>
              {{ success }}
            </div>

            <!-- 现有配置提示 -->
            <div v-if="isModify && existingConfig?.bird" class="alert alert-secondary mb-3">
              <small>
                <strong>Current BGP Session:</strong> {{ existingConfig.bird.session_type }}
                <span v-if="existingConfig.wg?.port"> | Port: {{ existingConfig.wg.port }}</span>
              </small>
            </div>

            <!-- WireGuard 配置 -->
            <h5 class="mb-3">
              <i class="bi bi-shield-lock me-1"></i>
              WireGuard Configuration
            </h5>

            <div class="row g-3 mb-4">
              <div class="col-md-6">
                <label class="form-label">Endpoint *</label>
                <input
                  v-model="form.endpoint"
                  type="text"
                  class="form-control font-monospace"
                  placeholder="e.g., example.com:51820"
                  required
                />
                <div class="form-text">Your WireGuard endpoint</div>
              </div>

              <div class="col-md-6">
                <label class="form-label">Public Key *</label>
                <input
                  v-model="form.pubkey"
                  type="text"
                  class="form-control font-monospace"
                  placeholder="WireGuard public key"
                  required
                />
              </div>
            </div>

            <div class="row g-3 mb-4">
              <div class="col-md-4">
                <label class="form-label">Pre-Shared Key</label>
                <input
                  v-model="form.psk"
                  type="text"
                  class="form-control font-monospace"
                  placeholder="Optional PSK"
                />
                <div class="form-text">Optional: provides post-quantum security</div>
              </div>

              <div class="col-md-4">
                <label class="form-label">MTU</label>
                <input
                  v-model.number="form.mtu"
                  type="number"
                  class="form-control"
                  placeholder="1420"
                  min="576"
                  max="9000"
                />
              </div>

              <div class="col-md-4">
                <label class="form-label">Port</label>
                <input
                  v-model.number="form.custom_port"
                  type="number"
                  class="form-control"
                  :placeholder="`Default: ${defaultPort}`"
                  min="1024"
                  max="65535"
                />
                <div class="form-text">Leave empty for auto: {{ defaultPort }}</div>
              </div>
            </div>

            <!-- IP 地址配置 -->
            <h5 class="mb-3">
              <i class="bi bi-diagram-3 me-1"></i>
              IP Address Configuration
            </h5>

            <div class="row g-3 mb-4">
              <div class="col-md-6">
                <label class="form-label">Tunnel IPv4</label>
                <input
                  v-model="form.v4"
                  type="text"
                  class="form-control font-monospace"
                  placeholder="DN42 IPv4 address"
                />
              </div>

              <div class="col-md-6">
                <label class="form-label">Tunnel IPv6</label>
                <input
                  v-model="form.v6"
                  type="text"
                  class="form-control font-monospace"
                  placeholder="DN42 IPv6 or Link-Local address"
                />
                <div class="form-text">Link-local address recommended</div>
              </div>
            </div>

            <!-- BGP 扩展 -->
            <h5 class="mb-3">
              <i class="bi bi-sliders me-1"></i>
              BGP Extensions
            </h5>

            <div class="card border mb-4">
              <div class="card-body">
                <div class="form-check form-switch mb-2">
                  <input
                    v-model="form.is_mhp"
                    class="form-check-input"
                    type="checkbox"
                    id="mpbgp"
                    @change="onMhpChange(form.is_mhp)"
                  />
                  <label class="form-check-label" for="mpbgp">
                    <strong>Multiprotocol BGP (MP-BGP)</strong>
                    <span class="d-block form-text">Single BGP session for both IPv4 and IPv6 routes</span>
                  </label>
                </div>

                <div class="form-check form-switch">
                  <input
                    v-model="form.is_nhp"
                    class="form-check-input"
                    type="checkbox"
                    id="enexthop"
                    :disabled="!form.is_mhp"
                  />
                  <label class="form-check-label" for="enexthop">
                    <strong>Extended Next Hop (IPv6 session)</strong>
                    <span class="d-block form-text">Use IPv6 session to carry IPv4 routes (ENH)</span>
                  </label>
                </div>
              </div>
            </div>

            <!-- 提供给对方的信息 -->
            <h5 class="mb-3">
              <i class="bi bi-share me-1"></i>
              Information for Your Side
            </h5>
            <p class="text-muted small mb-3">Provide this information to the peer for their configuration.</p>

            <div class="card border mb-4">
              <div class="card-body">
                <div class="row">
                  <div class="col-md-6">
                    <dl class="row mb-0">
                      <dt class="col-sm-4 text-muted small">IPv4 Endpoint</dt>
                      <dd class="col-sm-8 font-monospace small">
                        <code v-if="endpointV4">{{ endpointV4 }}</code>
                        <span v-else class="text-warning">Not configured</span>
                      </dd>

                      <dt class="col-sm-4 text-muted small">IPv6 Endpoint</dt>
                      <dd class="col-sm-8 font-monospace small">
                        <code v-if="endpointV6">{{ endpointV6 }}</code>
                        <span v-else class="text-warning">Not configured</span>
                      </dd>

                      <dt class="col-sm-4 text-muted small">Public Key</dt>
                      <dd class="col-sm-8 font-monospace small text-break">
                        <code style="font-size: 0.7rem;">{{ node?.conf.dn42.wgkey || 'N/A' }}</code>
                      </dd>
                    </dl>
                  </div>
                  <div class="col-md-6">
                    <dl class="row mb-0">
                      <dt class="col-sm-4 text-muted small">DN42 IPv4</dt>
                      <dd class="col-sm-8 font-monospace small">
                        <code v-if="node?.conf.dn42.ipv4">{{ node.conf.dn42.ipv4 }}</code>
                        <span v-else class="text-warning">Not configured</span>
                      </dd>

                      <dt class="col-sm-4 text-muted small">DN42 IPv6</dt>
                      <dd class="col-sm-8 font-monospace small">
                        <code v-if="node?.conf.dn42.ipv6">{{ node.conf.dn42.ipv6 }}</code>
                        <span v-else class="text-warning">Not configured</span>
                      </dd>

                      <dt class="col-sm-4 text-muted small">Link-Local</dt>
                      <dd class="col-sm-8 font-monospace small">
                        <code v-if="node?.conf.dn42.lla">{{ node.conf.dn42.lla }}</code>
                        <span v-else class="text-warning">Not configured</span>
                      </dd>
                    </dl>
                  </div>
                </div>
              </div>
            </div>

            <!-- 错误提示 -->
            <div v-if="error" class="alert alert-danger">
              {{ error }}
            </div>

            <!-- 提交按钮 -->
            <div class="d-flex justify-content-end gap-2">
              <button type="button" @click="goBack" class="btn btn-outline-secondary">
                Cancel
              </button>
              <button
                type="submit"
                class="btn"
                :class="isModify ? 'btn-warning' : 'btn-primary'"
                :disabled="submitting"
              >
                <span v-if="submitting" class="spinner-border spinner-border-sm me-1"></span>
                {{ isModify ? 'Update Peering' : 'Create Peering' }}
              </button>
            </div>
          </form>
        </div>
      </div>
    </template>
  </div>
</template>
