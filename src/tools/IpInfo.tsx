import { invoke } from '@tauri-apps/api/core'
import React, { useEffect, useState } from 'react'
import { Button } from '../components/common'
import { ToolLayout } from '../components/layouts'

interface SourceInfo {
  source: string
  ip?: string | null
  country?: string | null
  city?: string | null
  org?: string | null
  region?: string | null
  timezone?: string | null
}

interface IpLookupResponse {
  infos?: SourceInfo[] | null
  error?: string | null
}

const IpInfo: React.FC = () => {
  const [inputIp, setInputIp] = useState('')
  const [infos, setInfos] = useState<SourceInfo[]>([])
  const [rawJson, setRawJson] = useState('')
  const [showRaw, setShowRaw] = useState(false)

  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  const showToastMessage = (msg: string) => {
    // Simple console log for now - could be enhanced with a proper toast system later
    console.log(msg)
  }

  // 通过后端 Rust 查询IP信息
  const fetchIpInfoBackend = (ip: string) => {
    setError('')

    return invoke('query_ip_info', {
      ip: ip?.trim() || undefined,
    })
      .then((res: unknown) => {
        const typedRes = res as IpLookupResponse
        setInfos(typedRes.infos || [])
        // store raw response for optional display / debugging
        try {
          setRawJson(JSON.stringify(typedRes, null, 2))
        } catch {
          setRawJson('')
        }
        if (typedRes.error) {
          setError(typedRes.error)
        }
      })
      .catch((err) => {
        const message = err instanceof Error ? err.message : String(err)
        console.error('query_ip_info failed:', err)
        setError('查询出错: ' + message)
        setInfos([])
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

  const copyRawJson = () => {
    if (!rawJson) return
    navigator.clipboard
      .writeText(rawJson)
      .then(() => {
        showToastMessage('已复制完整响应')
      })
      .catch(() => {
        showToastMessage('复制失败')
      })
  }

  // IP地址格式校验（支持 IPv4 和 IPv6 简单校验）
  const validateIp = (ip: string) => {
    const ipv4 =
      /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?\d?\d)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?\d?\d)$/
    const ipv6 = new RegExp(
      '^(' +
        '([0-9A-Fa-f]{1,4}:){7}[0-9A-Fa-f]{1,4}|' +
        '([0-9A-Fa-f]{1,4}:){1,7}:|' +
        '([0-9A-Fa-f]{1,4}:){1,6}:[0-9A-Fa-f]{1,4}|' +
        '([0-9A-Fa-f]{1,4}:){1,5}(:[0-9A-Fa-f]{1,4}){1,2}|' +
        '([0-9A-Fa-f]{1,4}:){1,4}(:[0-9A-Fa-f]{1,4}){1,3}|' +
        '([0-9A-Fa-f]{1,4}:){1,3}(:[0-9A-Fa-f]{1,4}){1,4}|' +
        '([0-9A-Fa-f]{1,4}:){1,2}(:[0-9A-Fa-f]{1,4}){1,5}|' +
        '[0-9A-Fa-f]{1,4}:((:[0-9A-Fa-f]{1,4}){1,6})|' +
        ':((:[0-9A-Fa-f]{1,4}){1,7}|:)' +
        ')$',
    )
    return ipv4.test(ip) || ipv6.test(ip)
  }

  // 查询按钮点击处理
  const handleSearch = () => {
    setLoading(true)
    setError('')

    // 如果输入为空，分别从两个网站获取当前IP
    if (inputIp.trim().length > 0) {
      // 校验IP地址格式
      if (!validateIp(inputIp)) {
        setError('请输入有效的IP地址')
        setLoading(false)
        return
      }
    }

    // 查询输入的IP（使用后端）
    fetchIpInfoBackend(inputIp).finally(() => {
      setLoading(false)
    })
  }

  // 组件加载时的初始化逻辑
  useEffect(() => {
    handleSearch()
  }, [])

  return (
    <ToolLayout
      title='IP 地址信息查询'
      subtitle='查询IP地址的地理位置、运营商和相关信息'>
      <>
        {/* IP输入和查询区域 */}
        <div className='mb-6 space-y-4'>
          <div className='flex flex-col sm:flex-row sm:items-center gap-3'>
            <input
              type='text'
              value={inputIp}
              onChange={(e) => setInputIp(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  handleSearch()
                }
              }}
              placeholder='请输入IP地址，留空则查询当前IP'
              className='flex-1 px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
            />
            <div className='flex gap-2'>
              <Button
                variant='primary'
                size='sm'
                onClick={handleSearch}
                disabled={loading}>
                {loading ? '查询中...' : '查询'}
              </Button>
              <Button
                variant='secondary'
                size='sm'
                onClick={() => setShowRaw((s) => !s)}>
                {showRaw ? '隐藏原始' : '显示原始 JSON'}
              </Button>
            </div>
          </div>
        </div>

        {error && (
          <div className='mb-4 p-3 bg-red-100 text-red-700 rounded-md dark:bg-red-900 dark:text-red-100'>
            {error}
          </div>
        )}

        <div className='flex flex-col md:flex-row gap-4'>
          {loading && infos.length === 0 && (
            <>
              <div className='flex-1 p-6 bg-gray-100 dark:bg-gray-800 rounded-lg animate-pulse h-48' />
              <div className='flex-1 p-6 bg-gray-100 dark:bg-gray-800 rounded-lg animate-pulse h-48' />
            </>
          )}

          {infos.length > 0 &&
            infos.map((info) => (
              <div
                key={info.source}
                className='bg-white dark:bg-gray-800 rounded-lg shadow-md p-6 flex-1'>
                <h3 className='text-lg font-semibold mb-4 text-gray-800 dark:text-white'>
                  {`IP 信息 (${info.source})`}
                </h3>
                <div className='flex flex-col gap-2'>
                  <div className='flex items-center'>
                    <p className='text-sm text-gray-600 dark:text-gray-400 w-16'>
                      IP 地址:
                    </p>
                    <div className='ml-2 flex items-center gap-2'>
                      <p className='font-medium text-gray-900 dark:text-white'>
                        {info.ip || 'N/A'}
                      </p>
                      {info.ip && (
                        <button
                          type='button'
                          title='复制 IP'
                          onClick={() => copyToClipboard(info.ip as string)}
                          className='p-1 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-white rounded hover:bg-gray-200 dark:hover:bg-gray-600 flex items-center justify-center'>
                          <svg
                            xmlns='http://www.w3.org/2000/svg'
                            className='w-4 h-4'
                            fill='none'
                            viewBox='0 0 24 24'
                            stroke='currentColor'>
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

                  <div className='flex items-center'>
                    <p className='text-sm text-gray-600 dark:text-gray-400 w-16'>
                      国家:
                    </p>
                    <p className='font-medium text-gray-900 dark:text-white ml-2'>
                      {info.country || 'N/A'}
                    </p>
                  </div>

                  <div className='flex items-center'>
                    <p className='text-sm text-gray-600 dark:text-gray-400 w-16'>
                      城市:
                    </p>
                    <p className='font-medium text-gray-900 dark:text-white ml-2'>
                      {info.city || 'N/A'}
                    </p>
                  </div>

                  <div className='flex items-center'>
                    <p className='text-sm text-gray-600 dark:text-gray-400 w-16'>
                      地区:
                    </p>
                    <p className='font-medium text-gray-900 dark:text-white ml-2'>
                      {info.region || 'N/A'}
                    </p>
                  </div>

                  <div className='flex items-center'>
                    <p className='text-sm text-gray-600 dark:text-gray-400 w-16'>
                      运营商:
                    </p>
                    <p className='font-medium text-gray-900 dark:text-white ml-2'>
                      {info.org || 'N/A'}
                    </p>
                  </div>

                  <div className='flex items-center'>
                    <p className='text-sm text-gray-600 dark:text-gray-400 w-16'>
                      时区:
                    </p>
                    <p className='font-medium text-gray-900 dark:text-white ml-2'>
                      {info.timezone || 'N/A'}
                    </p>
                  </div>
                </div>
              </div>
            ))}
        </div>

        {showRaw && rawJson && (
          <div className='mt-4'>
            <div className='flex items-center justify-between'>
              <h4 className='text-sm font-medium mb-2 text-gray-700 dark:text-white'>
                原始响应
              </h4>
              <div className='flex items-center gap-2'>
                <button
                  type='button'
                  onClick={copyRawJson}
                  className='px-2 py-1 text-sm bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-white rounded-md hover:bg-gray-200 dark:hover:bg-gray-600'>
                  复制全部
                </button>
              </div>
            </div>
            <div
              className='bg-gray-50 dark:bg-gray-900 p-2 rounded text-xs overflow-auto font-mono'
              style={{ maxHeight: 300 }}>
              {rawJson.split('\n').map((line, idx) => (
                <div key={idx} className='flex'>
                  <div className='w-12 text-right pr-3 text-gray-500 dark:text-gray-400 select-none'>
                    {idx + 1}
                  </div>
                  <div className='flex-1 text-gray-900 dark:text-white whitespace-pre-wrap'>
                    {line === '' ? ' ' : line}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </>
    </ToolLayout>
  )
}

export default IpInfo
