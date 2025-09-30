import { invoke } from '@tauri-apps/api/core'
import React, { useEffect, useState } from 'react'
import { ToolLayout } from '../components/layouts'

interface CipherSuite {
  name: string
  version: string
  strength: string
  server_order: boolean
}

interface SecurityVulnerability {
  cve_id: string
  name: string
  description: string
  severity: string
  affected_components: string[]
  remediation: string
  references: string[]
  affected: boolean
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
  trust_status: string
  validation_errors: string[]
}

interface CertificateChain {
  certificates: CertificateChainNode[]
  chain_length: number
  is_complete: boolean
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
    if (!score) return 'text-gray-500'
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
    if (!grade) return 'text-gray-500'
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
        return 'text-gray-500'
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
            className='flex items-center gap-2 px-4 py-2 bg-gray-600 text-white rounded-md hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-gray-500'>
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
              <label className='text-lg font-medium text-gray-700 dark:text-gray-300 whitespace-nowrap'>
                域名:
              </label>
              <div className='flex items-center flex-1 min-w-0 w-full max-w-md border border-gray-300 rounded-md shadow-sm focus-within:ring-2 focus-within:ring-blue-500 dark:border-gray-600'>
                <span className='px-3 py-2 bg-gray-50 dark:bg-gray-600 text-gray-500 dark:text-gray-400 text-sm border-r border-gray-300 dark:border-gray-600 rounded-l-md'>
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
                  className='flex-1 px-3 py-2 border-0 focus:outline-none focus:ring-0 bg-white dark:bg-gray-700 dark:text-white rounded-r-md'
                  style={{ minWidth: '200px' }}
                />
              </div>
              <button
                type='button'
                onClick={() => handleCheck()}
                disabled={loading}
                className='px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 whitespace-nowrap'>
                {loading ? '检测中...' : '开始检测'}
              </button>
            </div>
            <p className='text-sm text-gray-600 dark:text-gray-400'>
              检测域名的 SSL 证书信息、支持的协议版本、加密套件和安全配置，包括
              HTTP/2 和 HTTP/3 支持性检测。
              <span className='block mt-1'>
                <span className='font-medium'>格式示例:</span>
                <code className='ml-1 px-1 bg-gray-100 dark:bg-gray-700 rounded text-xs'>
                  example.com
                </code>
                、
                <code className='ml-1 px-1 bg-gray-100 dark:bg-gray-700 rounded text-xs'>
                  www.example.com
                </code>
                、
                <code className='ml-1 px-1 bg-gray-100 dark:bg-gray-700 rounded text-xs'>
                  api.example.com
                </code>
              </span>
            </p>

            {/* 最近检测的域名 */}
            {recentDomains.length > 0 && (
              <div className='mt-4'>
                <div className='flex items-center justify-between mb-2'>
                  <span className='text-sm font-medium text-gray-700 dark:text-gray-300'>
                    最近检测的域名:
                  </span>
                  <button
                    onClick={() => {
                      setRecentDomains([])
                      localStorage.removeItem('ssl-checker-recent-domains')
                    }}
                    className='text-xs text-gray-500 hover:text-red-600 dark:text-gray-400 dark:hover:text-red-400 flex items-center gap-1'>
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
                      className='group relative flex items-center bg-gray-100 dark:bg-gray-700 rounded-lg overflow-hidden hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-colors'>
                      <button
                        onClick={() => handleRecentDomainClick(recentDomain)}
                        className='px-3 py-2 text-sm text-gray-700 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400 transition-colors flex items-center gap-2'
                        title={`检测 ${recentDomain}`}>
                        <svg
                          className='w-4 h-4 text-gray-400 group-hover:text-blue-500'
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
                        className='p-2 text-gray-400 hover:text-red-500 dark:hover:text-red-400 opacity-0 group-hover:opacity-100 transition-all'
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
                <p className='text-xs text-gray-500 dark:text-gray-400 mt-2'>
                  💡 点击域名可直接检测，最多保存10个最近检测的域名
                </p>
              </div>
            )}
          </div>

          {error && (
            <div className='mb-4 p-3 bg-red-100 text-red-700 rounded-md dark:bg-red-900 dark:text-red-100'>
              {error}
            </div>
          )}

