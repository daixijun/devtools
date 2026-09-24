import { invoke } from '@tauri-apps/api/core'
import React, { useEffect, useState } from 'react'
import { ToolLayout } from '../components/layouts'

interface CipherSuite {
  name: string
  /** 接受该套件的协议版本（真实探测结果） */
  versions: string[]
  strength: string
  server_order: boolean
}

interface SecurityVulnerability {
  id: string
  name: string
  description: string
  severity: string
  /** "protocol" / "cipher" / "certificate" / "key-exchange" */
  category: string
  /** 实际观测到的证据 */
  evidence: string
  cve_ids: string[]
  affected_components: string[]
  remediation: string
  references: string[]
  /** 等级封顶（"F"/"C"/"B"/"A-"），空字符串表示不封顶 */
  grade_cap: string
}

interface ProtocolSupport {
  version: string
  supported: boolean
  cipher_suites: CipherSuite[]
  alpn_protocols?: string[]
  http2_support?: boolean
  spdy_support?: boolean
  http3_support?: boolean
}

interface CertificateChainNode {
  certificate: {
    subject: string
    issuer: string
    valid_from: string
    valid_to: string
    fingerprint: string
    serial_number: string
    signature_algorithm: string
    public_key_algorithm: string
    key_size?: number
    san_domains?: string[]
  }
  is_root: boolean
  is_leaf: boolean
  /** 链级别：0 = 终端证书，1 = 中间 CA，2 = 根 CA */
  chain_level: number
  trust_status: string
  validation_errors: string[]
}

interface CertificateChain {
  certificates: CertificateChainNode[]
  chain_length: number
  is_complete: boolean
  /** 服务器是否发送了根（或交叉签名根）证书 */
  root_in_chain: boolean
  /** 链最终锚定到的受信任根 CA 名称 */
  trust_anchor_info?: string
  root_ca_info?: string
  chain_validation_status: string
  chain_errors: string[]
}

interface SslInfo {
  domain: string
  server_ip?: string
  server_info?: string
  certificate?: {
    subject: string
    issuer: string
    valid_from: string
    valid_to: string
    fingerprint: string
    serial_number: string
    signature_algorithm: string
    public_key_algorithm: string
    key_size?: number
    san_domains?: string[]
  }
  certificate_chain?: CertificateChain
  ssl_versions?: string[]
  cipher_suites?: CipherSuite[]
  protocol_support?: ProtocolSupport[]
  server_cipher_order?: boolean
  security_score?: number
  ssl_labs_rating?: {
    grade: string
    score: number
    has_warnings: boolean
    has_errors: boolean
    certificate_score: number
    protocol_score: number
    key_exchange_score: number
    cipher_strength_score: number
    /** 生效的等级封顶说明 */
    applied_caps: { finding: string; cap: string; reason: string }[]
    details: string
  }
  vulnerabilities?: string[]
  recommendations?: string[]
  cve_vulnerabilities?: SecurityVulnerability[]
  http2_support?: boolean
  spdy_support?: boolean
  http3_support?: boolean
  alpn_protocols?: string[]
}

