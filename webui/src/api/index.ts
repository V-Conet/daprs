// DAPRS WebUI - API 模块
import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  withCredentials: true,
  headers: {
    'Content-Type': 'application/json'
  },
  timeout: 20000  // 默认 20 秒超时
})

// 命令专用 axios 实例（更长超时）
const cmdApi = axios.create({
  baseURL: '/api',
  withCredentials: true,
  headers: {
    'Content-Type': 'application/json'
  },
  timeout: 35000  // 命令执行 35 秒超时
})

// ============================================================================
// Types
// ============================================================================

export interface NetConfig {
  ipv4: boolean
  ipv6: boolean
  accept_nat: boolean
  cn: boolean
}

export interface Dn42Config {
  asn: number
  ipv4: string
  ipv6: string
  lla: string
  wgkey: string
  ipv4_addr: string
  ipv6_addr: string
}

export interface FrontendConfig {
  version: number
  is_open: boolean
  is_verify: boolean
  extra_msg: string
  net: NetConfig
  dn42: Dn42Config
}

export interface NodeAgentConfig {
  address: string
  online: boolean
  error: string | null
  conf: FrontendConfig
}

export interface PeeringPayload {
  is_mhp: boolean
  is_nhp: boolean
  v4: string | null
  v6: string | null
  lla: string | null
  is_prefer_lla: boolean
  endpoint: string
  pubkey: string
  custom_port: number | null
  psk: string | null
  mtu: number | null
}

export interface NodeActionRequest<T> {
  node: string
  payload: T
}

export interface UserInfo {
  issued_at: number
  expires_at: number
  userinfo: Record<string, unknown>
}

// ============================================================================
// API Functions
// ============================================================================

// 认证
export const logout = () => api.post('/logout')
export const getMe = () => api.get<UserInfo>('/me')

// 节点
export const getNodes = () => api.get<Record<string, NodeAgentConfig>>('/nodes')

// Peering
export const createPeering = (data: NodeActionRequest<PeeringPayload>) =>
  api.post('/peering', data)

export const modifyPeering = (data: NodeActionRequest<PeeringPayload>) =>
  api.post('/modify', data)

export const removePeering = (node: string) =>
  api.post('/remove', { node })

export const getPeers = () =>
  api.get<NodeActionRequest<PeeringPayload>[]>('/peers')

export const getPeerInfo = (node: string) =>
  api.get<PeerInfoResponse>(`/peer/${node}/info`)

// 命令
export const executeCmd = (node: string, cmd: Record<string, unknown>) =>
  cmdApi.post('/cmd', { node, payload: cmd })

export default api

// ============================================================================
// Response Types
// ============================================================================

export interface WgConfig {
  port: number
  mtu: number
  pubkey: string
  psk: string | null
  endpoint: string | null
  peer_v4: string | null
  peer_v6: string | null
}

export interface BirdConfig {
  is_mhp: boolean
  is_nhp: boolean
  session_type: string
}

export interface RawCommandOutput {
  command: string
  output: string
}

export interface PeerInfo {
  pubkey: string
  endpoint: string | null
  v4: string | null
  v6: string | null
}

export interface PeerInfoResponse {
  asn: number
  interface_up: boolean
  wg_show: RawCommandOutput | null
  bird_protocols: RawCommandOutput[]
  my_v4: string
  my_v6: string
  my_lla: string
  my_pubkey: string
  peer: PeerInfo | null
  wg: WgConfig | null
  bird: BirdConfig | null
}

// ============================================================================
// Admin API
// ============================================================================

export interface PendingRequest {
  id: string
  node: string
  asn: number
  payload: PeeringPayload
  created_at: number
}

export interface AdminPeerRequest {
  node: string
  asn: number
  payload: PeeringPayload
}

export interface AdminDeleteRequest {
  node: string
  asn: number
}

// Admin API
export const checkAdmin = () => api.get<boolean>('/admin/check')
export const getPendingRequests = () => api.get<PendingRequest[]>('/admin/pending')
export const approveRequest = (id: string) => api.post(`/admin/pending/${id}/approve`)
export const rejectRequest = (id: string) => api.post(`/admin/pending/${id}/reject`)
export const getAllPeers = () => api.get<Record<string, PeerInfoResponse[]>>('/admin/peers')
export const adminModifyPeer = (data: AdminPeerRequest) => api.post('/admin/peer/modify', data)
export const adminDeletePeer = (data: AdminDeleteRequest) => api.post('/admin/peer/delete', data)

// 用户待处理请求
export const getMyPendingRequests = () => api.get<PendingRequest[]>('/pending')