          {loading && (
            <div className='mb-4 p-6 bg-gray-100 dark:bg-gray-800 rounded-lg animate-pulse'>
              <div className='h-4 bg-gray-300 dark:bg-gray-600 rounded mb-2'></div>
              <div className='h-4 bg-gray-300 dark:bg-gray-600 rounded w-3/4 mb-2'></div>
              <div className='h-4 bg-gray-300 dark:bg-gray-600 rounded w-1/2'></div>
            </div>
          )}
        </div>
      ) : (
        // 结果展示界面 - 添加滚动支持
        <div className='flex-1 overflow-y-auto'>
          {sslInfo && (
            <div className='bg-white dark:bg-gray-800 rounded-lg shadow-md'>
              {/* Tab Navigation */}
              <div className='border-b border-gray-200 dark:border-gray-700'>
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
                          ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                          : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300'
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
                          <span className='text-sm text-gray-600 dark:text-gray-400'>
                            域名:
                          </span>
                          <div className='flex items-center gap-2'>
                            <span className='font-medium text-gray-900 dark:text-white'>
                              {sslInfo.domain}
                            </span>
                            <button
                              onClick={() => copyToClipboard(sslInfo.domain)}
                              className='p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded'>
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
                            <span className='text-sm text-gray-600 dark:text-gray-400'>
                              服务器 IP:
                            </span>
                            <div className='flex items-center gap-2'>
                              <span className='font-medium text-gray-900 dark:text-white'>
                                {sslInfo.server_ip}
                              </span>
                              <button
                                onClick={() =>
                                  copyToClipboard(sslInfo.server_ip!)
                                }
                                className='p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded'>
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
                            <span className='text-sm text-gray-600 dark:text-gray-400'>
                              服务器版本:
                            </span>
                            <span className='font-medium text-gray-900 dark:text-white'>
                              {sslInfo.server_info}
                            </span>
                          </div>
                        )}

                        {sslInfo.security_score !== undefined && (
                          <div className='flex items-center justify-between'>
                            <span className='text-sm text-gray-600 dark:text-gray-400'>
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
                            <span className='text-sm text-gray-600 dark:text-gray-400'>
                              SSL Labs 评级:
                            </span>
                            <div className='flex items-center gap-2'>
                              <span
                                className={`font-bold text-2xl ${getSslLabsGradeColor(
                                  sslInfo.ssl_labs_rating.grade,
                                )}`}>
                                {sslInfo.ssl_labs_rating.grade}
                              </span>
                              <div className='text-sm text-gray-600 dark:text-gray-400'>
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

                        {/* CVE Vulnerabilities Indicator */}
                        {sslInfo.cve_vulnerabilities &&
                          sslInfo.cve_vulnerabilities.length > 0 && (
                            <div className='flex items-center justify-between'>
                              <span className='text-sm text-gray-600 dark:text-gray-400'>
                                CVE 漏洞:
                              </span>
                              <div className='flex items-center gap-2'>
                                <span
                                  className={`px-2 py-1 text-xs rounded-full ${
                                    sslInfo.cve_vulnerabilities.some(
                                      (v) =>
                                        v.affected && v.severity === 'CRITICAL',
                                    )
                                      ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                                      : sslInfo.cve_vulnerabilities.some(
                                          (v) =>
                                            v.affected && v.severity === 'HIGH',
                                        )
                                      ? 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200'
                                      : sslInfo.cve_vulnerabilities.some(
                                          (v) => v.affected,
                                        )
                                      ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                                      : 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                  }`}>
                                  {
                                    sslInfo.cve_vulnerabilities.filter(
                                      (v) => v.affected,
                                    ).length
                                  }{' '}
                                  个受影响
                                </span>
                                <span className='text-xs text-gray-500 dark:text-gray-400'>
                                  / {sslInfo.cve_vulnerabilities.length}{' '}
                                  个已检测
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
                              <span className='text-sm text-gray-600 dark:text-gray-400'>
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
                                          : 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'
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
                            <span className='text-sm text-gray-600 dark:text-gray-400'>
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
                            <span className='text-sm text-gray-600 dark:text-gray-400'>
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
                            <span className='text-sm text-gray-600 dark:text-gray-400'>
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
                              <span className='text-sm text-gray-600 dark:text-gray-400'>
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
                              <span className='text-sm text-gray-600 dark:text-gray-400'>
                                证书有效期:
                              </span>
                              <span className='text-sm font-medium text-gray-900 dark:text-white'>
                                {formatDate(sslInfo.certificate.valid_to)}
                              </span>
                            </div>
                            <div className='flex items-center justify-between'>
                              <span className='text-sm text-gray-600 dark:text-gray-400'>
                                证书颁发者:
                              </span>
                              <span className='text-sm font-medium text-gray-900 dark:text-white truncate ml-2'>
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
                            className='flex items-start justify-between py-2 border-b border-gray-100 dark:border-gray-700 last:border-0'>
                            <span className='text-sm text-gray-600 dark:text-gray-400 min-w-0 w-32 flex-shrink-0'>
                              {label}:
                            </span>
                            <div className='flex items-center gap-2 flex-1 min-w-0 ml-4'>
                              <span className='font-medium text-gray-900 dark:text-white text-sm break-all'>
                                {value}
                              </span>
                              {copyable && (
                                <button
                                  onClick={() => copyToClipboard(value!)}
                                  className='p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded flex-shrink-0'>
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
                            <span className='text-sm text-gray-600 dark:text-gray-400 block mb-2'>
                              SAN 域名:
                            </span>
                            <div className='flex flex-wrap gap-2'>
                              {sslInfo.certificate.san_domains.map(
                                (domain, index) => (
                                  <span
                                    key={index}
                                    className='px-2 py-1 text-xs bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200 rounded'>
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
                            <h3 className='font-medium text-gray-900 dark:text-white'>
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
                              <span className='text-gray-600 dark:text-gray-400'>
                                证书链长度:
                              </span>
                              <span className='ml-2 font-medium text-gray-900 dark:text-white'>
                                {sslInfo.certificate_chain.chain_length}
                              </span>
                            </div>
                            <div>
                              <span className='text-gray-600 dark:text-gray-400'>
                                链完整性:
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
                                <span className='text-gray-600 dark:text-gray-400 mb-1'>
                                  根CA:
                                </span>
                                <span
                                  className='font-medium text-gray-900 dark:text-white text-xs leading-4 break-words'
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
                          <h3 className='font-medium text-gray-900 dark:text-white flex items-center'>
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

                          <div className='relative bg-gray-50 dark:bg-gray-700/50 p-6 rounded-lg'>
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
                                        <div className='absolute left-1/2 transform -translate-x-1/2 top-full w-0.5 h-4 bg-gray-400 dark:bg-gray-500 z-0'></div>
                                      )}

                                      {/* Arrow pointing down */}
                                      {!isLast && (
                                        <div className='absolute left-1/2 transform -translate-x-1/2 -translate-y-1/2 top-full z-10'>
                                          <svg
                                            className='w-4 h-4 text-gray-400 dark:text-gray-500'
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
                                            ? 'bg-blue-50 dark:bg-blue-900/20 border-blue-300 dark:border-blue-700'
                                            : node.is_root
                                            ? 'bg-green-50 dark:bg-green-900/20 border-green-300 dark:border-green-700'
                                            : 'bg-yellow-50 dark:bg-yellow-900/20 border-yellow-300 dark:border-yellow-700'
                                        } shadow-sm`}>
                                        {/* Certificate Type Badge */}
                                        <div className='flex items-center justify-between mb-3'>
                                          <span
                                            className={`px-2 py-1 text-xs font-medium rounded-full ${
                                              node.is_leaf
                                                ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'
                                                : node.is_root
                                                ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                                : 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                                            }`}>
                                            {node.is_leaf
                                              ? '叶子证书 (服务器)'
                                              : node.is_root
                                              ? '根证书'
                                              : '中间证书'}
                                          </span>

                                          {/* Trust Status Badge */}
                                          <span
                                            className={`px-2 py-1 text-xs font-medium rounded-full ${
                                              node.trust_status === 'end-entity'
                                                ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'
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
                                            <span className='text-gray-600 dark:text-gray-400'>
                                              主题:
                                            </span>
                                            <div className='font-medium text-gray-900 dark:text-white break-all mt-1'>
                                              {extractCN(
                                                node.certificate.subject,
                                              )}
                                            </div>
                                          </div>

                                          {!node.is_root && (
                                            <div>
                                              <span className='text-gray-600 dark:text-gray-400'>
                                                颁发者:
                                              </span>
                                              <div className='font-medium text-gray-900 dark:text-white break-all mt-1'>
                                                {extractCN(
                                                  node.certificate.issuer,
                                                )}
                                              </div>
                                            </div>
                                          )}

                                          <div className='grid grid-cols-1 md:grid-cols-2 gap-2'>
                                            <div>
                                              <span className='text-gray-600 dark:text-gray-400'>
                                                有效期至:
                                              </span>
                                              <div className='font-medium text-gray-900 dark:text-white'>
                                                {formatDate(
                                                  node.certificate.valid_to,
                                                )}
                                              </div>
                                            </div>
                                            <div>
                                              <span className='text-gray-600 dark:text-gray-400'>
                                                签名算法:
                                              </span>
                                              <div className='font-medium text-gray-900 dark:text-white'>
                                                {
                                                  node.certificate
                                                    .signature_algorithm
                                                }
                                              </div>
                                            </div>
                                          </div>

                                          {node.certificate.key_size && (
                                            <div>
                                              <span className='text-gray-600 dark:text-gray-400'>
                                                密钥:
                                              </span>
                                              <span className='ml-2 font-medium text-gray-900 dark:text-white'>
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
                                          <div className='flex items-center gap-2 pt-2 border-t border-gray-200 dark:border-gray-600'>
                                            <button
                                              onClick={() =>
                                                copyToClipboard(
                                                  node.certificate.subject,
                                                )
                                              }
                                              className='text-xs px-2 py-1 bg-gray-100 dark:bg-gray-600 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-200 dark:hover:bg-gray-500 flex items-center gap-1'>
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
                                              className='text-xs px-2 py-1 bg-gray-100 dark:bg-gray-600 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-200 dark:hover:bg-gray-500 flex items-center gap-1'>
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
                            <div className='mt-6 pt-4 border-t border-gray-300 dark:border-gray-600'>
                              <div className='flex flex-wrap gap-4 text-xs'>
                                <div className='flex items-center gap-2'>
                                  <div className='w-3 h-3 rounded-full bg-blue-300 dark:bg-blue-700'></div>
                                  <span className='text-gray-600 dark:text-gray-400'>
                                    叶子证书 (服务器)
                                  </span>
                                </div>
                                <div className='flex items-center gap-2'>
                                  <div className='w-3 h-3 rounded-full bg-yellow-300 dark:bg-yellow-700'></div>
                                  <span className='text-gray-600 dark:text-gray-400'>
                                    中间证书
                                  </span>
                                </div>
                                <div className='flex items-center gap-2'>
                                  <div className='w-3 h-3 rounded-full bg-green-300 dark:bg-green-700'></div>
                                  <span className='text-gray-600 dark:text-gray-400'>
                                    根证书
                                  </span>
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>

                        {/* Chain Analysis Summary */}
                        <div className='bg-gray-50 dark:bg-gray-700/50 p-4 rounded-lg'>
                          <h4 className='font-medium text-gray-900 dark:text-white mb-3'>
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
                              <span className='text-gray-700 dark:text-gray-300'>
                                证书链
                                {sslInfo.certificate_chain.is_complete
                                  ? '完整'
                                  : '不完整'}
                                ，共包含{' '}
                                {sslInfo.certificate_chain.chain_length} 个证书
                              </span>
                            </div>

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
                              <span className='text-gray-700 dark:text-gray-300'>
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
                                className='w-4 h-4 text-blue-500'
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
                              <span className='text-gray-700 dark:text-gray-300'>
                                建议定期监控证书链完整性和各证书的有效期
                              </span>
                            </div>
                          </div>
                        </div>
                      </div>
                    ) : (
                      <div className='text-center py-8 text-gray-500 dark:text-gray-400'>
                        <svg
                          className='w-12 h-12 mx-auto mb-4 text-gray-400'
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
                        <div className='text-lg font-medium text-gray-900 dark:text-white mb-2'>
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
                          <div className='p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg'>
                            <div className='flex items-center justify-between'>
                              <span className='font-medium text-blue-800 dark:text-blue-200'>
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
                            <div className='text-sm text-blue-700 dark:text-blue-300 mt-2'>
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
                                <h4 className='font-medium text-gray-900 dark:text-white'>
                                  {protocol.version} 加密套件 (
                                  {protocol.cipher_suites.length})
                                </h4>
                                <div className='flex items-center gap-2'>
                                  <span
                                    className={`text-xs px-2 py-1 rounded-full ${
                                      protocol.version.includes('TLS 1.3')
                                        ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                        : protocol.version.includes('TLS 1.2')
                                        ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'
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
                                          : 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200'
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
                                          : 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200'
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
                                          : 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200'
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
                                    <span className='text-sm text-gray-600 dark:text-gray-400'>
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
                                      className='flex items-center justify-between py-2 px-3 bg-gray-50 dark:bg-gray-700 rounded'>
                                      <div className='flex-1 min-w-0'>
                                        <div className='font-mono text-sm text-gray-900 dark:text-white truncate'>
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
                                                : 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200'
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
                                        className='p-1 hover:bg-gray-200 dark:hover:bg-gray-600 rounded ml-2'>
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
                      <div className='text-center py-8 text-gray-500 dark:text-gray-400'>
                        未获取到加密套件信息
                      </div>
                    )}
                  </div>
                )}

                {/* CVE Vulnerabilities Tab */}
                {activeTab === 'cve' && (
                  <div className='space-y-6'>
                    {sslInfo.cve_vulnerabilities &&
                    sslInfo.cve_vulnerabilities.length > 0 ? (
                      <div className='space-y-4'>
                        <div className='text-sm text-gray-600 dark:text-gray-400'>
                          已检测 {sslInfo.cve_vulnerabilities.length} 个安全漏洞
                          (包括受影响和未受影响的)
                        </div>

                        {/* Severity Filter */}
                        <div className='flex flex-wrap gap-2 p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg'>
                          <span className='text-sm font-medium text-gray-700 dark:text-gray-300 mr-2'>
                            漏洞等级过滤:
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
                                    ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'
                                    : 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200'
                                  : 'bg-gray-50 text-gray-600 hover:bg-gray-100 dark:bg-gray-600 dark:text-gray-300 dark:hover:bg-gray-500'
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
                                className={`border rounded-lg overflow-hidden ${
                                  vuln.affected
                                    ? 'border-red-200 dark:border-red-800 bg-red-50/30 dark:bg-red-900/10'
                                    : 'border-green-200 dark:border-green-800 bg-green-50/30 dark:bg-green-900/10'
                                }`}>
                                {/* Header */}
                                <div
                                  className={`px-4 py-3 flex items-center justify-between ${
                                    vuln.affected
                                      ? vuln.severity === 'CRITICAL'
                                        ? 'bg-red-50 dark:bg-red-900/20 border-b border-red-200 dark:border-red-800'
                                        : vuln.severity === 'HIGH'
                                        ? 'bg-orange-50 dark:bg-orange-900/20 border-b border-orange-200 dark:border-orange-800'
                                        : vuln.severity === 'MEDIUM'
                                        ? 'bg-yellow-50 dark:bg-yellow-900/20 border-b border-yellow-200 dark:border-yellow-800'
                                        : 'bg-blue-50 dark:bg-blue-900/20 border-b border-blue-200 dark:border-blue-800'
                                      : 'bg-green-50 dark:bg-green-900/20 border-b border-green-200 dark:border-green-800'
                                  }`}>
                                  <div className='flex items-center gap-3'>
                                    {/* Status Badge */}
                                    <span
                                      className={`px-2 py-1 text-xs font-medium rounded-full ${
                                        vuln.affected
                                          ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                                          : 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                                      }`}>
                                      {vuln.affected ? '受影响' : '未受影响'}
                                    </span>

                                    {/* Severity Badge */}
                                    <span
                                      className={`px-2 py-1 text-xs font-medium rounded-full ${
                                        vuln.severity === 'CRITICAL'
                                          ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                                          : vuln.severity === 'HIGH'
                                          ? 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200'
                                          : vuln.severity === 'MEDIUM'
                                          ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                                          : 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'
                                      }`}>
                                      {vuln.severity}
                                    </span>

                                    <div>
                                      <h4 className='font-medium text-gray-900 dark:text-white'>
                                        {vuln.name} ({vuln.cve_id})
                                      </h4>
                                      <div className='text-xs text-gray-600 dark:text-gray-400 mt-1'>
                                        影响组件:{' '}
                                        {vuln.affected_components.join(', ')}
                                      </div>
                                    </div>
                                  </div>
                                </div>

                                {/* Description */}
                                <div className='px-4 py-3 bg-white dark:bg-gray-800'>
                                  <p className='text-sm text-gray-700 dark:text-gray-300'>
                                    {vuln.description}
                                  </p>
                                </div>

                                {/* Status Message */}
                                <div
                                  className={`px-4 py-2 ${
                                    vuln.affected
                                      ? 'bg-red-50 dark:bg-red-900/20 text-red-800 dark:text-red-200'
                                      : 'bg-green-50 dark:bg-green-900/20 text-green-800 dark:text-green-200'
                                  }`}>
                                  <div className='text-sm font-medium'>
                                    {vuln.affected
                                      ? '⚠️ 当前配置存在此漏洞风险'
                                      : '✅ 当前配置不受此漏洞影响'}
                                  </div>
                                </div>

                                {/* Remediation */}
                                <div className='px-4 py-3 bg-gray-50 dark:bg-gray-700/50'>
                                  <h5 className='text-sm font-medium text-gray-900 dark:text-white mb-2'>
                                    修复建议:
                                  </h5>
                                  <p className='text-sm text-gray-700 dark:text-gray-300'>
                                    {vuln.remediation}
                                  </p>
                                </div>

                                {/* References */}
                                {vuln.references.length > 0 && (
                                  <div className='px-4 py-3 bg-white dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700'>
                                    <h5 className='text-sm font-medium text-gray-900 dark:text-white mb-2'>
                                      参考资料:
                                    </h5>
                                    <div className='space-y-1'>
                                      {vuln.references.map((ref, refIndex) => (
                                        <a
                                          key={refIndex}
                                          href={ref}
                                          target='_blank'
                                          rel='noopener noreferrer'
                                          className='text-sm text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300 block truncate'>
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
                        <div className='mt-6 p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg'>
                          <h5 className='text-sm font-medium text-gray-900 dark:text-white mb-2'>
                            检测总结:
                          </h5>
                          <div className='grid grid-cols-1 md:grid-cols-3 gap-4 text-sm'>
                            <div>
                              <span className='text-red-600 dark:text-red-400 font-medium'>
                                受影响的漏洞:{' '}
                                {
                                  sslInfo.cve_vulnerabilities.filter(
                                    (v) => v.affected,
                                  ).length
                                }
                              </span>
                            </div>
                            <div>
                              <span className='text-green-600 dark:text-green-400 font-medium'>
                                未受影响的漏洞:{' '}
                                {
                                  sslInfo.cve_vulnerabilities.filter(
                                    (v) => !v.affected,
                                  ).length
                                }
                              </span>
                            </div>
                            <div>
                              <span className='text-blue-600 dark:text-blue-400 font-medium'>
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
                      </div>
                    ) : (
                      <div className='text-center py-8 text-gray-500 dark:text-gray-400'>
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
                        <div className='text-lg font-medium text-gray-900 dark:text-white mb-2'>
                          未进行漏洞检测
                        </div>
                        <div className='text-sm'>
                          请先运行 SSL 检测以获取完整的漏洞检测报告
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
                          {getSecurityLabel(sslInfo.security_score)}
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
                          <h4 className='font-medium text-blue-600 dark:text-blue-400 mb-3 flex items-center'>
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
                                className='p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded'>
                                <div className='text-sm text-blue-800 dark:text-blue-200'>
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
                        <div className='text-center py-8 text-gray-500 dark:text-gray-400'>
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
        <div className='fixed bottom-6 right-6 bg-gray-900 text-white dark:bg-gray-100 dark:text-gray-900 px-3 py-2 rounded shadow-md text-sm z-50'>
          {toast}
        </div>
      )}
    </ToolLayout>
  )
}

export default SslChecker