const SslChecker: React.FC = () => {
  const [domain, setDomain] = useState('')
  const [sslInfo, setSslInfo] = useState<SslInfo | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [toast, setToast] = useState('')
  const [showToast, setShowToast] = useState(false)
  const [activeTab, setActiveTab] = useState<
    'overview' | 'certificate' | 'chain' | 'cipher' | 'cve' | 'security'
  >('overview')
  const [severityFilter, setSeverityFilter] = useState<
    'ALL' | 'CRITICAL' | 'HIGH' | 'MEDIUM' | 'LOW'
  >('ALL')
  const [recentDomains, setRecentDomains] = useState<string[]>([])
  const [showResults, setShowResults] = useState(false)

  // 加载最近检测的域名
  useEffect(() => {
    const storedDomains = localStorage.getItem('ssl-checker-recent-domains')
    if (storedDomains) {
      try {
        const domains = JSON.parse(storedDomains) as string[]
        setRecentDomains(domains)
      } catch (e) {
        console.warn('Failed to parse recent domains from localStorage:', e)
      }
    }
  }, [])

  // 添加域名到最近检测列表
  const addToRecentDomains = (domainToAdd: string) => {
    const trimmedDomain = domainToAdd.trim().toLowerCase()
    if (!trimmedDomain) return

    setRecentDomains((prevDomains) => {
      const newDomains = [
        trimmedDomain,
        ...prevDomains.filter((d) => d !== trimmedDomain),
      ].slice(0, 10)
      localStorage.setItem(
        'ssl-checker-recent-domains',
        JSON.stringify(newDomains),
      )
      return newDomains
    })
  }

  // 从最近检测列表中移除域名
  const removeFromRecentDomains = (domainToRemove: string) => {
    setRecentDomains((prevDomains) => {
      const newDomains = prevDomains.filter((d) => d !== domainToRemove)
      localStorage.setItem(
        'ssl-checker-recent-domains',
        JSON.stringify(newDomains),
      )
      return newDomains
    })
  }

  // 点击最近域名进行检测
  const handleRecentDomainClick = (clickedDomain: string) => {
    setDomain(clickedDomain)
    handleCheck(clickedDomain)
  }

  const showToastMessage = (msg: string, timeout = 2000) => {
    setToast(msg)
    setShowToast(true)
    setTimeout(() => setShowToast(false), timeout)
  }

  const validateDomain = (domain: string): boolean => {
    const domainRegex =
      /^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/i
    return domainRegex.test(domain.trim())
  }

  const handleCheck = (domainToCheck?: string) => {
    const trimmedDomain = (domainToCheck || domain).trim()
    if (!trimmedDomain) {
      setError('请输入域名')
      return
    }

    if (!validateDomain(trimmedDomain)) {
      setError('请输入有效的域名格式')
      return
    }

    setLoading(true)
    setError('')
    setSslInfo(null)

    invoke<SslInfo>('check_ssl_info', {
      domain: trimmedDomain,
    })
      .then((response) => {
        setSslInfo(response)
        setShowResults(true)
        showToastMessage('SSL 检测完成')
        // 添加到最近检测列表
        addToRecentDomains(trimmedDomain)
      })
      .catch((err) => {
        const message = err instanceof Error ? err.message : String(err)
        console.error('SSL check failed:', err)
        setError('检测失败: ' + message)
      })
      .finally(() => {
        setLoading(false)
      })
  }

  const copyToClipboard = (text: string) => {
    navigator.clipboard
      .writeText(text)
      .then(() => {
        showToastMessage('已复制到剪贴板')
      })
      .catch(() => {
        showToastMessage('复制失败')
      })
  }

  const formatDate = (dateStr: string) => {
    try {
      return new Date(dateStr).toLocaleString('zh-CN')
    } catch {
      return dateStr
    }
  }

  const extractCN = (dn: string) => {
    // Extract CN (Common Name) from Distinguished Name
    const parts = dn.split(',')
    for (const part of parts) {
      const trimmed = part.trim()
      if (trimmed.startsWith('CN=') || trimmed.startsWith('cn=')) {
        return trimmed.substring(3)
      }
    }
    return dn // Return full DN if CN not found
  }

  const getSecurityColor = (score?: number) => {
    if (!score) return 'text-slate-500'
    if (score >= 90) return 'text-green-600 dark:text-green-400'
    if (score >= 70) return 'text-yellow-600 dark:text-yellow-400'
    if (score >= 50) return 'text-orange-600 dark:text-orange-400'
    return 'text-red-600 dark:text-red-400'
  }

  const getSecurityLabel = (score?: number) => {
    if (!score) return '未知'
    if (score >= 90) return '优秀'
    if (score >= 70) return '良好'
    if (score >= 50) return '一般'
    return '较差'
  }

  const getSslLabsGradeColor = (grade?: string) => {
    if (!grade) return 'text-slate-500'
    switch (grade) {
      case 'A+':
        return 'text-green-600 dark:text-green-400'
      case 'A':
        return 'text-green-500 dark:text-green-400'
      case 'A-':
        return 'text-green-400 dark:text-green-300'
      case 'B':
        return 'text-yellow-600 dark:text-yellow-400'
      case 'C':
        return 'text-orange-600 dark:text-orange-400'
      case 'D':
        return 'text-red-600 dark:text-red-400'
      case 'F':
        return 'text-red-700 dark:text-red-300'
      default:
        return 'text-slate-500'
    }
  }

  const handleBackToInput = () => {
    setShowResults(false)
    setActiveTab('overview')
  }

  return (
    <ToolLayout
      title='在线SSL检测'
      subtitle='检测域名的SSL证书信息、支持的协议版本、加密套件和安全配置'
      actions={
        showResults && sslInfo ? (
          <button
            onClick={handleBackToInput}
            className='flex items-center gap-2 px-4 py-2 bg-slate-600 text-white rounded-lg hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-slate-500'>
            <svg
              className='w-4 h-4'
              fill='none'
              stroke='currentColor'
              viewBox='0 0 24 24'>
              <path
                strokeLinecap='round'
                strokeLinejoin='round'
                strokeWidth={2}
                d='M10 19l-7-7m0 0l7-7m-7 7h18'
              />
            </svg>
            返回
          </button>
        ) : undefined
      }>
      {!showResults ? (
        // 输入界面
        <div className='space-y-6'>
          <div className='mb-6'>
            <div className='flex items-center gap-2 mb-4'>
              <label className='text-lg font-medium text-slate-700 dark:text-slate-300 whitespace-nowrap'>
                域名:
              </label>
              <div className='flex items-center flex-1 min-w-0 w-full max-w-md border border-slate-300 rounded-lg shadow-sm focus-within:ring-2 focus-within:ring-primary-500 dark:border-slate-600'>
                <span className='px-3 py-2 bg-slate-50 dark:bg-slate-600 text-slate-500 dark:text-slate-400 text-sm border-r border-slate-300 dark:border-slate-600 rounded-l-md'>
                  https://
                </span>
                <input
                  type='text'
                  value={domain}
                  onChange={(e) => setDomain(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      handleCheck()
                    }
                  }}
                  placeholder='example.com'
                  className='flex-1 px-3 py-2 border-0 focus:outline-none focus:ring-0 bg-white dark:bg-slate-700 dark:text-white rounded-r-md'
                  style={{ minWidth: '200px' }}
                />
              </div>
              <button
                type='button'
                onClick={() => handleCheck()}
                disabled={loading}
                className='px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-primary-500 disabled:opacity-50 whitespace-nowrap'>
                {loading ? '检测中...' : '开始检测'}
              </button>
            </div>
            <p className='text-sm text-slate-600 dark:text-slate-400'>
              检测域名的 SSL 证书信息、支持的协议版本、加密套件和安全配置，包括
              HTTP/2 和 HTTP/3 支持性检测。
              <span className='block mt-1'>
                <span className='font-medium'>格式示例:</span>
                <code className='ml-1 px-1 bg-slate-100 dark:bg-slate-700 rounded text-xs'>
                  example.com
                </code>
                、
                <code className='ml-1 px-1 bg-slate-100 dark:bg-slate-700 rounded text-xs'>
                  www.example.com
                </code>
                、
                <code className='ml-1 px-1 bg-slate-100 dark:bg-slate-700 rounded text-xs'>
                  api.example.com
                </code>
              </span>
            </p>

            {/* 最近检测的域名 */}
            {recentDomains.length > 0 && (
              <div className='mt-4'>
                <div className='flex items-center justify-between mb-2'>
                  <span className='text-sm font-medium text-slate-700 dark:text-slate-300'>
                    最近检测的域名:
                  </span>
                  <button
                    onClick={() => {
                      setRecentDomains([])
                      localStorage.removeItem('ssl-checker-recent-domains')
                    }}
                    className='text-xs text-slate-500 hover:text-red-600 dark:text-slate-400 dark:hover:text-red-400 flex items-center gap-1'>
                    <svg
                      className='w-3 h-3'
                      fill='none'
                      stroke='currentColor'
                      viewBox='0 0 24 24'>
                      <path
                        strokeLinecap='round'
                        strokeLinejoin='round'
                        strokeWidth={2}
                        d='M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16'
                      />
                    </svg>
                    清除历史
                  </button>
                </div>
                <div className='flex flex-wrap gap-2'>
                  {recentDomains.map((recentDomain, index) => (
                    <div
                      key={index}
                      className='group relative flex items-center bg-slate-100 dark:bg-slate-700 rounded-lg overflow-hidden hover:bg-primary-50 dark:hover:bg-primary-900/20 transition-colors'>
                      <button
                        onClick={() => handleRecentDomainClick(recentDomain)}
                        className='px-3 py-2 text-sm text-slate-700 dark:text-slate-300 hover:text-primary-600 dark:hover:text-primary-400 transition-colors flex items-center gap-2'
                        title={`检测 ${recentDomain}`}>
                        <svg
                          className='w-4 h-4 text-slate-400 group-hover:text-primary-500'
                          fill='none'
                          stroke='currentColor'
                          viewBox='0 0 24 24'>
                          <path
                            strokeLinecap='round'
                            strokeLinejoin='round'
                            strokeWidth={2}
                            d='M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z'
                          />
                        </svg>
                        <span className='truncate max-w-40'>
                          {recentDomain}
                        </span>
                      </button>
                      <button
                        onClick={(e) => {
                          e.stopPropagation()
                          removeFromRecentDomains(recentDomain)
                        }}
                        className='p-2 text-slate-400 hover:text-red-500 dark:hover:text-red-400 opacity-0 group-hover:opacity-100 transition-all'
                        title={`删除 ${recentDomain}`}>
                        <svg
                          className='w-3 h-3'
                          fill='none'
                          stroke='currentColor'
                          viewBox='0 0 24 24'>
                          <path
                            strokeLinecap='round'
                            strokeLinejoin='round'
                            strokeWidth={2}
                            d='M6 18L18 6M6 6l12 12'
                          />
                        </svg>
                      </button>
                    </div>
                  ))}
                </div>
                <p className='text-xs text-slate-500 dark:text-slate-400 mt-2'>
                  💡 点击域名可直接检测，最多保存10个最近检测的域名
                </p>
              </div>
            )}
          </div>

          {error && (
            <div className='mb-4 p-3 bg-red-100 text-red-700 rounded-lg dark:bg-red-900 dark:text-red-100'>
              {error}
            </div>
          )}

          {loading && (
            <div className='mb-4 p-6 bg-slate-100 dark:bg-slate-800 rounded-lg animate-pulse'>
              <div className='h-4 bg-slate-300 dark:bg-slate-600 rounded mb-2'></div>
              <div className='h-4 bg-slate-300 dark:bg-slate-600 rounded w-3/4 mb-2'></div>
              <div className='h-4 bg-slate-300 dark:bg-slate-600 rounded w-1/2'></div>
            </div>
          )}
        </div>
      ) : (
        // 结果展示界面 - 添加滚动支持
        <div className='flex-1 overflow-y-auto'>
          {sslInfo && (
            <div className='bg-white dark:bg-slate-800 rounded-lg shadow-md'>
              {/* Tab Navigation */}
              <div className='border-b border-slate-200 dark:border-slate-700'>
                <nav className='flex space-x-8 px-6'>
                  {[
                    { key: 'overview', label: '概览' },
                    { key: 'certificate', label: '证书信息' },
                    { key: 'chain', label: '证书链' },
                    { key: 'cipher', label: '加密套件' },
                    { key: 'cve', label: 'CVE 漏洞' },
                    { key: 'security', label: '安全评估' },
                  ].map(({ key, label }) => (
                    <button
                      key={key}
                      onClick={() => setActiveTab(key as any)}
                      className={`py-4 px-1 border-b-2 font-medium text-sm ${
                        activeTab === key
                          ? 'border-primary-500 text-primary-600 dark:text-primary-400'
                          : 'border-transparent text-slate-500 hover:text-slate-700 hover:border-slate-300 dark:text-slate-400 dark:hover:text-slate-300'
                      }`}>
                      {label}
                    </button>
                  ))}
                </nav>
              </div>

              <div className='p-6'>
                {/* Overview Tab */}
                {activeTab === 'overview' && (
                  <div className='space-y-4'>
                    <div className='grid grid-cols-1 lg:grid-cols-2 gap-4'>
                      {/* Basic Info Column */}
                      <div className='space-y-3'>
                        <div className='flex items-center justify-between'>
                          <span className='text-sm text-slate-600 dark:text-slate-400'>
                            域名:
                          </span>
                          <div className='flex items-center gap-2'>
                            <span className='font-medium text-slate-900 dark:text-white'>
                              {sslInfo.domain}
                            </span>
                            <button
                              onClick={() => copyToClipboard(sslInfo.domain)}
                              className='p-1 hover:bg-slate-100 dark:hover:bg-slate-700 rounded'>
                              <svg
                                className='w-4 h-4'
                                fill='none'
                                stroke='currentColor'
                                viewBox='0 0 24 24'>
                                <path
                                  strokeLinecap='round'
                                  strokeLinejoin='round'
                                  strokeWidth={2}
                                  d='M8 7V3h8v4M8 7h8v13H8z'
                                />
                              </svg>
                            </button>
                          </div>
                        </div>

                        {sslInfo.server_ip && (
                          <div className='flex items-center justify-between'>
                            <span className='text-sm text-slate-600 dark:text-slate-400'>
                              服务器 IP:
                            </span>
                            <div className='flex items-center gap-2'>
                              <span className='font-medium text-slate-900 dark:text-white'>
                                {sslInfo.server_ip}
                              </span>
                              <button
                                onClick={() =>
                                  copyToClipboard(sslInfo.server_ip!)
                                }
                                className='p-1 hover:bg-slate-100 dark:hover:bg-slate-700 rounded'>
                                <svg
                                  className='w-4 h-4'
                                  fill='none'
                                  stroke='currentColor'
                                  viewBox='0 0 24 24'>
                                  <path
                                    strokeLinecap='round'
                                    strokeLinejoin='round'
                                    strokeWidth={2}
                                    d='M8 7V3h8v4M8 7h8v13H8z'
                                  />
                                </svg>
                              </button>
                            </div>
                          </div>
                        )}

                        {sslInfo.server_info && (
                          <div className='flex items-center justify-between'>
                            <span className='text-sm text-slate-600 dark:text-slate-400'>
                              服务器版本:
                            </span>
                            <span className='font-medium text-slate-900 dark:text-white'>
                              {sslInfo.server_info}
                            </span>
                          </div>
                        )}

                        {sslInfo.security_score !== undefined && (
                          <div className='flex items-center justify-between'>
                            <span className='text-sm text-slate-600 dark:text-slate-400'>
                              安全评分:
                            </span>
                            <div className='flex items-center gap-2'>
                              <span
                                className={`font-bold text-lg ${getSecurityColor(
                                  sslInfo.security_score,
                                )}`}>
                                {sslInfo.security_score}/100
                              </span>
                              <span
                                className={`text-sm ${getSecurityColor(
                                  sslInfo.security_score,
                                )}`}>
                                ({getSecurityLabel(sslInfo.security_score)})
                              </span>
                            </div>
                          </div>
                        )}

                        {sslInfo.ssl_labs_rating && (
                          <div className='flex items-center justify-between'>
                            <span className='text-sm text-slate-600 dark:text-slate-400'>
                              SSL Labs 评级:
                            </span>
                            <div className='flex items-center gap-2'>
                              <span
                                className={`font-bold text-2xl ${getSslLabsGradeColor(
                                  sslInfo.ssl_labs_rating.grade,
                                )}`}>
                                {sslInfo.ssl_labs_rating.grade}
                              </span>
                              <div className='text-sm text-slate-600 dark:text-slate-400'>
                                <div>{sslInfo.ssl_labs_rating.score}/100</div>
                                <div className='text-xs'>
                                  {sslInfo.ssl_labs_rating.has_errors &&
                                    '有错误'}
                                  {sslInfo.ssl_labs_rating.has_warnings &&
                                    !sslInfo.ssl_labs_rating.has_errors &&
                                    '有警告'}
                                  {!sslInfo.ssl_labs_rating.has_errors &&
                                    !sslInfo.ssl_labs_rating.has_warnings &&
                                    '无问题'}
                                </div>
                              </div>
                            </div>
                          </div>
                        )}

                        {/* Security Findings Indicator */}
                        {sslInfo.cve_vulnerabilities &&
                          sslInfo.cve_vulnerabilities.length > 0 && (
                            <div className='flex items-center justify-between'>
                              <span className='text-sm text-slate-600 dark:text-slate-400'>
                                安全发现:
                              </span>
                              <div className='flex items-center gap-2'>
                                <span
                                  className={`px-2 py-1 text-xs rounded-full ${
                                    sslInfo.cve_vulnerabilities.some(
                                      (v) => v.severity === 'CRITICAL',
                                    )
                                      ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                                      : sslInfo.cve_vulnerabilities.some(
                                          (v) => v.severity === 'HIGH',
                                        )
                                      ? 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200'
                                      : 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                                  }`}>
                                  {sslInfo.cve_vulnerabilities.length} 项待处理
                                </span>
                              </div>
                            </div>
                          )}
                      </div>

                      {/* Certificate & Protocol Info Column */}
                      <div className='space-y-3'>
                        {sslInfo.ssl_versions &&
                          sslInfo.ssl_versions.length > 0 && (
                            <div className='flex items-center justify-between'>
                              <span className='text-sm text-slate-600 dark:text-slate-400'>
                                支持的 SSL/TLS 版本:
                              </span>
                              <div className='flex flex-wrap gap-2 justify-end'>
                                {sslInfo.ssl_versions.map((version, index) => {
                                  const isDeprecated =
                                    version.includes('TLS 1.0') ||
                                    version.includes('TLS 1.1') ||
                                    version.includes('TLSv1.0') ||
                                    version.includes('TLSv1.1') ||
                                    version.includes('TLS1.0') ||
                                    version.includes('TLS1.1')
                                  return (
                                    <span
                                      key={index}
                                      className={`px-2 py-1 text-xs rounded ${
                                        isDeprecated
                                          ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200 border border-red-300 dark:border-red-700'
                                          : 'bg-primary-100 text-primary-800 dark:bg-primary-900 dark:text-primary-200'
                                      }`}>
                                      {version}
                                      {isDeprecated && (
                                        <span
                                          className='ml-1 text-red-600 dark:text-red-400'
                                          title='已废弃的不安全版本'>
                                          ⚠️
                                        </span>
                                      )}
                                    </span>
                                  )
                                })}
                              </div>
                            </div>
                          )}

                        {/* HTTP/2 Support */}
                        {sslInfo.http2_support !== undefined && (
                          <div className='flex items-center justify-between'>
                            <span className='text-sm text-slate-600 dark:text-slate-400'>
                              HTTP/2 支持:
                            </span>
                            <span
                              className={`px-2 py-1 text-xs rounded ${
                                sslInfo.http2_support
                                  ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                  : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                              }`}>
                              {sslInfo.http2_support ? '支持' : '不支持'}
                            </span>
                          </div>
                        )}

                        {/* HTTP/3 Support */}
                        {sslInfo.http3_support !== undefined && (
                          <div className='flex items-center justify-between'>
                            <span className='text-sm text-slate-600 dark:text-slate-400'>
                              HTTP/3 支持:
                            </span>
                            <span
                              className={`px-2 py-1 text-xs rounded ${
                                sslInfo.http3_support
                                  ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                  : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                              }`}>
                              {sslInfo.http3_support ? '支持' : '不支持'}
                            </span>
                          </div>
                        )}

                        {/* SPDY Support */}
                        {sslInfo.spdy_support !== undefined && (
                          <div className='flex items-center justify-between'>
                            <span className='text-sm text-slate-600 dark:text-slate-400'>
                              SPDY 支持:
                            </span>
                            <span
                              className={`px-2 py-1 text-xs rounded ${
                                sslInfo.spdy_support
                                  ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                  : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                              }`}>
                              {sslInfo.spdy_support ? '支持' : '不支持'}
                            </span>
                          </div>
                        )}

                        {/* ALPN Protocols */}
                        {sslInfo.alpn_protocols &&
                          sslInfo.alpn_protocols.length > 0 && (
                            <div className='flex items-center justify-between'>
                              <span className='text-sm text-slate-600 dark:text-slate-400'>
                                ALPN 协议:
                              </span>
                              <div className='flex flex-wrap gap-2 justify-end'>
                                {sslInfo.alpn_protocols.map(
                                  (protocol, index) => (
                                    <span
                                      key={index}
                                      className='px-2 py-1 text-xs bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200 rounded'>
                                      {protocol}
                                    </span>
                                  ),
                                )}
                              </div>
                            </div>
                          )}

                        {sslInfo.certificate && (
                          <div className='space-y-2'>
                            <div className='flex items-center justify-between'>
                              <span className='text-sm text-slate-600 dark:text-slate-400'>
                                证书有效期:
                              </span>
                              <span className='text-sm font-medium text-slate-900 dark:text-white'>
                                {formatDate(sslInfo.certificate.valid_to)}
                              </span>
                            </div>
                            <div className='flex items-center justify-between'>
                              <span className='text-sm text-slate-600 dark:text-slate-400'>
                                证书颁发者:
                              </span>
                              <span className='text-sm font-medium text-slate-900 dark:text-white truncate ml-2'>
                                {extractCN(sslInfo.certificate.issuer)}
                              </span>
                            </div>
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                )}

                {/* Certificate Tab */}
                {activeTab === 'certificate' && sslInfo.certificate && (
                  <div className='space-y-4'>
                    <div className='space-y-3'>
                      {[
                        {
                          label: '主题',
                          value: sslInfo.certificate.subject,
                          copyable: true,
                        },
                        {
                          label: '颁发者',
                          value: sslInfo.certificate.issuer,
                          copyable: true,
                        },
                        {
                          label: '有效期开始',
                          value: formatDate(sslInfo.certificate.valid_from),
                        },
                        {
                          label: '有效期结束',
                          value: formatDate(sslInfo.certificate.valid_to),
                        },
                        {
                          label: '序列号',
                          value: sslInfo.certificate.serial_number,
                          copyable: true,
                        },
                        {
                          label: '指纹',
                          value: sslInfo.certificate.fingerprint,
                          copyable: true,
                        },
                        {
                          label: '签名算法',
                          value: sslInfo.certificate.signature_algorithm,
                        },
                        {
                          label: '公钥算法',
                          value: sslInfo.certificate.public_key_algorithm,
                        },
                        {
                          label: '密钥长度',
                          value: sslInfo.certificate.key_size
                            ? `${sslInfo.certificate.key_size}`
                            : undefined,
                        },
                      ]
                        .filter((item) => item.value)
                        .map(({ label, value, copyable }) => (
                          <div
                            key={label}
                            className='flex items-start justify-between py-2 border-b border-slate-100 dark:border-slate-700 last:border-0'>
                            <span className='text-sm text-slate-600 dark:text-slate-400 min-w-0 w-32 flex-shrink-0'>
                              {label}:
                            </span>
                            <div className='flex items-center gap-2 flex-1 min-w-0 ml-4'>
                              <span className='font-medium text-slate-900 dark:text-white text-sm break-all'>
                                {value}
                              </span>
                              {copyable && (
                                <button
                                  onClick={() => copyToClipboard(value!)}
                                  className='p-1 hover:bg-slate-100 dark:hover:bg-slate-700 rounded flex-shrink-0'>
                                  <svg
                                    className='w-4 h-4'
                                    fill='none'
                                    stroke='currentColor'
                                    viewBox='0 0 24 24'>
                                    <path
                                      strokeLinecap='round'
                                      strokeLinejoin='round'
                                      strokeWidth={2}
                                      d='M8 7V3h8v4M8 7h8v13H8z'
                                    />
                                  </svg>
                                </button>
                              )}
                            </div>
                          </div>
                        ))}

                      {sslInfo.certificate.san_domains &&
                        sslInfo.certificate.san_domains.length > 0 && (
                          <div className='py-2'>
                            <span className='text-sm text-slate-600 dark:text-slate-400 block mb-2'>
                              SAN 域名:
                            </span>
                            <div className='flex flex-wrap gap-2'>
                              {sslInfo.certificate.san_domains.map(
                                (domain, index) => (
                                  <span
                                    key={index}
                                    className='px-2 py-1 text-xs bg-slate-100 text-slate-800 dark:bg-slate-700 dark:text-slate-200 rounded'>
                                    {domain}
                                  </span>
                                ),
                              )}
                            </div>
                          </div>
                        )}
                    </div>
                  </div>
                )}

                {/* Certificate Chain Tab */}
                {activeTab === 'chain' && (
                  <div className='space-y-6'>
                    {sslInfo.certificate_chain ? (
                      <div className='space-y-6'>
                        {/* Chain Status Overview */}
                        <div
                          className={`p-4 rounded-lg border ${
                            sslInfo.certificate_chain
                              .chain_validation_status === 'valid'
                              ? 'bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-800'
                              : sslInfo.certificate_chain
                                  .chain_validation_status ===
                                'valid-with-warnings'
                              ? 'bg-yellow-50 dark:bg-yellow-900/20 border-yellow-200 dark:border-yellow-800'
                              : 'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800'
                          }`}>
                          <div className='flex items-center justify-between mb-3'>
                            <h3 className='font-medium text-slate-900 dark:text-white'>
                              证书链状态
                            </h3>
                            <span
                              className={`px-3 py-1 text-sm font-medium rounded-full ${
                                sslInfo.certificate_chain
                                  .chain_validation_status === 'valid'
                                  ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                  : sslInfo.certificate_chain
                                      .chain_validation_status ===
                                    'valid-with-warnings'
                                  ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                                  : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                              }`}>
                              {sslInfo.certificate_chain
                                .chain_validation_status === 'valid'
                                ? '有效'
                                : sslInfo.certificate_chain
                                    .chain_validation_status ===
                                  'valid-with-warnings'
                                ? '有效(有警告)'
                                : '无效'}
                            </span>
                          </div>

                          <div className='grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 text-sm'>
                            <div>
                              <span className='text-slate-600 dark:text-slate-400'>
                                证书链长度:
                              </span>
                              <span className='ml-2 font-medium text-slate-900 dark:text-white'>
                                {sslInfo.certificate_chain.chain_length}
                              </span>
                            </div>
                            <div>
                              <span className='text-slate-600 dark:text-slate-400'>
                                链锚定:
                              </span>
                              <span
                                className={`ml-2 font-medium ${
                                  sslInfo.certificate_chain.is_complete
                                    ? 'text-green-600 dark:text-green-400'
                                    : 'text-red-600 dark:text-red-400'
                                }`}>
                                {sslInfo.certificate_chain.is_complete
                                  ? '完整'
                                  : '不完整'}
                              </span>
                            </div>
                            <div className='md:col-span-2 lg:col-span-1'>
                              <div className='flex flex-col'>
                                <span className='text-slate-600 dark:text-slate-400 mb-1'>
                                  根CA:
                                </span>
                                <span
                                  className='font-medium text-slate-900 dark:text-white text-xs leading-4 break-words'
                                  title={
                                    sslInfo.certificate_chain.root_ca_info
                                  }>
                                  {sslInfo.certificate_chain.root_ca_info ||
                                    '未知'}
                                </span>
                              </div>
                            </div>
                          </div>

                          {sslInfo.certificate_chain.chain_errors.length >
                            0 && (
                            <div className='mt-4 p-3 bg-red-50 dark:bg-red-900/30 border border-red-200 dark:border-red-700 rounded'>
                              <h4 className='font-medium text-red-800 dark:text-red-200 mb-2'>
                                链错误:
                              </h4>
                              <ul className='space-y-1'>
                                {sslInfo.certificate_chain.chain_errors.map(
                                  (error, index) => (
                                    <li
                                      key={index}
                                      className='text-sm text-red-700 dark:text-red-300'>
                                      • {error}
                                    </li>
                                  ),
                                )}
                              </ul>
                            </div>
                          )}
                        </div>

                        {/* Certificate Chain Diagram */}
                        <div className='space-y-4'>
                          <h3 className='font-medium text-slate-900 dark:text-white flex items-center'>
                            <svg
                              className='w-5 h-5 mr-2'
                              fill='none'
                              stroke='currentColor'
                              viewBox='0 0 24 24'>
                              <path
                                strokeLinecap='round'
                                strokeLinejoin='round'
                                strokeWidth={2}
                                d='M9 12l2 2 4-4M7.835 4.697a3.42 3.42 0 001.946-.806 3.42 3.42 0 014.438 0 3.42 3.42 0 001.946.806 3.42 3.42 0 013.138 3.138 3.42 3.42 0 00.806 1.946 3.42 3.42 0 010 4.438 3.42 3.42 0 00-.806 1.946 3.42 3.42 0 01-3.138 3.138 3.42 3.42 0 00-1.946.806 3.42 3.42 0 01-4.438 0 3.42 3.42 0 00-1.946-.806 3.42 3.42 0 01-3.138-3.138 3.42 3.42 0 00-.806-1.946 3.42 3.42 0 010-4.438 3.42 3.42 0 00.806-1.946 3.42 3.42 0 013.138-3.138z'
                              />
                            </svg>
                            证书链关系图
                          </h3>

                          <div className='relative bg-slate-50 dark:bg-slate-700/50 p-6 rounded-lg'>
                            {/* Chain Visualization */}
                            <div className='flex flex-col space-y-4'>
                              {sslInfo.certificate_chain.certificates.map(
                                (node, index) => {
                                  const isLast =
                                    index ===
                                    sslInfo.certificate_chain!.certificates
                                      .length -
                                      1

                                  return (
                                    <div key={index} className='relative'>
                                      {/* Connection Line to Next Certificate */}
                                      {!isLast && (
                                        <div className='absolute left-1/2 transform -translate-x-1/2 top-full w-0.5 h-4 bg-slate-400 dark:bg-slate-500 z-0'></div>
                                      )}

                                      {/* Arrow pointing down */}
                                      {!isLast && (
                                        <div className='absolute left-1/2 transform -translate-x-1/2 -translate-y-1/2 top-full z-10'>
                                          <svg
                                            className='w-4 h-4 text-slate-400 dark:text-slate-500'
                                            fill='currentColor'
                                            viewBox='0 0 20 20'>
                                            <path
                                              fillRule='evenodd'
                                              d='M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z'
                                              clipRule='evenodd'
                                            />
                                          </svg>
                                        </div>
                                      )}

                                      {/* Certificate Node */}
                                      <div
                                        className={`relative z-10 p-4 rounded-lg border-2 ${
                                          node.is_leaf
                                            ? 'bg-primary-50 dark:bg-primary-900/20 border-primary-300 dark:border-primary-700'
                                            : node.is_root
                                            ? 'bg-green-50 dark:bg-green-900/20 border-green-300 dark:border-green-700'
                                            : 'bg-yellow-50 dark:bg-yellow-900/20 border-yellow-300 dark:border-yellow-700'
                                        } shadow-sm`}>
                                        {/* Certificate Type Badge */}
                                        <div className='flex items-center justify-between mb-3'>
                                          <span
                                            className={`px-2 py-1 text-xs font-medium rounded-full ${
                                              node.is_leaf
                                                ? 'bg-primary-100 text-primary-800 dark:bg-primary-900 dark:text-primary-200'
                                                : node.is_root
                                                ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                                : 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                                            }`}>
                                            {node.chain_level === 0
                                              ? '叶子证书 (服务器)'
                                              : node.chain_level === 2
                                              ? node.trust_status === 'root-ca'
                                                ? '根证书（交叉签名形式）'
                                                : '根证书'
                                              : '中间CA证书'}
                                          </span>

                                          {/* Trust Status Badge */}
                                          <span
                                            className={`px-2 py-1 text-xs font-medium rounded-full ${
                                              node.trust_status === 'end-entity'
                                                ? 'bg-primary-100 text-primary-800 dark:bg-primary-900 dark:text-primary-200'
                                                : node.trust_status ===
                                                  'self-signed'
                                                ? 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200'
                                                : node.trust_status ===
                                                  'root-ca'
                                                ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                                : 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200'
                                            }`}>
                                            {node.trust_status === 'end-entity'
                                              ? '服务器证书'
                                              : node.trust_status ===
                                                'self-signed'
                                              ? '自签名'
                                              : node.trust_status === 'root-ca'
                                              ? '根CA'
                                              : node.trust_status ===
                                                'intermediate'
                                              ? '中间CA'
                                              : node.trust_status}
                                          </span>
                                        </div>

                                        {/* Certificate Details */}
                                        <div className='space-y-2 text-sm'>
                                          <div>
                                            <span className='text-slate-600 dark:text-slate-400'>
                                              主题:
                                            </span>
                                            <div className='font-medium text-slate-900 dark:text-white break-all mt-1'>
                                              {extractCN(
                                                node.certificate.subject,
                                              )}
                                            </div>
                                          </div>

                                          {node.trust_status !== 'self-signed' && (
                                            <div>
                                              <span className='text-slate-600 dark:text-slate-400'>
                                                颁发者:
                                              </span>
                                              <div className='font-medium text-slate-900 dark:text-white break-all mt-1'>
                                                {extractCN(
                                                  node.certificate.issuer,
                                                )}
                                              </div>
                                            </div>
                                          )}

                                          <div className='grid grid-cols-1 md:grid-cols-2 gap-2'>
                                            <div>
                                              <span className='text-slate-600 dark:text-slate-400'>
                                                有效期至:
                                              </span>
                                              <div className='font-medium text-slate-900 dark:text-white'>
                                                {formatDate(
                                                  node.certificate.valid_to,
                                                )}
                                              </div>
                                            </div>
                                            <div>
                                              <span className='text-slate-600 dark:text-slate-400'>
                                                签名算法:
                                              </span>
                                              <div className='font-medium text-slate-900 dark:text-white'>
                                                {
                                                  node.certificate
                                                    .signature_algorithm
                                                }
                                              </div>
                                            </div>
                                          </div>

                                          {node.certificate.key_size && (
                                            <div>
                                              <span className='text-slate-600 dark:text-slate-400'>
                                                密钥:
                                              </span>
                                              <span className='ml-2 font-medium text-slate-900 dark:text-white'>
                                                {
                                                  node.certificate
                                                    .public_key_algorithm
                                                }{' '}
                                                {node.certificate.key_size} 位
                                              </span>
                                            </div>
                                          )}

                                          {/* Validation Errors */}
                                          {node.validation_errors.length >
                                            0 && (
                                            <div className='mt-3 p-2 bg-red-50 dark:bg-red-900/30 border border-red-200 dark:border-red-700 rounded'>
                                              <h5 className='font-medium text-red-800 dark:text-red-200 text-xs mb-1'>
                                                验证错误:
                                              </h5>
                                              <ul className='space-y-1'>
                                                {node.validation_errors.map(
                                                  (error, errorIndex) => (
                                                    <li
                                                      key={errorIndex}
                                                      className='text-xs text-red-700 dark:text-red-300'>
                                                      • {error}
                                                    </li>
                                                  ),
                                                )}
                                              </ul>
                                            </div>
                                          )}

                                          {/* Copy Actions */}
                                          <div className='flex items-center gap-2 pt-2 border-t border-slate-200 dark:border-slate-600'>
                                            <button
                                              onClick={() =>
                                                copyToClipboard(
                                                  node.certificate.subject,
                                                )
                                              }
                                              className='text-xs px-2 py-1 bg-slate-100 dark:bg-slate-600 text-slate-700 dark:text-slate-300 rounded hover:bg-slate-200 dark:hover:bg-slate-500 flex items-center gap-1'>
                                              <svg
                                                className='w-3 h-3'
                                                fill='none'
                                                stroke='currentColor'
                                                viewBox='0 0 24 24'>
                                                <path
                                                  strokeLinecap='round'
                                                  strokeLinejoin='round'
                                                  strokeWidth={2}
                                                  d='M8 7V3h8v4M8 7h8v13H8z'
                                                />
                                              </svg>
                                              主题
                                            </button>
                                            <button
                                              onClick={() =>
                                                copyToClipboard(
                                                  node.certificate.fingerprint,
                                                )
                                              }
                                              className='text-xs px-2 py-1 bg-slate-100 dark:bg-slate-600 text-slate-700 dark:text-slate-300 rounded hover:bg-slate-200 dark:hover:bg-slate-500 flex items-center gap-1'>
                                              <svg
                                                className='w-3 h-3'
                                                fill='none'
                                                stroke='currentColor'
                                                viewBox='0 0 24 24'>
                                                <path
                                                  strokeLinecap='round'
                                                  strokeLinejoin='round'
                                                  strokeWidth={2}
                                                  d='M8 7V3h8v4M8 7h8v13H8z'
                                                />
                                              </svg>
                                              指纹
                                            </button>
                                          </div>
                                        </div>
                                      </div>
                                    </div>
                                  )
                                },
                              )}
                            </div>

                            {/* Legend */}
                            <div className='mt-6 pt-4 border-t border-slate-300 dark:border-slate-600'>
                              <div className='flex flex-wrap gap-4 text-xs'>
                                <div className='flex items-center gap-2'>
                                  <div className='w-3 h-3 rounded-full bg-primary-300 dark:bg-primary-700'></div>
                                  <span className='text-slate-600 dark:text-slate-400'>
                                    叶子证书 (服务器)
                                  </span>
                                </div>
                                <div className='flex items-center gap-2'>
                                  <div className='w-3 h-3 rounded-full bg-yellow-300 dark:bg-yellow-700'></div>
                                  <span className='text-slate-600 dark:text-slate-400'>
                                    中间证书
                                  </span>
                                </div>
                                <div className='flex items-center gap-2'>
                                  <div className='w-3 h-3 rounded-full bg-green-300 dark:bg-green-700'></div>
                                  <span className='text-slate-600 dark:text-slate-400'>
                                    根证书
                                  </span>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>

                        {/* Chain Analysis Summary */}
                        <div className='bg-slate-50 dark:bg-slate-700/50 p-4 rounded-lg'>
                          <h4 className='font-medium text-slate-900 dark:text-white mb-3'>
                            证书链分析总结
                          </h4>
                          <div className='space-y-2 text-sm'>
                            <div className='flex items-center gap-2'>
                              <svg
                                className={`w-4 h-4 ${
                                  sslInfo.certificate_chain.is_complete
                                    ? 'text-green-500'
                                    : 'text-red-500'
                                }`}
                                fill='none'
                                stroke='currentColor'
                                viewBox='0 0 24 24'>
                                <path
                                  strokeLinecap='round'
                                  strokeLinejoin='round'
                                  strokeWidth={2}
                                  d={
                                    sslInfo.certificate_chain.is_complete
                                      ? 'M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z'
                                      : 'M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z'
                                  }
                                />
                              </svg>
                              <span className='text-slate-700 dark:text-slate-300'>
                                证书链
                                {sslInfo.certificate_chain.is_complete
                                  ? '可验证至受信任的根 CA'
                                  : '无法验证到受信任的根 CA'}
                                ，共包含{' '}
                                {sslInfo.certificate_chain.chain_length} 个证书
                              </span>
                            </div>

                            {sslInfo.certificate_chain.trust_anchor_info && (
                              <div className='flex items-center gap-2'>
                                <svg
                                  className='w-4 h-4 text-green-500'
                                  fill='none'
                                  stroke='currentColor'
                                  viewBox='0 0 24 24'>
                                  <path
                                    strokeLinecap='round'
                                    strokeLinejoin='round'
                                    strokeWidth={2}
                                    d='M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z'
                                  />
                                </svg>
                                <span className='text-slate-700 dark:text-slate-300'>
                                  受信任根 CA：{
                                    sslInfo.certificate_chain.trust_anchor_info
                                  }
                                  （
                                  {sslInfo.certificate_chain.root_in_chain
                                    ? '服务器已发送'
                                    : '由客户端信任库提供，属标准配置'}
                                  ）
                                </span>
                              </div>
                            )}

                            <div className='flex items-center gap-2'>
                              <svg
                                className={`w-4 h-4 ${
                                  sslInfo.certificate_chain
                                    .chain_validation_status === 'valid'
                                    ? 'text-green-500'
                                    : sslInfo.certificate_chain
                                        .chain_validation_status ===
                                      'valid-with-warnings'
                                    ? 'text-yellow-500'
                                    : 'text-red-500'
                                }`}
                                fill='none'
                                stroke='currentColor'
                                viewBox='0 0 24 24'>
                                <path
                                  strokeLinecap='round'
                                  strokeLinejoin='round'
                                  strokeWidth={2}
                                  d='M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z'
                                />
                              </svg>
                              <span className='text-slate-700 dark:text-slate-300'>
                                证书链验证状态:{' '}
                                {sslInfo.certificate_chain
                                  .chain_validation_status === 'valid'
                                  ? '有效'
                                  : sslInfo.certificate_chain
                                      .chain_validation_status ===
                                    'valid-with-warnings'
                                  ? '有效但有警告'
                                  : '无效'}
                              </span>
                            </div>

                            <div className='flex items-center gap-2'>
                              <svg
                                className='w-4 h-4 text-primary-500'
                                fill='none'
                                stroke='currentColor'
                                viewBox='0 0 24 24'>
                                <path
                                  strokeLinecap='round'
                                  strokeLinejoin='round'
                                  strokeWidth={2}
                                  d='M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z'
                                />
                              </svg>
                              <span className='text-slate-700 dark:text-slate-300'>
                                建议定期监控证书链完整性和各证书的有效期
                              </span>
                            </div>
                          </div>
                        </div>
                      </div>
                    ) : (
                      <div className='text-center py-8 text-slate-500 dark:text-slate-400'>
                        <svg
                          className='w-12 h-12 mx-auto mb-4 text-slate-400'
                          fill='none'
                          stroke='currentColor'
                          viewBox='0 0 24 24'>
                          <path
                            strokeLinecap='round'
                            strokeLinejoin='round'
                            strokeWidth={2}
                            d='M9 12l2 2 4-4M7.835 4.697a3.42 3.42 0 001.946-.806 3.42 3.42 0 014.438 0 3.42 3.42 0 001.946.806 3.42 3.42 0 013.138 3.138 3.42 3.42 0 00.806 1.946 3.42 3.42 0 010 4.438 3.42 3.42 0 00-.806 1.946 3.42 3.42 0 01-3.138 3.138 3.42 3.42 0 00-1.946.806 3.42 3.42 0 01-4.438 0 3.42 3.42 0 00-1.946-.806 3.42 3.42 0 01-3.138-3.138 3.42 3.42 0 00-.806-1.946 3.42 3.42 0 010-4.438 3.42 3.42 0 00.806-1.946 3.42 3.42 0 013.138-3.138z'
                          />
                        </svg>
                        <div className='text-lg font-medium text-slate-900 dark:text-white mb-2'>
                          未获取到证书链信息
                        </div>
                        <div className='text-sm'>
                          请先运行 SSL 检测以获取完整的证书链信息
                        </div>
                      </div>
                    )}
                  </div>
                )}

                {/* Cipher Suite Tab */}
                {activeTab === 'cipher' && (
                  <div className='space-y-6'>
                    {sslInfo.protocol_support ? (
                      <div className='space-y-6'>
                        {sslInfo.server_cipher_order !== undefined && (
                          <div className='p-4 bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 rounded-lg'>
                            <div className='flex items-center justify-between'>
                              <span className='font-medium text-primary-800 dark:text-primary-200'>
                                服务器加密套件顺序偏好:
                              </span>
                              <span
                                className={`text-sm px-2 py-1 rounded ${
                                  sslInfo.server_cipher_order
                                    ? 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200'
                                    : 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                }`}>
                                {sslInfo.server_cipher_order
                                  ? '服务器优先'
                                  : '客户端优先'}
                              </span>
                            </div>
                            <div className='text-sm text-primary-700 dark:text-primary-300 mt-2'>
                              {sslInfo.server_cipher_order
                                ? '服务器按照自己的配置选择加密套件，忽略客户端的偏好。'
                                : '服务器会尊重客户端提供的加密套件顺序。'}
                            </div>
                          </div>
                        )}

                        {sslInfo.protocol_support
                          .filter(
                            (protocol) =>
                              protocol.supported &&
                              protocol.cipher_suites.length > 0,
                          )
                          .map((protocol, protocolIndex) => (
                            <div key={protocolIndex} className='space-y-3'>
                              <div className='flex items-center justify-between'>
                                <h4 className='font-medium text-slate-900 dark:text-white'>
                                  {protocol.version} 加密套件 (
                                  {protocol.cipher_suites.length})
                                </h4>
                                <div className='flex items-center gap-2'>
                                  <span
                                    className={`text-xs px-2 py-1 rounded-full ${
                                      protocol.version.includes('TLS 1.3')
                                        ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                        : protocol.version.includes('TLS 1.2')
                                        ? 'bg-primary-100 text-primary-800 dark:bg-primary-900 dark:text-primary-200'
                                        : protocol.version.includes(
                                            'TLS 1.1',
                                          ) ||
                                          protocol.version.includes('TLS 1.0')
                                        ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                                        : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                                    }`}>
                                    {protocol.version.includes('TLS 1.3')
                                      ? '推荐'
                                      : protocol.version.includes('TLS 1.2')
                                      ? '安全'
                                      : protocol.version.includes('TLS 1.1') ||
                                        protocol.version.includes('TLS 1.0')
                                      ? '过时'
                                      : '不安全'}
                                  </span>

                                  {/* HTTP/2 Support Badge */}
                                  {protocol.http2_support !== undefined && (
                                    <span
                                      className={`text-xs px-2 py-1 rounded ${
                                        protocol.http2_support
                                          ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                          : 'bg-slate-100 text-slate-800 dark:bg-slate-700 dark:text-slate-200'
                                      }`}>
                                      HTTP/2:{' '}
                                      {protocol.http2_support ? '✓' : '✗'}
                                    </span>
                                  )}

                                  {/* HTTP/3 Support Badge */}
                                  {protocol.http3_support !== undefined && (
                                    <span
                                      className={`text-xs px-2 py-1 rounded ${
                                        protocol.http3_support
                                          ? 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200'
                                          : 'bg-slate-100 text-slate-800 dark:bg-slate-700 dark:text-slate-200'
                                      }`}>
                                      HTTP/3:{' '}
                                      {protocol.http3_support ? '✓' : '✗'}
                                    </span>
                                  )}

                                  {/* SPDY Support Badge */}
                                  {protocol.spdy_support !== undefined && (
                                    <span
                                      className={`text-xs px-2 py-1 rounded ${
                                        protocol.spdy_support
                                          ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                          : 'bg-slate-100 text-slate-800 dark:bg-slate-700 dark:text-slate-200'
                                      }`}>
                                      SPDY: {protocol.spdy_support ? '✓' : '✗'}
                                    </span>
                                  )}
                                </div>
                              </div>

                              {/* ALPN Protocols */}
                              {protocol.alpn_protocols &&
                                protocol.alpn_protocols.length > 0 && (
                                  <div className='flex items-center gap-2'>
                                    <span className='text-sm text-slate-600 dark:text-slate-400'>
                                      ALPN:
                                    </span>
                                    <div className='flex flex-wrap gap-1'>
                                      {protocol.alpn_protocols.map(
                                        (alpn, alpnIndex) => (
                                          <span
                                            key={alpnIndex}
                                            className='px-2 py-1 text-xs bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200 rounded'>
                                            {alpn}
                                          </span>
                                        ),
                                      )}
                                    </div>
                                  </div>
                                )}

                              <div className='space-y-2'>
                                {protocol.cipher_suites.map(
                                  (cipher, cipherIndex) => (
                                    <div
                                      key={cipherIndex}
                                      className='flex items-center justify-between py-2 px-3 bg-slate-50 dark:bg-slate-700 rounded'>
                                      <div className='flex-1 min-w-0'>
                                        <div className='font-mono text-sm text-slate-900 dark:text-white truncate'>
                                          {cipher.name}
                                        </div>
                                        <div className='flex items-center gap-4 mt-1'>
                                          <span
                                            className={`text-xs px-2 py-1 rounded ${
                                              cipher.strength === 'HIGH'
                                                ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                                : cipher.strength === 'MEDIUM'
                                                ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                                                : cipher.strength === 'WEAK'
                                                ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                                                : 'bg-slate-100 text-slate-800 dark:bg-slate-900 dark:text-slate-200'
                                            }`}>
                                            {cipher.strength === 'HIGH'
                                              ? '高强度'
                                              : cipher.strength === 'MEDIUM'
                                              ? '中等强度'
                                              : cipher.strength === 'WEAK'
                                              ? '弱加密'
                                              : cipher.strength}
                                          </span>
                                          {cipher.server_order && (
                                            <span className='text-xs px-2 py-1 bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200 rounded'>
                                              服务器优先
                                            </span>
                                          )}
                                        </div>
                                      </div>
                                      <button
                                        onClick={() =>
                                          copyToClipboard(cipher.name)
                                        }
                                        className='p-1 hover:bg-slate-200 dark:hover:bg-slate-600 rounded ml-2'>
                                        <svg
                                          className='w-4 h-4'
                                          fill='none'
                                          stroke='currentColor'
                                          viewBox='0 0 24 24'>
                                          <path
                                            strokeLinecap='round'
                                            strokeLinejoin='round'
                                            strokeWidth={2}
                                            d='M8 7V3h8v4M8 7h8v13H8z'
                                          />
                                        </svg>
                                      </button>
                                    </div>
                                  ),
                                )}
                              </div>
                            </div>
                          ))}
                      </div>
                    ) : (
                      <div className='text-center py-8 text-slate-500 dark:text-slate-400'>
                        未获取到加密套件信息
                      </div>
                    )}
                  </div>
                )}

                {/* Security Findings (CVE) Tab */}
                {activeTab === 'cve' && (
                  <div className='space-y-6'>
                    {sslInfo.cve_vulnerabilities &&
                    sslInfo.cve_vulnerabilities.length > 0 ? (
                      <div className='space-y-4'>
                        <div className='text-sm text-slate-600 dark:text-slate-400'>
                          基于真实握手探测确认的安全发现共{' '}
                          {sslInfo.cve_vulnerabilities.length}{' '}
                          项，每项均附观测证据
                        </div>

                        {/* Severity Filter */}
                        <div className='flex flex-wrap gap-2 p-4 bg-slate-50 dark:bg-slate-700/50 rounded-lg'>
                          <span className='text-sm font-medium text-slate-700 dark:text-slate-300 mr-2'>
                            严重等级过滤:
                          </span>
                          {[
                            {
                              value: 'ALL',
                              label: '全部',
                              count: sslInfo.cve_vulnerabilities.length,
                            },
                            {
                              value: 'CRITICAL',
                              label: '严重',
                              count: sslInfo.cve_vulnerabilities.filter(
                                (v) => v.severity === 'CRITICAL',
                              ).length,
                            },
                            {
                              value: 'HIGH',
                              label: '高危',
                              count: sslInfo.cve_vulnerabilities.filter(
                                (v) => v.severity === 'HIGH',
                              ).length,
                            },
                            {
                              value: 'MEDIUM',
                              label: '中危',
                              count: sslInfo.cve_vulnerabilities.filter(
                                (v) => v.severity === 'MEDIUM',
                              ).length,
                            },
                            {
                              value: 'LOW',
                              label: '低危',
                              count: sslInfo.cve_vulnerabilities.filter(
                                (v) => v.severity === 'LOW',
                              ).length,
                            },
                          ].map(({ value, label, count }) => (
                            <button
                              key={value}
                              onClick={() => setSeverityFilter(value as any)}
                              className={`px-3 py-1 text-sm rounded-full transition-colors ${
                                severityFilter === value
                                  ? value === 'CRITICAL'
                                    ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                                    : value === 'HIGH'
                                    ? 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200'
                                    : value === 'MEDIUM'
                                    ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                                    : value === 'LOW'
                                    ? 'bg-primary-100 text-primary-800 dark:bg-primary-900 dark:text-primary-200'
                                    : 'bg-slate-100 text-slate-800 dark:bg-slate-700 dark:text-slate-200'
                                  : 'bg-slate-50 text-slate-600 hover:bg-slate-100 dark:bg-slate-600 dark:text-slate-300 dark:hover:bg-slate-500'
                              }`}>
                              {label} ({count})
                            </button>
                          ))}
                        </div>

                        <div className='grid grid-cols-1 gap-4'>
                          {sslInfo.cve_vulnerabilities
                            .filter(
                              (vuln) =>
                                severityFilter === 'ALL' ||
                                vuln.severity === severityFilter,
                            )
                            .map((vuln, index) => (
                              <div
                                key={index}
                                className='border border-red-200 dark:border-red-800 rounded-lg overflow-hidden bg-red-50/30 dark:bg-red-900/10'>
                                {/* Header */}
                                <div
                                  className={`px-4 py-3 flex items-center justify-between border-b ${
                                    vuln.severity === 'CRITICAL'
                                      ? 'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800'
                                      : vuln.severity === 'HIGH'
                                      ? 'bg-orange-50 dark:bg-orange-900/20 border-orange-200 dark:border-orange-800'
                                      : vuln.severity === 'MEDIUM'
                                      ? 'bg-yellow-50 dark:bg-yellow-900/20 border-yellow-200 dark:border-yellow-800'
                                      : 'bg-primary-50 dark:bg-primary-900/20 border-primary-200 dark:border-primary-800'
                                  }`}>
                                  <div className='flex items-center gap-3 flex-wrap'>
                                    {/* Severity Badge */}
                                    <span
                                      className={`px-2 py-1 text-xs font-medium rounded-full ${
                                        vuln.severity === 'CRITICAL'
                                          ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                                          : vuln.severity === 'HIGH'
                                          ? 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200'
                                          : vuln.severity === 'MEDIUM'
                                          ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                                          : 'bg-primary-100 text-primary-800 dark:bg-primary-900 dark:text-primary-200'
                                      }`}>
                                      {vuln.severity}
                                    </span>

                                    {/* Category Badge */}
                                    <span className='px-2 py-1 text-xs font-medium rounded-full bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200'>
                                      {vuln.category === 'protocol'
                                        ? '协议'
                                        : vuln.category === 'cipher'
                                        ? '加密套件'
                                        : vuln.category === 'certificate'
                                        ? '证书'
                                        : vuln.category === 'key-exchange'
                                        ? '密钥交换'
                                        : vuln.category}
                                    </span>

                                    {/* Grade Cap Badge */}
                                    {vuln.grade_cap && (
                                      <span className='px-2 py-1 text-xs font-medium rounded-full bg-slate-800 text-white dark:bg-slate-200 dark:text-slate-900'>
                                        等级封顶 {vuln.grade_cap}
                                      </span>
                                    )}

                                    <div>
                                      <h4 className='font-medium text-slate-900 dark:text-white'>
                                        {vuln.name}
                                      </h4>
                                      <div className='text-xs text-slate-600 dark:text-slate-400 mt-1'>
                                        {vuln.cve_ids.length > 0 && (
                                          <span className='font-mono mr-2'>
                                            {vuln.cve_ids.join(', ')}
                                          </span>
                                        )}
                                        影响组件:{' '}
                                        {vuln.affected_components.join(', ')}
                                      </div>
                                    </div>
                                  </div>
                                </div>

                                {/* Description */}
                                <div className='px-4 py-3 bg-white dark:bg-slate-800'>
                                  <p className='text-sm text-slate-700 dark:text-slate-300'>
                                    {vuln.description}
                                  </p>
                                </div>

                                {/* Evidence */}
                                <div className='px-4 py-2 bg-amber-50 dark:bg-amber-900/20 border-t border-amber-100 dark:border-amber-800'>
                                  <div className='text-sm'>
                                    <span className='font-medium text-amber-800 dark:text-amber-200'>
                                      🔍 观测证据:{' '}
                                    </span>
                                    <span className='text-amber-800 dark:text-amber-200'>
                                      {vuln.evidence}
                                    </span>
                                  </div>
                                </div>

                                {/* Remediation */}
                                <div className='px-4 py-3 bg-slate-50 dark:bg-slate-700/50'>
                                  <h5 className='text-sm font-medium text-slate-900 dark:text-white mb-2'>
                                    修复建议:
                                  </h5>
                                  <p className='text-sm text-slate-700 dark:text-slate-300'>
                                    {vuln.remediation}
                                  </p>
                                </div>

                                {/* References */}
                                {vuln.references.length > 0 && (
                                  <div className='px-4 py-3 bg-white dark:bg-slate-800 border-t border-slate-200 dark:border-slate-700'>
                                    <h5 className='text-sm font-medium text-slate-900 dark:text-white mb-2'>
                                      参考资料:
                                    </h5>
                                    <div className='space-y-1'>
                                      {vuln.references.map((ref, refIndex) => (
                                        <a
                                          key={refIndex}
                                          href={ref}
                                          target='_blank'
                                          rel='noopener noreferrer'
                                          className='text-sm text-primary-600 hover:text-primary-800 dark:text-primary-400 dark:hover:text-primary-300 block truncate'>
                                          {ref}
                                        </a>
                                      ))}
                                    </div>
                                  </div>
                                )}
                              </div>
                            ))}
                        </div>

                        {/* Summary */}
                        <div className='mt-6 p-4 bg-slate-50 dark:bg-slate-700/50 rounded-lg space-y-3'>
                          <div>
                            <h5 className='text-sm font-medium text-slate-900 dark:text-white mb-2'>
                              检测总结:
                            </h5>
                            <div className='grid grid-cols-2 md:grid-cols-4 gap-4 text-sm'>
                              <div>
                                <span className='text-red-600 dark:text-red-400 font-medium'>
                                  严重:{' '}
                                  {
                                    sslInfo.cve_vulnerabilities.filter(
                                      (v) => v.severity === 'CRITICAL',
                                    ).length
                                  }
                                </span>
                              </div>
                              <div>
                                <span className='text-orange-600 dark:text-orange-400 font-medium'>
                                  高危:{' '}
                                  {
                                    sslInfo.cve_vulnerabilities.filter(
                                      (v) => v.severity === 'HIGH',
                                    ).length
                                  }
                                </span>
                              </div>
                              <div>
                                <span className='text-yellow-600 dark:text-yellow-400 font-medium'>
                                  中危:{' '}
                                  {
                                    sslInfo.cve_vulnerabilities.filter(
                                      (v) => v.severity === 'MEDIUM',
                                    ).length
                                  }
                                </span>
                              </div>
                              <div>
                                <span className='text-primary-600 dark:text-primary-400 font-medium'>
                                  当前显示:{' '}
                                  {
                                    sslInfo.cve_vulnerabilities.filter(
                                      (v) =>
                                        severityFilter === 'ALL' ||
                                        v.severity === severityFilter,
                                    ).length
                                  }
                                </span>
                              </div>
                            </div>
                          </div>
                          <div className='text-xs text-slate-500 dark:text-slate-400 border-t border-slate-200 dark:border-slate-700 pt-3'>
                            ℹ️
                            本列表仅包含可通过远程握手证据确认的安全发现。依赖服务器软件版本的漏洞（如
                            Heartbleed/CVE-2014-0160
                            ）无法远程可靠判定，不在列表中，请以服务器软件补丁状态为准。
                          </div>
                        </div>
                      </div>
                    ) : sslInfo.cve_vulnerabilities ? (
                      <div className='text-center py-8'>
                        <svg
                          className='w-12 h-12 mx-auto mb-4 text-green-500'
                          fill='none'
                          stroke='currentColor'
                          viewBox='0 0 24 24'>
                          <path
                            strokeLinecap='round'
                            strokeLinejoin='round'
                            strokeWidth={2}
                            d='M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z'
                          />
                        </svg>
                        <div className='text-lg font-medium text-slate-900 dark:text-white mb-2'>
                          未发现可确认的安全问题
                        </div>
                        <div className='text-sm text-slate-500 dark:text-slate-400'>
                          基于真实握手探测，未发现协议、加密套件或证书层面的安全问题
                        </div>
                      </div>
                    ) : (
                      <div className='text-center py-8 text-slate-500 dark:text-slate-400'>
                        <svg
                          className='w-12 h-12 mx-auto mb-4 text-slate-400'
                          fill='none'
                          stroke='currentColor'
                          viewBox='0 0 24 24'>
                          <path
                            strokeLinecap='round'
                            strokeLinejoin='round'
                            strokeWidth={2}
                            d='M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z'
                          />
                        </svg>
                        <div className='text-lg font-medium text-slate-900 dark:text-white mb-2'>
                          未进行漏洞检测
                        </div>
                        <div className='text-sm'>
                          请先运行 SSL 检测以获取完整的安全检测报告
                        </div>
                      </div>
                    )}
                  </div>
                )}

                {/* Security Tab */}
                {activeTab === 'security' && (
                  <div className='space-y-6'>
                    {sslInfo.security_score !== undefined && (
                      <div className='text-center'>
                        <div
                          className={`text-4xl font-bold mb-2 ${getSecurityColor(
                            sslInfo.security_score,
                          )}`}>
                          {sslInfo.security_score}/100
                        </div>
                        <div
                          className={`text-lg ${getSecurityColor(
                            sslInfo.security_score,
                          )}`}>
                          {sslInfo.ssl_labs_rating
                            ? `等级 ${sslInfo.ssl_labs_rating.grade} · ${getSecurityLabel(sslInfo.security_score)}`
                            : getSecurityLabel(sslInfo.security_score)}
                        </div>
                        {sslInfo.ssl_labs_rating && (
                          <div className='grid grid-cols-2 md:grid-cols-4 gap-3 mt-4 max-w-xl mx-auto text-sm'>
                            {[
                              {
                                label: '证书',
                                value: sslInfo.ssl_labs_rating
                                  .certificate_score,
                              },
                              {
                                label: '协议',
                                value: sslInfo.ssl_labs_rating.protocol_score,
                              },
                              {
                                label: '密钥交换',
                                value: sslInfo.ssl_labs_rating
                                  .key_exchange_score,
                              },
                              {
                                label: '套件强度',
                                value: sslInfo.ssl_labs_rating
                                  .cipher_strength_score,
                              },
                            ].map(({ label, value }) => (
                              <div
                                key={label}
                                className='p-2 bg-slate-50 dark:bg-slate-700/50 rounded'>
                                <div className='text-xs text-slate-500 dark:text-slate-400'>
                                  {label}
                                </div>
                                <div
                                  className={`font-medium ${getSecurityColor(value)}`}>
                                  {value}
                                </div>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    )}

                    {/* Grade Cap Explanations */}
                    {sslInfo.ssl_labs_rating &&
                      sslInfo.ssl_labs_rating.applied_caps.length > 0 && (
                        <div>
                          <h4 className='font-medium text-orange-600 dark:text-orange-400 mb-3 flex items-center'>
                            <svg
                              className='w-5 h-5 mr-2'
                              fill='none'
                              stroke='currentColor'
                              viewBox='0 0 24 24'>
                              <path
                                strokeLinecap='round'
                                strokeLinejoin='round'
                                strokeWidth={2}
                                d='M3 21v-4m0 0V5a2 2 0 012-2h6a2 2 0 012 2v12m0 0v4m0-4h6a2 2 0 002-2v-6a2 2 0 00-2-2h-6m-8 4h6'
                              />
                            </svg>
                            等级封顶原因 ({' '}
                            {sslInfo.ssl_labs_rating.applied_caps.length} )
                          </h4>
                          <div className='space-y-2'>
                            {sslInfo.ssl_labs_rating.applied_caps.map(
                              (cap, index) => (
                                <div
                                  key={index}
                                  className='p-3 bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded'>
                                  <div className='text-sm text-orange-800 dark:text-orange-200'>
                                    {cap.finding} → 等级上限{' '}
                                    <span className='font-bold'>
                                      {cap.cap}
                                    </span>
                                  </div>
                                  <div className='text-xs text-orange-600 dark:text-orange-300 mt-1'>
                                    证据：{cap.reason}
                                  </div>
                                </div>
                              ),
                            )}
                          </div>
                          <div className='mt-2 text-xs text-slate-500 dark:text-slate-400'>
                            总分取各类别加权分与封顶上限中的较小值，每个问题只封顶一次。
                          </div>
                        </div>
                      )}

                    {sslInfo.vulnerabilities &&
                      sslInfo.vulnerabilities.length > 0 && (
                        <div>
                          <h4 className='font-medium text-red-600 dark:text-red-400 mb-3 flex items-center'>
                            <svg
                              className='w-5 h-5 mr-2'
                              fill='none'
                              stroke='currentColor'
                              viewBox='0 0 24 24'>
                              <path
                                strokeLinecap='round'
                                strokeLinejoin='round'
                                strokeWidth={2}
                                d='M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.5 0L4.314 18.5c-.77.833.192 2.5 1.732 2.5z'
                              />
                            </svg>
                            发现的安全问题 ({sslInfo.vulnerabilities.length})
                          </h4>
                          <div className='space-y-2'>
                            {sslInfo.vulnerabilities.map((vuln, index) => (
                              <div
                                key={index}
                                className='p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded'>
                                <div className='text-sm text-red-800 dark:text-red-200'>
                                  {vuln}
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}

                    {sslInfo.recommendations &&
                      sslInfo.recommendations.length > 0 && (
                        <div>
                          <h4 className='font-medium text-primary-600 dark:text-primary-400 mb-3 flex items-center'>
                            <svg
                              className='w-5 h-5 mr-2'
                              fill='none'
                              stroke='currentColor'
                              viewBox='0 0 24 24'>
                              <path
                                strokeLinecap='round'
                                strokeLinejoin='round'
                                strokeWidth={2}
                                d='M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z'
                              />
                            </svg>
                            安全配置建议 ({sslInfo.recommendations.length})
                          </h4>
                          <div className='space-y-2'>
                            {sslInfo.recommendations.map((rec, index) => (
                              <div
                                key={index}
                                className='p-3 bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 rounded'>
                                <div className='text-sm text-primary-800 dark:text-primary-200'>
                                  {rec}
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}

                    {(!sslInfo.vulnerabilities ||
                      sslInfo.vulnerabilities.length === 0) &&
                      (!sslInfo.recommendations ||
                        sslInfo.recommendations.length === 0) && (
                        <div className='text-center py-8 text-slate-500 dark:text-slate-400'>
                          暂无安全评估信息
                        </div>
                      )}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      )}

      {showToast && (
        <div className='fixed bottom-6 right-6 bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-900 px-3 py-2 rounded shadow-md text-sm z-50'>
          {toast}
        </div>
      )}
    </ToolLayout>
  )
}

export default SslChecker
