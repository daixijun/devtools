import { invoke } from '@tauri-apps/api/core'
import React, { useEffect, useState } from 'react'
import { Button } from '../components/common'
import FileUpload from '../components/common/FileUpload'
import { ToolLayout } from '../components/layouts'

// 右侧全屏证书展示组件
const RightSideFullScreenCertificateView: React.FC<{
  chainInfo: CertificateChainInfo
  output: Array<{
    label: string
    value: string
    isExpiryWarning?: boolean
    indentLevel?: number
    isSectionHeader?: boolean
    statusType?: 'valid' | 'warning' | 'expired'
    certIndex?: number
  }>
  activeTab: number
  onBack: () => void
}> = ({ chainInfo, output, activeTab, onBack }) => {
  const [currentTab, setCurrentTab] = useState(activeTab)

  const getCertificateType = (chainLevel: number) => {
    switch (chainLevel) {
      case 0:
        return '终端证书'
      case 1:
        return '中间CA证书'
      case 2:
        return '根CA证书'
      default:
        return '未知类型'
    }
  }

  // 过滤显示特定证书的数据
  const getOutputForCertificate = (certIndex: number | null) => {
    if (certIndex === null) {
      return output.filter((item) => item.certIndex === undefined)
    }
    return output.filter((item) => item.certIndex === certIndex)
  }

  return (
    <div className='fixed inset-y-0 right-0 left-80 bg-white dark:bg-gray-900 z-50 flex flex-col'>
      {/* 顶部导航栏 */}
      <div className='flex-shrink-0 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 border-l px-6 py-4'>
        <div className='flex items-center justify-between'>
          <div className='flex items-center space-x-4'>
            <Button
              variant='secondary'
              size='sm'
              onClick={onBack}
              className='flex items-center space-x-2'>
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
              <span>退出全屏</span>
            </Button>
            <h1 className='text-xl font-semibold text-gray-900 dark:text-white'>
              证书详细信息
            </h1>
          </div>
          <div className='text-sm text-gray-500 dark:text-gray-400'>
            右侧全屏模式
          </div>
        </div>
      </div>

      {/* 标签页导航 */}
      <div className='flex-shrink-0 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 border-l border-gray-200 dark:border-gray-700'>
        <div className='px-6'>
          <nav className='flex space-x-8' aria-label='证书标签页'>
            <button
              onClick={() => setCurrentTab(0)}
              className={`py-4 px-1 border-b-2 font-medium text-sm whitespace-nowrap ${
                currentTab === 0
                  ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 hover:border-gray-300 dark:hover:border-gray-600'
              }`}>
              证书链概览
            </button>
            {chainInfo.certificates.map((cert, index) => {
              const certType = getCertificateType(cert.chain_level)
              return (
                <button
                  key={index + 1}
                  onClick={() => setCurrentTab(index + 1)}
                  className={`py-4 px-1 border-b-2 font-medium text-sm whitespace-nowrap ${
                    currentTab === index + 1
                      ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                      : 'border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 hover:border-gray-300 dark:hover:border-gray-600'
                  }`}>
                  证书 {index + 1} - {certType}
                </button>
              )
            })}
          </nav>
        </div>
      </div>

      {/* 内容区域 */}
      <div className='flex-1 overflow-auto bg-gray-50 dark:bg-gray-900'>
        <div className='max-w-6xl mx-auto p-6'>
          {/* 证书链状态卡片 */}
          <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden mb-6'>
            <div className='p-4'>
              <div className='flex items-center justify-between'>
                <div className='flex items-center space-x-3'>
                  <div
                    className={`flex-shrink-0 w-3 h-3 rounded-full ${
                      chainInfo.is_full_chain ? 'bg-green-500' : 'bg-red-500'
                    }`}></div>
                  <span
                    className={`font-medium ${
                      chainInfo.is_full_chain
                        ? 'text-green-700 dark:text-green-400'
                        : 'text-red-700 dark:text-red-400'
                    }`}>
                    {chainInfo.is_full_chain
                      ? '完整证书链'
                      : '证书链不完整 - 缺少CA证书'}
                  </span>
                </div>
                <div
                  className={`text-sm font-medium ${
                    chainInfo.is_full_chain
                      ? 'text-green-600 dark:text-green-400'
                      : 'text-red-600 dark:text-red-400'
                  }`}>
                  {chainInfo.chain_status}
                </div>
              </div>
            </div>
          </div>

          {/* 当前标签页的内容 */}
          <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden'>
            <div className='p-6 space-y-6'>
              {/* 证书链概览标签页 */}
              {currentTab === 0 && (
                <>
                  {/* 证书颁发关系图 */}
                  <div className='border-t border-gray-200 dark:border-gray-700 pt-6'>
                    <h3 className='text-xl font-semibold text-gray-900 dark:text-white mb-6'>
                      证书颁发关系图
                    </h3>
                    <CertificateChainVisualization chainInfo={chainInfo} />
                  </div>

                  {getOutputForCertificate(null).map((item, index) => (
                    <div
                      key={index}
                      className={`${
                        item.isSectionHeader
                          ? 'border-t border-gray-200 dark:border-gray-700 pt-6 mt-6 first:border-t-0 first:pt-0 first:mt-0'
                          : 'mb-3'
                      }`}>
                      {item.label && (
                        <div
                          className={`flex items-start ${
                            item.indentLevel === 1
                              ? 'ml-0'
                              : item.indentLevel === 2
                              ? 'ml-6'
                              : ''
                          }`}>
                          <span
                            className={`font-medium text-gray-700 dark:text-gray-300 ${
                              item.isSectionHeader
                                ? 'text-xl font-semibold text-gray-900 dark:text-white mb-3 block'
                                : 'inline-block min-w-40'
                            }`}>
                            {item.label}
                            {!item.isSectionHeader && ':'}
                          </span>
                          <span
                            className={`flex-1 ml-3 text-base ${
                              item.statusType === 'expired'
                                ? 'text-red-600 dark:text-red-400 font-medium'
                                : item.statusType === 'warning'
                                ? 'text-yellow-600 dark:text-yellow-400 font-medium'
                                : item.statusType === 'valid'
                                ? 'text-green-600 dark:text-green-400 font-medium'
                                : item.isExpiryWarning
                                ? 'text-red-600 dark:text-red-400 font-medium'
                                : 'text-gray-600 dark:text-gray-400'
                            } ${item.isSectionHeader ? 'hidden' : ''}`}>
                            {item.value}
                          </span>
                        </div>
                      )}
                    </div>
                  ))}
                </>
              )}

              {/* 单个证书标签页 */}
              {currentTab > 0 && (
                <>
                  {getOutputForCertificate(currentTab - 1).map(
                    (item, index) => {
                      // 跳过证书标题，因为已经在标签页中显示
                      if (
                        item.label?.startsWith('证书 ') &&
                        item.isSectionHeader &&
                        item.certIndex === currentTab - 1
                      ) {
                        return null
                      }

                      return (
                        <div
                          key={index}
                          className={`${
                            item.isSectionHeader
                              ? 'border-t border-gray-200 dark:border-gray-700 pt-6 mt-6 first:border-t-0 first:pt-0 first:mt-0'
                              : 'mb-3'
                          }`}>
                          {item.label && (
                            <div
                              className={`flex items-start ${
                                item.indentLevel === 1
                                  ? 'ml-0'
                                  : item.indentLevel === 2
                                  ? 'ml-6'
                                  : ''
                              }`}>
                              <span
                                className={`font-medium text-gray-700 dark:text-gray-300 ${
                                  item.isSectionHeader
                                    ? 'text-xl font-semibold text-gray-900 dark:text-white mb-3 block'
                                    : 'inline-block min-w-40'
                                }`}>
                                {item.label}
                                {!item.isSectionHeader && ':'}
                              </span>
                              <span
                                className={`flex-1 ml-3 text-base ${
                                  item.statusType === 'expired'
                                    ? 'text-red-600 dark:text-red-400 font-medium'
                                    : item.statusType === 'warning'
                                    ? 'text-yellow-600 dark:text-yellow-400 font-medium'
                                    : item.statusType === 'valid'
                                    ? 'text-green-600 dark:text-green-400 font-medium'
                                    : item.isExpiryWarning
                                    ? 'text-red-600 dark:text-red-400 font-medium'
                                    : 'text-gray-600 dark:text-gray-400'
                                } ${item.isSectionHeader ? 'hidden' : ''}`}>
                                {item.value}
                              </span>
                            </div>
                          )}
                        </div>
                      )
                    },
                  )}
                </>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

// 证书链可视化组件
const CertificateChainVisualization: React.FC<{
  chainInfo: CertificateChainInfo
}> = ({ chainInfo }) => {
  const getCertificateIcon = (chainLevel: number) => {
    switch (chainLevel) {
      case 2:
        return (
          <div className='w-8 h-8 bg-red-100 dark:bg-red-900/20 rounded-full flex items-center justify-center'>
            <svg
              className='w-5 h-5 text-red-600 dark:text-red-400'
              fill='none'
              stroke='currentColor'
              viewBox='0 0 24 24'>
              <path
                strokeLinecap='round'
                strokeLinejoin='round'
                strokeWidth={2}
                d='M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z'
              />
            </svg>
          </div>
        )
      case 1:
        return (
          <div className='w-8 h-8 bg-yellow-100 dark:bg-yellow-900/20 rounded-full flex items-center justify-center'>
            <svg
              className='w-5 h-5 text-yellow-600 dark:text-yellow-400'
              fill='none'
              stroke='currentColor'
              viewBox='0 0 24 24'>
              <path
                strokeLinecap='round'
                strokeLinejoin='round'
                strokeWidth={2}
                d='M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z'
              />
            </svg>
          </div>
        )
      case 0:
        return (
          <div className='w-8 h-8 bg-blue-100 dark:bg-blue-900/20 rounded-full flex items-center justify-center'>
            <svg
              className='w-5 h-5 text-blue-600 dark:text-blue-400'
              fill='none'
              stroke='currentColor'
              viewBox='0 0 24 24'>
              <path
                strokeLinecap='round'
                strokeLinejoin='round'
                strokeWidth={2}
                d='M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z'
              />
            </svg>
          </div>
        )
      default:
        return (
          <div className='w-8 h-8 bg-gray-100 dark:bg-gray-700 rounded-full flex items-center justify-center'>
            <svg
              className='w-5 h-5 text-gray-500 dark:text-gray-400'
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
          </div>
        )
    }
  }

  const getCertificateColor = (chainLevel: number) => {
    switch (chainLevel) {
      case 2:
        return 'border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/10'
      case 1:
        return 'border-yellow-200 dark:border-yellow-800 bg-yellow-50 dark:bg-yellow-900/10'
      case 0:
        return 'border-blue-200 dark:border-blue-800 bg-blue-50 dark:bg-blue-900/10'
      default:
        return 'border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800'
    }
  }

  const getCertificateType = (chainLevel: number) => {
    switch (chainLevel) {
      case 0:
        return '终端证书'
      case 1:
        return '中间CA证书'
      case 2:
        return '根CA证书'
      default:
        return '未知类型'
    }
  }

  const getCommonName = (cert: CertificateInfo) => {
    return (
      cert.subject['通用名称 (CN)'] || cert.subject['组织名称 (O)'] || '未知'
    )
  }

  const getIssuerName = (cert: CertificateInfo) => {
    return (
      cert.issuer['通用名称 (CN)'] ||
      cert.issuer['组织名称 (O)'] ||
      '未知颁发者'
    )
  }

  // 按照证书链级别排序证书和缺失的证书，并合并显示
  const allChainItems = [
    ...chainInfo.certificates,
    ...chainInfo.missing_certificates.map((missing) => ({
      ...missing,
      isMissing: true,
    })),
  ].sort((a, b) => b.chain_level - a.chain_level)

  const formatValidityDate = (dateString: string) => {
    try {
      const date = new Date(dateString)
      return date.toLocaleDateString('zh-CN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
      })
    } catch {
      return dateString
    }
  }

  const getExpiryStatus = (daysUntilExpiry: number) => {
    // 前端展示逻辑：
    // 正数：证书未过期，表示剩余天数
    // 负数：证书已过期，绝对值表示过期天数
    const isExpired = daysUntilExpiry < 0
    const remainingDays = Math.abs(daysUntilExpiry)

    if (isExpired) {
      return {
        text: `已过期 ${remainingDays} 天`,
        color: 'text-red-600 dark:text-red-400',
      }
    } else if (remainingDays < 30) {
      return {
        text: `还有 ${remainingDays} 天到期`,
        color: 'text-yellow-600 dark:text-yellow-400',
      }
    } else {
      return {
        text: `还有 ${remainingDays} 天到期`,
        color: 'text-green-600 dark:text-green-400',
      }
    }
  }

  return (
    <div className='space-y-4 p-2'>
      <div className='flex justify-center'>
        <div className='flex flex-col items-center space-y-4'>
          {allChainItems.map((item, index) => {
            const isMissing = 'isMissing' in item && item.isMissing

            if (isMissing) {
              // 显示缺失的证书
              const missingCert = item as MissingCertificateInfo & {
                isMissing: boolean
              }
              const certType = getCertificateType(missingCert.chain_level)

              return (
                <div
                  key={`missing-${index}`}
                  className='flex flex-col items-center'>
                  {/* 缺失证书节点 */}
                  <div className='border-2 border-dashed border-red-300 dark:border-red-700 rounded-lg p-4 max-w-2xl w-full bg-red-50 dark:bg-red-900/10'>
                    <div className='flex items-start space-x-3'>
                      <div className='w-8 h-8 bg-red-100 dark:bg-red-900/20 rounded-full flex items-center justify-center'>
                        <svg
                          className='w-5 h-5 text-red-600 dark:text-red-400'
                          fill='none'
                          stroke='currentColor'
                          viewBox='0 0 24 24'>
                          <path
                            strokeLinecap='round'
                            strokeLinejoin='round'
                            strokeWidth={2}
                            d='M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z'
                          />
                        </svg>
                      </div>
                      <div className='flex-1 min-w-0'>
                        <div className='font-medium text-red-800 dark:text-red-200 mb-1'>
                          {certType} (缺失)
                        </div>
                        <div
                          className='text-sm text-red-600 dark:text-red-400 mb-1 truncate'
                          title={missingCert.subject_name}>
                          <strong>缺失证书:</strong> {missingCert.subject_name}
                        </div>
                        <div className='text-sm text-red-500 dark:text-red-500 mb-2'>
                          {missingCert.description}
                        </div>
                        <div className='text-xs text-red-500 dark:text-red-400 bg-red-100 dark:bg-red-900/20 px-2 py-1 rounded'>
                          💡 此证书缺失可能导致证书链验证失败
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* 连接箭头和颁发关系说明 */}
                  {index < allChainItems.length - 1 && (
                    <div className='flex flex-col items-center my-3'>
                      <div className='text-xs text-red-500 dark:text-red-400 mb-1 text-center'>
                        应该颁发给 ↓
                      </div>
                      <svg
                        className='w-6 h-6 text-red-400 dark:text-red-500'
                        fill='none'
                        stroke='currentColor'
                        viewBox='0 0 24 24'>
                        <path
                          strokeLinecap='round'
                          strokeLinejoin='round'
                          strokeWidth={2}
                          d='M19 14l-7 7m0 0l-7-7m7 7V3'
                        />
                      </svg>
                    </div>
                  )}
                </div>
              )
            } else {
              // 显示现有证书
              const cert = item as CertificateInfo
              const certType = getCertificateType(cert.chain_level)
              const commonName = getCommonName(cert)
              const expiryStatus = getExpiryStatus(
                cert.validity.days_until_expiry,
              )

              return (
                <div
                  key={`cert-${index}`}
                  className='flex flex-col items-center'>
                  {/* 证书节点 */}
                  <div
                    className={`border-2 rounded-lg p-4 max-w-2xl w-full ${getCertificateColor(
                      cert.chain_level,
                    )}`}>
                    <div className='flex items-start space-x-3'>
                      {getCertificateIcon(cert.chain_level)}
                      <div className='flex-1 min-w-0'>
                        <div className='font-medium text-gray-900 dark:text-white mb-1'>
                          {certType}
                        </div>
                        <div
                          className='text-sm text-gray-600 dark:text-gray-400 mb-1 truncate'
                          title={commonName}>
                          <strong>颁发给:</strong> {commonName}
                        </div>
                        {/* 显示颁发者信息 */}
                        <div
                          className='text-sm text-gray-500 dark:text-gray-500 mb-1 truncate'
                          title={getIssuerName(cert)}>
                          <strong>颁发者:</strong> {getIssuerName(cert)}
                        </div>
                        <div className='text-xs text-gray-500 dark:text-gray-500 mb-2 break-all'>
                          序列号: {cert.serial_number}
                        </div>

                        {/* 有效期信息 */}
                        <div className='border-t border-gray-200 dark:border-gray-700 pt-2 mt-2'>
                          <div className='grid grid-cols-2 gap-2 text-xs'>
                            <div>
                              <div className='text-gray-500 dark:text-gray-500'>
                                开始时间
                              </div>
                              <div className='text-gray-600 dark:text-gray-400'>
                                {formatValidityDate(cert.validity.not_before)}
                              </div>
                            </div>
                            <div>
                              <div className='text-gray-500 dark:text-gray-500'>
                                结束时间
                              </div>
                              <div className='text-gray-600 dark:text-gray-400'>
                                {formatValidityDate(cert.validity.not_after)}
                              </div>
                            </div>
                          </div>
                          <div className='mt-2'>
                            <div className='text-gray-500 dark:text-gray-500'>
                              证书状态
                            </div>
                            <div
                              className={`text-xs font-medium ${expiryStatus.color}`}>
                              {expiryStatus.text}
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* 连接箭头和颁发关系说明 */}
                  {index < allChainItems.length - 1 && (
                    <div className='flex flex-col items-center my-3'>
                      {/* 颁发关系说明 */}
                      <div className='text-xs text-gray-500 dark:text-gray-400 mb-1 text-center'>
                        颁发给 ↓
                      </div>
                      <svg
                        className='w-6 h-6 text-gray-400 dark:text-gray-500'
                        fill='none'
                        stroke='currentColor'
                        viewBox='0 0 24 24'>
                        <path
                          strokeLinecap='round'
                          strokeLinejoin='round'
                          strokeWidth={2}
                          d='M19 14l-7 7m0 0l-7-7m7 7V3'
                        />
                      </svg>
                    </div>
                  )}
                </div>
              )
            }
          })}
        </div>
      </div>

      {/* 图例 */}
      <div className='flex justify-center mt-6'>
        <div className='flex flex-wrap justify-center gap-4 text-sm'>
          <div className='flex items-center space-x-2'>
            <div className='w-4 h-4 bg-red-100 dark:bg-red-900/20 rounded-full flex items-center justify-center'>
              <div className='w-2 h-2 bg-red-600 dark:text-red-400 rounded-full'></div>
            </div>
            <span className='text-gray-600 dark:text-gray-400'>根CA证书</span>
          </div>
          <div className='flex items-center space-x-2'>
            <div className='w-4 h-4 bg-yellow-100 dark:bg-yellow-900/20 rounded-full flex items-center justify-center'>
              <div className='w-2 h-2 bg-yellow-600 dark:text-yellow-400 rounded-full'></div>
            </div>
            <span className='text-gray-600 dark:text-gray-400'>中间CA证书</span>
          </div>
          <div className='flex items-center space-x-2'>
            <div className='w-4 h-4 bg-blue-100 dark:bg-blue-900/20 rounded-full flex items-center justify-center'>
              <div className='w-2 h-2 bg-blue-600 dark:text-blue-400 rounded-full'></div>
            </div>
            <span className='text-gray-600 dark:text-gray-400'>终端证书</span>
          </div>
          <div className='flex items-center space-x-2'>
            <div className='w-4 h-4 border-2 border-dashed border-red-300 dark:border-red-700 rounded-full flex items-center justify-center bg-red-50 dark:bg-red-900/10'>
              <div className='w-2 h-2 bg-red-600 dark:bg-red-400 rounded-full'></div>
            </div>
            <span className='text-gray-600 dark:text-gray-400'>缺失证书</span>
          </div>
        </div>
      </div>
    </div>
  )
}

interface CertificateInfo {
  subject: Record<string, string>
  issuer: Record<string, string>
  validity: {
    not_before: string
    not_after: string
    days_until_expiry: number
  }
  serial_number: string
  signature_algorithm: string
  public_key_info: {
    key_type: string
    key_size?: number
    algorithm: string
  }
  extensions: Array<{
    name: string
    value: string
    critical: boolean
  }>
  sans: string[]
  chain_level: number
  certificate_type?: string
  brand?: string
  sha1_fingerprint?: string
  sha256_fingerprint?: string
}

interface CertificateChainInfo {
  certificates: CertificateInfo[]
  missing_certificates: MissingCertificateInfo[]
  is_full_chain: boolean
  chain_status: string
  ca_download_urls: string[]
  missing_ca_info?: string
}

interface MissingCertificateInfo {
  subject_name: string
  issuer_name: string
  certificate_type: string
  chain_level: number
  description: string
}

const PemCertificateViewer: React.FC = () => {
  const [input, setInput] = useState('')
  const [, setCertificateInfo] = useState<CertificateInfo | null>(null)
  const [chainInfo, setChainInfo] = useState<CertificateChainInfo | null>(null)
  const [output, setOutput] = useState<
    {
      label: string
      value: string
      isExpiryWarning?: boolean
      indentLevel?: number
      isSectionHeader?: boolean
      statusType?: 'valid' | 'warning' | 'expired'
      certIndex?: number
    }[]
  >([])
  const [error, setError] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [activeTab, setActiveTab] = useState(0)
  const [uploadedFileName, setUploadedFileName] = useState('')
  const [uploadedData, setUploadedData] = useState<Uint8Array | null>(null)
  const [pfxPassword, setPfxPassword] = useState('')
  const [fileType, setFileType] = useState<'PEM' | 'PFX'>('PEM')
  const [isRightSideFullScreen, setIsRightSideFullScreen] = useState(false)
  const [isBase64Decode, setIsBase64Decode] = useState(false) // 新增：控制是否进行base64解码

  // 检测文件类型
  const detectFileType = (
    content: string,
    fileName?: string,
  ): 'PEM' | 'PFX' => {
    if (fileName) {
      const name = fileName.toLowerCase()
      if (name.endsWith('.pfx') || name.endsWith('.p12')) {
        return 'PFX'
      }
      if (
        name.endsWith('.pem') ||
        name.endsWith('.crt') ||
        name.endsWith('.cer')
      ) {
        return 'PEM'
      }
    }

    // 基于内容检测
    if (content.includes('-----BEGIN CERTIFICATE-----')) {
      return 'PEM'
    }

    // 如果有文件且不是PEM格式，假定为PFX
    if (fileName) {
      return 'PFX'
    }

    // 默认为PEM（文本输入）
    return 'PEM'
  }

  const parseCertificate = async (
    content: string,
    fileName?: string,
    data?: Uint8Array,
    password?: string,
  ) => {
    if (!content.trim() && !data) {
      resetState()
      return
    }

    setIsLoading(true)
    setError('')

    try {
      const detectedFileType = detectFileType(content, fileName)
      let result: CertificateChainInfo

      // 处理base64解码
      let processedContent = content
      if (isBase64Decode && content.trim()) {
        try {
          // 移除可能的base64前缀和空格
          const base64Content = content.replace(/^data:[^;]+;base64,/, '').replace(/\s/g, '')
          const decodedContent = atob(base64Content)
          processedContent = decodedContent
        } catch (err) {
          throw new Error('Base64解码失败：请检查输入内容是否为有效的Base64编码')
        }
      }

      if (detectedFileType === 'PFX' && data) {
        // 处理PFX文件
        const pfxDataArray = Array.from(data)

        result = (await invoke('parse_pfx_certificate', {
          pfxData: pfxDataArray,
          password: password || null,
        })) as CertificateChainInfo
      } else if (data) {
        // 处理PEM文件（从二进制数据转换为文本）
        const textContent = new TextDecoder().decode(data)
        result = (await invoke('parse_pem_certificate', {
          pemContent: textContent,
        })) as CertificateChainInfo
      } else {
        // 处理PEM文本内容
        result = (await invoke('parse_pem_certificate', {
          pemContent: processedContent,
        })) as CertificateChainInfo
      }

      setChainInfo(result)
      setCertificateInfo(result.certificates[0] || null)
      setActiveTab(0) // 重置到第一个标签页

      // 自动进入右侧全屏模式
      setIsRightSideFullScreen(true)

      // 转换格式用于显示 - 按证书分组
      const displayData: Array<{
        label: string
        value: string
        isExpiryWarning?: boolean
        indentLevel?: number
        isSectionHeader?: boolean
        statusType?: 'valid' | 'warning' | 'expired'
        certIndex?: number
      }> = []

      // CA下载建议
      if (!result.is_full_chain && result.ca_download_urls.length > 0) {
        displayData.push({
          label: 'CA证书下载建议',
          value: '',
          indentLevel: 1,
          isSectionHeader: true,
        })
        result.ca_download_urls.forEach((url: string, index: number) => {
          displayData.push({
            label: `下载链接 ${index + 1}`,
            value: url,
            indentLevel: 2,
          })
        })
      }

      // 缺失CA信息
      if (result.missing_ca_info) {
        displayData.push({
          label: '缺失CA信息',
          value: result.missing_ca_info,
          indentLevel: 1,
          isExpiryWarning: true,
          statusType: 'warning',
        })
      }

      // 为每个证书生成详细信息
      result.certificates.forEach(
        (cert: CertificateInfo, certIndex: number) => {
          const certType = getCertificateType(cert.chain_level)

          // 证书标题
          displayData.push({
            label: `证书 ${certIndex + 1} - ${certType}`,
            value: '',
            indentLevel: 0,
            isSectionHeader: true,
            certIndex,
          })

          // 前端根据时间差值判断过期状态
          // 正数：证书未过期，表示剩余天数
          // 负数：证书已过期，绝对值表示过期天数
          const isExpired = cert.validity.days_until_expiry < 0
          const remainingDays = Math.abs(cert.validity.days_until_expiry)
          const status = isExpired
            ? `已过期 ${remainingDays} 天`
            : `还有 ${remainingDays} 天到期`

          // 主题信息
          displayData.push({
            label: '主题信息',
            value: '',
            indentLevel: 1,
            isSectionHeader: true,
            certIndex,
          })
          Object.entries(cert.subject).forEach(([key, value]) => {
            displayData.push({
              label: key,
              value: value,
              indentLevel: 2,
              certIndex,
            })
          })

          // 颁发者信息
          displayData.push({
            label: '颁发者信息',
            value: '',
            indentLevel: 1,
            isSectionHeader: true,
            certIndex,
          })
          Object.entries(cert.issuer).forEach(([key, value]) => {
            displayData.push({
              label: key,
              value: value,
              indentLevel: 2,
              certIndex,
            })
          })

          // 有效期信息
          displayData.push({
            label: '有效期信息',
            value: '',
            indentLevel: 1,
            isSectionHeader: true,
            certIndex,
          })
          displayData.push({
            label: '开始时间',
            value: cert.validity.not_before,
            indentLevel: 2,
            certIndex,
          })
          displayData.push({
            label: '结束时间',
            value: cert.validity.not_after,
            indentLevel: 2,
            certIndex,
          })
          displayData.push({
            label: '证书状态',
            value: status,
            indentLevel: 2,
            isExpiryWarning: isExpired || (!isExpired && remainingDays < 30),
            statusType: isExpired
              ? 'expired'
              : remainingDays < 30
              ? 'warning'
              : 'valid',
            certIndex,
          })

          // 证书详细信息
          displayData.push({
            label: '证书详细信息',
            value: '',
            indentLevel: 1,
            isSectionHeader: true,
            certIndex,
          })

          // 仅对终端证书显示证书类型和品牌信息
          if (cert.chain_level === 0) {
            if (cert.certificate_type) {
              displayData.push({
                label: '证书类型',
                value: cert.certificate_type,
                indentLevel: 2,
                certIndex,
              })
            }
            if (cert.brand) {
              displayData.push({
                label: '证书品牌',
                value: cert.brand,
                indentLevel: 2,
                certIndex,
              })
            }
          }

          displayData.push({
            label: '序列号',
            value: cert.serial_number,
            indentLevel: 2,
            certIndex,
          })
          displayData.push({
            label: '签名算法',
            value: cert.signature_algorithm,
            indentLevel: 2,
            certIndex,
          })

          // 指纹信息 - 对所有证书显示
          if (cert.sha1_fingerprint) {
            displayData.push({
              label: 'SHA1指纹',
              value: cert.sha1_fingerprint,
              indentLevel: 2,
              certIndex,
            })
          }
          if (cert.sha256_fingerprint) {
            displayData.push({
              label: 'SHA256指纹',
              value: cert.sha256_fingerprint,
              indentLevel: 2,
              certIndex,
            })
          }

          // 公钥信息
          displayData.push({
            label: '公钥信息',
            value: '',
            indentLevel: 1,
            isSectionHeader: true,
            certIndex,
          })
          displayData.push({
            label: '类型',
            value: cert.public_key_info.key_type,
            indentLevel: 2,
            certIndex,
          })
          if (cert.public_key_info.key_size) {
            displayData.push({
              label: '密钥长度',
              value: `${cert.public_key_info.key_size} bits`,
              indentLevel: 2,
              certIndex,
            })
          }
          displayData.push({
            label: '算法',
            value: cert.public_key_info.algorithm,
            indentLevel: 2,
            certIndex,
          })

          // 域名信息 - 仅对终端证书显示
          if (cert.sans.length > 0 && cert.chain_level === 0) {
            displayData.push({
              label: '域名清单 (SAN)',
              value: '',
              indentLevel: 1,
              isSectionHeader: true,
              certIndex,
            })
            cert.sans.forEach((domain: string, index: number) => {
              displayData.push({
                label: `域名 ${index + 1}`,
                value: domain,
                indentLevel: 2,
                certIndex,
              })
            })
          }
        },
      )

      setOutput(displayData)
    } catch (err) {
      const errorMessage = typeof err === 'string' ? err : '证书解析失败'
      setError(errorMessage)
    } finally {
      setIsLoading(false)
    }
  }

  const getCertificateType = (chainLevel: number) => {
    switch (chainLevel) {
      case 0:
        return '终端证书'
      case 1:
        return '中间CA证书'
      case 2:
        return '根CA证书'
      default:
        return '未知类型'
    }
  }

  // 过滤显示特定证书的数据
  const getOutputForCertificate = (certIndex: number | null) => {
    if (certIndex === null) {
      // 显示证书链状态信息（没有 certIndex 的项目）
      return output.filter((item) => item.certIndex === undefined)
    }
    // 显示特定证书的信息
    return output.filter((item) => item.certIndex === certIndex)
  }

  // 处理证书文件数据（PEM和PFX都支持）
  const handleCertificateBinaryFileData = (
    fileName: string,
    data: Uint8Array,
  ) => {
    const detectedType = detectFileType('', fileName)

    setUploadedData(data)
    setUploadedFileName(fileName)
    setFileType(detectedType)
    setError('')
    setInput('') // 清空文本输入
  }

  // 监听输入变化
  useEffect(() => {
    if (!input.trim() && !uploadedData) {
      // 清空输入时重置状态
      resetState()
    }
  }, [input, uploadedData])

  // 监听文本输入变化，如果有文本输入则清除上传的文件状态
  useEffect(() => {
    if (input.trim() && uploadedData) {
      // 用户在文本框中输入内容，清除文件上传状态
      setUploadedData(null)
      setUploadedFileName('')
      setPfxPassword('')
      setFileType('PEM')
    }
  }, [input])

  // 手动解析证书
  const handleParseCertificate = () => {
    if (uploadedData && uploadedFileName) {
      const detectedFileType = detectFileType('', uploadedFileName)
      if (detectedFileType === 'PFX') {
        parseCertificate('', uploadedFileName, uploadedData, pfxPassword)
      } else {
        parseCertificate('', uploadedFileName, uploadedData)
      }
    } else if (input.trim()) {
      parseCertificate(input)
    }
  }

  // 处理密码输入框的回车键
  const handlePasswordKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleParseCertificate()
    }
  }

  // 创建适配函数来处理错误信息设置
  const handleError = (errorMsg: string | null) => {
    setError(errorMsg ?? '')
  }

  // 重置状态
  const resetState = () => {
    setCertificateInfo(null)
    setChainInfo(null)
    setOutput([])
    setError('')
    setActiveTab(0)
    setUploadedData(null)
    setUploadedFileName('')
    setPfxPassword('')
    setFileType('PEM')
    setIsBase64Decode(false) // 重置base64解码选项
  }

  // 如果处于右侧全屏模式，显示右侧全屏组件
  if (isRightSideFullScreen && chainInfo) {
    return (
      <RightSideFullScreenCertificateView
        chainInfo={chainInfo}
        output={output}
        activeTab={activeTab}
        onBack={() => setIsRightSideFullScreen(false)}
      />
    )
  }

  return (
    <ToolLayout
      title='证书查看器'
      subtitle='解析和查看PEM、PFX格式的X.509证书信息，支持证书链检测、CA证书下载建议和多证书展示'>
      <div className='flex flex-col h-full space-y-6'>
        {/* 统一的证书文件上传 */}
        <div className='space-y-4 mb-4'>
          <FileUpload
            value={input}
            onChange={setInput}
            onError={handleError}
            error={error}
            accept='.pem,.crt,.cer,.pfx,.p12'
            fileType='binary'
            onBinaryFileData={handleCertificateBinaryFileData}
            placeholder='请输入PEM证书内容或上传证书文件...

支持格式：
- PEM文本格式 (.pem, .crt, .cer)
- PFX二进制格式 (.pfx, .p12)

注意：输入内容后会自动解析证书'
          />

          {/* 显示已上传的文件信息 */}
          {uploadedFileName && (
            <div className='p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg'>
              <div className='flex items-center justify-between'>
                <div className='flex items-center space-x-2'>
                  <svg
                    className='w-5 h-5 text-blue-600 dark:text-blue-400'
                    fill='none'
                    stroke='currentColor'
                    viewBox='0 0 24 24'>
                    <path
                      strokeLinecap='round'
                      strokeLinejoin='round'
                      strokeWidth={2}
                      d='M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z'
                    />
                  </svg>
                  <span className='text-blue-800 dark:text-blue-200 font-medium'>
                    已上传文件: {uploadedFileName}
                  </span>
                  <span className='text-xs px-2 py-1 bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300 rounded'>
                    {fileType}
                  </span>
                </div>
              </div>
            </div>
          )}

          {/* PFX密码输入（仅在上传PFX文件时显示） */}
          {uploadedData && fileType === 'PFX' && (
            <div className='p-4 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg'>
              <div className='flex items-start space-x-3'>
                <div className='w-5 h-5 text-yellow-600 dark:text-yellow-400 mt-0.5'>
                  <svg fill='none' stroke='currentColor' viewBox='0 0 24 24'>
                    <path
                      strokeLinecap='round'
                      strokeLinejoin='round'
                      strokeWidth={2}
                      d='M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z'
                    />
                  </svg>
                </div>
                <div className='flex-1'>
                  <label className='text-yellow-800 dark:text-yellow-200 font-medium mb-2 block'>
                    PFX文件密码（可选）
                  </label>
                  <input
                    type='password'
                    value={pfxPassword}
                    onChange={(e) => setPfxPassword(e.target.value)}
                    onKeyPress={handlePasswordKeyPress}
                    placeholder='请输入PFX文件密码，如无密码请留空'
                    className='w-full px-3 py-2 border border-yellow-300 dark:border-yellow-700 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-yellow-500 focus:border-yellow-500 bg-white dark:bg-gray-800 text-gray-900 dark:text-white'
                  />
                  <p className='text-yellow-700 dark:text-yellow-300 text-sm mt-2'>
                    如果PFX文件没有密码保护，请留空此字段
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* Base64解码选项（仅在文本输入时显示） */}
          {!uploadedData && (
            <div className='p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg'>
              <div className='flex items-start space-x-3'>
                <div className='flex items-center h-5'>
                  <input
                    id='base64-decode'
                    name='base64-decode'
                    type='checkbox'
                    checked={isBase64Decode}
                    onChange={(e) => setIsBase64Decode(e.target.checked)}
                    className='w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500'
                  />
                </div>
                <div className='flex-1'>
                  <label htmlFor='base64-decode' className='text-blue-800 dark:text-blue-200 font-medium'>
                    Base64解码
                  </label>
                  <p className='text-blue-700 dark:text-blue-300 text-sm mt-1'>
                    如果输入的证书内容是Base64编码的，请勾选此选项进行解码后再解析
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* 操作按钮区域 */}
        <div>
          <Button
            variant='primary'
            size='md'
            onClick={handleParseCertificate}
            disabled={isLoading || (!input.trim() && !uploadedData)}
            className='w-full'>
            {isLoading ? '解析中...' : '解析证书'}
          </Button>
        </div>

        <div className='flex-1 min-h-0 overflow-auto'>
          {/* 初始状态 */}
          {!isLoading &&
            !input &&
            !error &&
            (!output || output.length === 0) && (
              <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm p-8'>
                <div className='flex flex-col items-center justify-center space-y-4'>
                  <div className='w-16 h-16 bg-blue-100 dark:bg-blue-900/20 rounded-full flex items-center justify-center'>
                    <svg
                      className='w-8 h-8 text-blue-600 dark:text-blue-400'
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
                  </div>
                  <div className='text-center'>
                    <p className='text-gray-900 dark:text-white font-medium text-lg'>
                      欢迎使用证书查看器
                    </p>
                    <p className='text-gray-600 dark:text-gray-400 mt-2'>
                      支持PEM和PFX格式证书，提供证书链分析和CA下载建议和多证书展示
                    </p>
                    <div className='mt-4 text-sm text-gray-500 dark:text-gray-500'>
                      <p>支持的格式：.pem, .crt, .pfx, .p12</p>
                      <p>支持多个证书：自动检测证书链完整性</p>
                      <p>
                        示例格式：-----BEGIN CERTIFICATE----- 或 PFX二进制文件
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            )}

          {/* 空状态 */}
          {!isLoading &&
            input &&
            !error &&
            (!output || output.length === 0) && (
              <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm p-8'>
                <div className='flex flex-col items-center justify-center space-y-4'>
                  <div className='w-12 h-12 bg-gray-100 dark:bg-gray-700 rounded-full flex items-center justify-center'>
                    <svg
                      className='w-6 h-6 text-gray-400 dark:text-gray-500'
                      fill='none'
                      stroke='currentColor'
                      viewBox='0 0 24 24'>
                      <path
                        strokeLinecap='round'
                        strokeLinejoin='round'
                        strokeWidth={2}
                        d='M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z'
                      />
                    </svg>
                  </div>
                  <div className='text-center'>
                    <p className='text-gray-600 dark:text-gray-400 font-medium'>
                      等待证书输入
                    </p>
                    <p className='text-sm text-gray-500 dark:text-gray-500 mt-1'>
                      请输入PEM格式的证书内容
                    </p>
                  </div>
                </div>
              </div>
            )}

          {isLoading && (
            <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm p-8'>
              <div className='flex flex-col items-center justify-center space-y-4'>
                <div className='animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500'></div>
                <div className='text-center'>
                  <p className='text-gray-600 dark:text-gray-400 font-medium'>
                    正在解析证书...
                  </p>
                  <p className='text-sm text-gray-500 dark:text-gray-500 mt-1'>
                    请稍候，我们正在分析证书信息
                  </p>
                </div>
              </div>
            </div>
          )}

          {!isLoading && output && output.length > 0 && (
            <div className='w-full space-y-4 pb-4'>
              {/* 证书链状态卡片 */}
              {chainInfo && (
                <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden'>
                  <div className='p-4'>
                    <div className='flex items-center justify-between'>
                      <div className='flex items-center space-x-3'>
                        <div
                          className={`flex-shrink-0 w-3 h-3 rounded-full ${
                            chainInfo.is_full_chain
                              ? 'bg-green-500'
                              : 'bg-red-500'
                          }`}></div>
                        <span
                          className={`font-medium ${
                            chainInfo.is_full_chain
                              ? 'text-green-700 dark:text-green-400'
                              : 'text-red-700 dark:text-red-400'
                          }`}>
                          {chainInfo.is_full_chain
                            ? '完整证书链'
                            : '证书链不完整 - 缺少CA证书'}
                        </span>
                      </div>
                      <div
                        className={`text-sm font-medium ${
                          chainInfo.is_full_chain
                            ? 'text-green-600 dark:text-green-400'
                            : 'text-red-600 dark:text-red-400'
                        }`}>
                        {chainInfo.chain_status}
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {/* 标签页导航 */}
              {chainInfo && (
                <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden'>
                  <div className='border-b border-gray-200 dark:border-gray-700'>
                    <div className='flex items-center justify-between px-4'>
                      <nav className='flex space-x-8' aria-label='证书标签页'>
                        <button
                          onClick={() => setActiveTab(0)}
                          className={`py-4 px-1 border-b-2 font-medium text-sm whitespace-nowrap ${
                            activeTab === 0
                              ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                              : 'border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 hover:border-gray-300 dark:hover:border-gray-600'
                          }`}>
                          证书链概览
                        </button>
                        {chainInfo.certificates.map((cert, index) => {
                          const certType = getCertificateType(cert.chain_level)
                          return (
                            <button
                              key={index + 1}
                              onClick={() => setActiveTab(index + 1)}
                              className={`py-4 px-1 border-b-2 font-medium text-sm whitespace-nowrap ${
                                activeTab === index + 1
                                  ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 hover:border-gray-300 dark:hover:border-gray-600'
                              }`}>
                              证书 {index + 1} - {certType}
                            </button>
                          )
                        })}
                      </nav>
                      <Button
                        variant='primary'
                        size='sm'
                        onClick={() => setIsRightSideFullScreen(true)}
                        className='flex items-center space-x-2'>
                        <svg
                          className='w-4 h-4'
                          fill='none'
                          stroke='currentColor'
                          viewBox='0 0 24 24'>
                          <path
                            strokeLinecap='round'
                            strokeLinejoin='round'
                            strokeWidth={2}
                            d='M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4'
                          />
                        </svg>
                        <span>右侧全屏</span>
                      </Button>
                    </div>
                  </div>
                </div>
              )}

              {/* 当前标签页的内容 */}
              <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden'>
                <div className='p-4 space-y-4'>
                  {/* 证书链概览标签页 */}
                  {activeTab === 0 && chainInfo && (
                    <>
                      {/* 证书颁发关系图 */}
                      <div className='border-t border-gray-200 dark:border-gray-700 pt-4'>
                        <h3 className='text-lg font-semibold text-gray-900 dark:text-white mb-4'>
                          证书颁发关系图
                        </h3>
                        <CertificateChainVisualization chainInfo={chainInfo} />
                      </div>

                      {getOutputForCertificate(null).map((item, index) => (
                        <div
                          key={index}
                          className={`${
                            item.isSectionHeader
                              ? 'border-t border-gray-200 dark:border-gray-700 pt-4 mt-4 first:border-t-0 first:pt-0 first:mt-0'
                              : 'mb-2'
                          }`}>
                          {item.label && (
                            <div
                              className={`flex items-start ${
                                item.indentLevel === 1
                                  ? 'ml-0'
                                  : item.indentLevel === 2
                                  ? 'ml-6'
                                  : ''
                              }`}>
                              <span
                                className={`font-medium text-gray-700 dark:text-gray-300 ${
                                  item.isSectionHeader
                                    ? 'text-xl font-semibold text-gray-900 dark:text-white mb-2 block'
                                    : 'inline-block min-w-32'
                                }`}>
                                {item.label}
                                {!item.isSectionHeader && ':'}
                              </span>
                              <span
                                className={`flex-1 ml-2 ${
                                  item.statusType === 'expired'
                                    ? 'text-red-600 dark:text-red-400 font-medium'
                                    : item.statusType === 'warning'
                                    ? 'text-yellow-600 dark:text-yellow-400 font-medium'
                                    : item.statusType === 'valid'
                                    ? 'text-green-600 dark:text-green-400 font-medium'
                                    : item.isExpiryWarning
                                    ? 'text-red-600 dark:text-red-400 font-medium'
                                    : 'text-gray-600 dark:text-gray-400'
                                } ${item.isSectionHeader ? 'hidden' : ''}`}>
                                {item.value}
                              </span>
                            </div>
                          )}
                        </div>
                      ))}
                    </>
                  )}

                  {/* 单个证书标签页 */}
                  {activeTab > 0 && chainInfo && (
                    <>
                      {getOutputForCertificate(activeTab - 1).map(
                        (item, index) => {
                          // 跳过证书标题，因为已经在标签页中显示
                          if (
                            item.label?.startsWith('证书 ') &&
                            item.isSectionHeader &&
                            item.certIndex === activeTab - 1
                          ) {
                            return null
                          }

                          return (
                            <div
                              key={index}
                              className={`${
                                item.isSectionHeader
                                  ? 'border-t border-gray-200 dark:border-gray-700 pt-4 mt-4 first:border-t-0 first:pt-0 first:mt-0'
                                  : 'mb-3'
                              }`}>
                              {item.label && (
                                <div
                                  className={`flex items-start ${
                                    item.indentLevel === 1
                                      ? 'ml-0'
                                      : item.indentLevel === 2
                                      ? 'ml-6'
                                      : ''
                                  }`}>
                                  <span
                                    className={`font-medium text-gray-700 dark:text-gray-300 ${
                                      item.isSectionHeader
                                        ? 'text-xl font-semibold text-gray-900 dark:text-white mb-3 block'
                                        : 'inline-block min-w-32'
                                    }`}>
                                    {item.label}
                                    {!item.isSectionHeader && ':'}
                                  </span>
                                  <span
                                    className={`flex-1 ml-3 text-base ${
                                      item.statusType === 'expired'
                                        ? 'text-red-600 dark:text-red-400 font-medium'
                                        : item.statusType === 'warning'
                                        ? 'text-yellow-600 dark:text-yellow-400 font-medium'
                                        : item.statusType === 'valid'
                                        ? 'text-green-600 dark:text-green-400 font-medium'
                                        : item.isExpiryWarning
                                        ? 'text-red-600 dark:text-red-400 font-medium'
                                        : 'text-gray-600 dark:text-gray-400'
                                    } ${item.isSectionHeader ? 'hidden' : ''}`}>
                                    {item.value}
                                  </span>
                                </div>
                              )}
                            </div>
                          )
                        },
                      )}
                    </>
                  )}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </ToolLayout>
  )
}

export default PemCertificateViewer
