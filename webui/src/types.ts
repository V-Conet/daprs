export type RoutingPolicy = 'FullTable' | 'Transit' | 'PeeringOnly' | 'Downstream'

export interface FrontendAgentConfig {
  version: number
  is_open: boolean
  is_verify: boolean
  extra_msg: string
  net: NetConfig
  dn42: Dn42Config
}

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
}

export interface NodeAgentConfig {
  address: string
  online: boolean
  error: string | null
  conf: FrontendAgentConfig
}

export interface PeeringPayload {
  is_mhp: boolean
  is_nhp: boolean
  policy: RoutingPolicy
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

export interface MeResponse {
  issued_at: number
  expires_at: number
  userinfo: Record<string, unknown>
}

export interface RemoveRequest {
  node: string
}