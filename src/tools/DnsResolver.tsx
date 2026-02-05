import React, { useCallback, useEffect, useState } from 'react'
import { Button } from '../components/common'
import { ToolLayout } from '../components/layouts'
import {
  batchReverseDnsLookup,
  getDnsServers,
  lookupDns,
  reverseDnsLookup,
  type BatchReverseDnsResponse,
  type DnsLookupResponse,
  type ReverseDnsResponse,
} from '../utils/api'

const DnsResolver: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'forward' | 'reverse'>('forward')
  const [dnsServers, setDnsServers] = useState<Record<string, string>>({})
  const [selectedDnsServer, setSelectedDnsServer] = useState<string>('')
  const [customDnsServer, setCustomDnsServer] = useState<string>('')
  const [recordType, setRecordType] = useState<
    'A' | 'AAAA' | 'MX' | 'TXT' | 'CNAME' | 'NS' | 'SOA' | 'ALL'
  >('ALL')
  const [domain, setDomain] = useState('')
  const [dnsResponse, setDnsResponse] = useState<DnsLookupResponse | null>(null)
  const [dnsError, setDnsError] = useState('')
  const [reverseIp, setReverseIp] = useState('')
  const [reverseResponse, setReverseResponse] =
    useState<ReverseDnsResponse | null>(null)
  const [reverseError, setReverseError] = useState('')
  const [batchIps, setBatchIps] = useState('')
  const [batchResponse, setBatchResponse] =
    useState<BatchReverseDnsResponse | null>(null)
  const [isLoading, setIsLoading] = useState(false)

  // 加载 DNS 服务器列表
  useEffect(() => {
    const loadDnsServers = async () => {
      try {
        const servers = await getDnsServers()
        setDnsServers(servers)
        // 默认选择第一个
        const firstServer = Object.values(servers)[0]
        setSelectedDnsServer(firstServer || '')
      } catch (err) {
        console.error('加载 DNS 服务器失败:', err)
      }
    }
    loadDnsServers()
  }, [])

  // 处理正向 DNS 解析
  const handleDnsLookup = useCallback(async () => {
    if (!domain.trim()) {
      setDnsResponse(null)
      setDnsError('请输入域名')
      return
    }

    setIsLoading(true)
    setDnsError('')

    try {
      const activeDnsServer = selectedDnsServer || customDnsServer
      const response = await lookupDns(
        domain,
        activeDnsServer || undefined,
        recordType,
      )
      setDnsResponse(response)
    } catch (err) {
      setDnsError('查询失败: ' + (err as Error).message)
      setDnsResponse(null)
    } finally {
      setIsLoading(false)
    }
  }, [domain, selectedDnsServer, customDnsServer, recordType])

  // 处理反向 DNS 解析
  const handleReverseDns = useCallback(async () => {
    if (!reverseIp.trim()) {
      setReverseResponse(null)
      setReverseError('请输入 IP 地址')
      return
    }

    setIsLoading(true)
    setReverseError('')

    try {
      const response = await reverseDnsLookup(reverseIp)
      setReverseResponse(response)
    } catch (err) {
      setReverseError('查询失败: ' + (err as Error).message)
      setReverseResponse(null)
    } finally {
      setIsLoading(false)
    }
  }, [reverseIp])

  // 处理批量反向 DNS 解析
  const handleBatchReverseDns = useCallback(async () => {
    if (!batchIps.trim()) {
      setBatchResponse(null)
      setReverseError('请输入 IP 地址列表')
      return
    }

    const ips = batchIps
      .split('\n')
      .map((ip: string) => ip.trim())
      .filter((ip: string) => ip.length > 0)

    setIsLoading(true)
    setReverseError('')

    try {
      const response = await batchReverseDnsLookup(ips)
      setBatchResponse(response)
    } catch (err) {
      setReverseError('批量查询失败: ' + (err as Error).message)
      setBatchResponse(null)
    } finally {
      setIsLoading(false)
    }
  }, [batchIps])

  // 加载示例数据
  const loadExample = (type: 'forward' | 'reverse' | 'batch') => {
    if (type === 'forward') {
      setDomain('www.example.com')
      setRecordType('ALL')
    } else if (type === 'reverse') {
      setReverseIp('8.8.8.8')
    } else {
      setBatchIps('8.8.8.8\n114.114.114.114\n1.1.1.1')
    }
  }

  return (
    <ToolLayout
      title='DNS 解析工具'
      subtitle='正向解析和反向 DNS 查询，支持 A 记录和 AAAA 记录'>
      <div className='flex flex-col h-full'>
        {/* 选项卡 */}
        <div className='border-b border-slate-200 dark:border-slate-700 mb-4'>
          <div className='flex space-x-4'>
            <button
              className={`px-4 py-2 font-medium ${
                activeTab === 'forward'
                  ? 'text-primary-600 dark:text-primary-400 border-b-2 border-primary-600 dark:border-primary-400'
                  : 'text-slate-600 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200'
              }`}
              onClick={() => setActiveTab('forward')}>
              正向解析
            </button>
            <button
              className={`px-4 py-2 font-medium ${
                activeTab === 'reverse'
                  ? 'text-primary-600 dark:text-primary-400 border-b-2 border-primary-600 dark:border-primary-400'
                  : 'text-slate-600 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200'
              }`}
              onClick={() => setActiveTab('reverse')}>
              反向解析
            </button>
          </div>
        </div>

        {activeTab === 'forward' ? (
          <div className='flex-1 overflow-auto'>
            <div className='grid grid-cols-1 lg:grid-cols-2 gap-4 h-full'>
              {/* 左侧输入区 */}
              <div className='space-y-4'>
                <div className='bg-slate-50 dark:bg-slate-700/50 rounded-lg p-4 h-full'>
                  <h3 className='text-lg font-semibold text-slate-800 dark:text-slate-200 mb-4'>
                    查询配置
                  </h3>

                  <div className='space-y-4'>
                    <div>
                      <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                        DNS 服务器
                      </label>
                      <select
                        className='w-full p-2 border border-slate-300 rounded-lg shadow-sm focus:ring-primary-500 focus:border-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                        value={selectedDnsServer}
                        onChange={(e) => setSelectedDnsServer(e.target.value)}>
                        <option value=''>选择预设 DNS 服务器</option>
                        {Object.entries(dnsServers).map(([name, server]) => (
                          <option key={server} value={server}>
                            {name} ({server})
                          </option>
                        ))}
                      </select>
                    </div>

                    <div>
                      <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                        或自定义 DNS 服务器
                      </label>
                      <input
                        type='text'
                        className='w-full p-2 border border-slate-300 rounded-lg shadow-sm focus:ring-primary-500 focus:border-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                        placeholder='例如: 8.8.8.8'
                        value={customDnsServer}
                        onChange={(e) => setCustomDnsServer(e.target.value)}
                      />
                    </div>

                    <div>
                      <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                        记录类型
                      </label>
                      <select
                        className='w-full p-2 border border-slate-300 rounded-lg shadow-sm focus:ring-primary-500 focus:border-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                        value={recordType}
                        onChange={(e) => setRecordType(e.target.value as any)}>
                        <option value='ALL'>全部记录</option>
                        <option value='A'>A 记录 (IPv4)</option>
                        <option value='AAAA'>AAAA 记录 (IPv6)</option>
                        <option value='MX'>MX 记录 (邮件交换)</option>
                        <option value='TXT'>TXT 记录 (文本)</option>
                        <option value='CNAME'>CNAME 记录 (别名)</option>
                        <option value='NS'>NS 记录 (域名服务器)</option>
                        <option value='SOA'>SOA 记录 (权威开始)</option>
                      </select>
                    </div>

                    <div>
                      <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                        域名
                      </label>
                      <input
                        type='text'
                        className='w-full p-2 border border-slate-300 rounded-lg shadow-sm focus:ring-primary-500 focus:border-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                        placeholder='例如: www.example.com'
                        value={domain}
                        onChange={(e) => setDomain(e.target.value)}
                      />
                    </div>

                    <div className='flex gap-2'>
                      <Button onClick={handleDnsLookup} disabled={isLoading}>
                        {isLoading ? '查询中...' : '查询'}
                      </Button>
                      <Button
                        variant='secondary'
                        onClick={() => loadExample('forward')}>
                        示例
                      </Button>
                      <Button
                        variant='secondary'
                        onClick={() => {
                          setDomain('')
                          setDnsResponse(null)
                          setDnsError('')
                        }}>
                        清空
                      </Button>
                    </div>
                  </div>
                </div>
              </div>

              {/* 右侧结果区 */}
              <div className='space-y-4'>
                <div className='bg-slate-50 dark:bg-slate-700/50 rounded-lg p-4 h-full'>
                  <h3 className='text-lg font-semibold text-slate-800 dark:text-slate-200 mb-4'>
                    查询结果
                  </h3>

                  <div className='flex-1 overflow-auto'>
                    {dnsError && (
                      <div className='p-4 mb-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg'>
                        <p className='text-red-700 dark:text-red-400'>
                          {dnsError}
                        </p>
                      </div>
                    )}

                    {dnsResponse && (
                      <div className='space-y-4'>
                        <div>
                          <p className='text-sm text-slate-600 dark:text-slate-400 mb-3'>
                            域名: {dnsResponse.domain}
                          </p>

                          {dnsResponse.records.length > 0 ? (
                            <div className='space-y-2'>
                              <h4 className='font-medium text-slate-700 dark:text-slate-300'>
                                DNS 记录:
                              </h4>
                              <div className='overflow-x-auto'>
                                <table className='min-w-full divide-y divide-slate-200 dark:divide-slate-700'>
                                  <thead className='bg-slate-100 dark:bg-slate-800'>
                                    <tr>
                                      <th className='px-4 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-400 uppercase tracking-wider'>
                                        类型
                                      </th>
                                      <th className='px-4 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-400 uppercase tracking-wider'>
                                        值
                                      </th>
                                    </tr>
                                  </thead>
                                  <tbody className='bg-white dark:bg-slate-700 divide-y divide-slate-200 dark:divide-slate-600'>
                                    {dnsResponse.records.map(
                                      (record: any, idx: number) => (
                                        <tr key={idx}>
                                          <td className='px-4 py-2 whitespace-nowrap text-sm font-medium text-slate-900 dark:text-slate-100'>
                                            {record.recordType}
                                          </td>
                                          <td className='px-4 py-2 text-sm text-slate-500 dark:text-slate-400 font-mono'>
                                            {record.value}
                                          </td>
                                        </tr>
                                      ),
                                    )}
                                  </tbody>
                                </table>
                              </div>
                            </div>
                          ) : (
                            <p className='text-slate-500 dark:text-slate-400'>
                              未找到记录
                            </p>
                          )}
                        </div>

                        {dnsResponse.ipInfo && (
                          <div className='mt-4 pt-4 border-t border-slate-200 dark:border-slate-600'>
                            <h4 className='font-medium text-slate-700 dark:text-slate-300 mb-3'>
                              IP 地理位置信息
                            </h4>
                            <div className='grid grid-cols-2 gap-4'>
                              <div>
                                <p className='text-sm text-slate-600 dark:text-slate-400'>
                                  IP 地址
                                </p>
                                <p className='font-mono text-slate-800 dark:text-slate-200'>
                                  {dnsResponse.ipInfo.ip}
                                </p>
                              </div>
                              <div>
                                <p className='text-sm text-slate-600 dark:text-slate-400'>
                                  国家
                                </p>
                                <p className='text-slate-800 dark:text-slate-200'>
                                  {dnsResponse.ipInfo.country || '-'}
                                </p>
                              </div>
                              <div>
                                <p className='text-sm text-slate-600 dark:text-slate-400'>
                                  城市
                                </p>
                                <p className='text-slate-800 dark:text-slate-200'>
                                  {dnsResponse.ipInfo.city || '-'}
                                </p>
                              </div>
                              <div>
                                <p className='text-sm text-slate-600 dark:text-slate-400'>
                                  运营商
                                </p>
                                <p className='text-slate-800 dark:text-slate-200'>
                                  {dnsResponse.ipInfo.org || '-'}
                                </p>
                              </div>
                              <div>
                                <p className='text-sm text-slate-600 dark:text-slate-400'>
                                  地区
                                </p>
                                <p className='text-slate-800 dark:text-slate-200'>
                                  {dnsResponse.ipInfo.region || '-'}
                                </p>
                              </div>
                              <div>
                                <p className='text-sm text-slate-600 dark:text-slate-400'>
                                  时区
                                </p>
                                <p className='text-slate-800 dark:text-slate-200'>
                                  {dnsResponse.ipInfo.timezone || '-'}
                                </p>
                              </div>
                            </div>
                            <p className='text-xs text-slate-500 dark:text-slate-400 mt-2'>
                              数据来源: {dnsResponse.ipInfo.source}
                            </p>
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              </div>
            </div>
          </div>
        ) : (
          <div className='flex-1 overflow-auto'>
            <div className='grid grid-cols-1 lg:grid-cols-2 gap-4 h-full'>
              {/* 左侧输入区 */}
              <div className='space-y-4'>
                <div className='bg-slate-50 dark:bg-slate-700/50 rounded-lg p-4 h-full'>
                  <h3 className='text-lg font-semibold text-slate-800 dark:text-slate-200 mb-4'>
                    反向解析配置
                  </h3>

                  <div className='space-y-4'>
                    <div>
                      <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                        单个 IP 反向解析
                      </label>
                      <input
                        type='text'
                        className='w-full p-2 border border-slate-300 rounded-lg shadow-sm focus:ring-primary-500 focus:border-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                        placeholder='例如: 8.8.8.8'
                        value={reverseIp}
                        onChange={(e) => setReverseIp(e.target.value)}
                      />
                    </div>

                    <div className='flex gap-2'>
                      <Button onClick={handleReverseDns} disabled={isLoading}>
                        {isLoading ? '查询中...' : '查询'}
                      </Button>
                      <Button
                        variant='secondary'
                        onClick={() => loadExample('reverse')}>
                        示例
                      </Button>
                      <Button
                        variant='secondary'
                        onClick={() => {
                          setReverseIp('')
                          setReverseResponse(null)
                          setReverseError('')
                        }}>
                        清空
                      </Button>
                    </div>

                    <div className='border-t border-slate-200 dark:border-slate-600 pt-4'>
                      <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                        批量反向解析 (每行一个 IP)
                      </label>
                      <textarea
                        className='w-full p-2 border border-slate-300 rounded-lg shadow-sm focus:ring-primary-500 focus:border-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                        rows={6}
                        placeholder='输入多个 IP 地址，每行一个'
                        value={batchIps}
                        onChange={(e) => setBatchIps(e.target.value)}
                      />
                    </div>

                    <div className='flex gap-2'>
                      <Button
                        onClick={handleBatchReverseDns}
                        disabled={isLoading}>
                        {isLoading ? '查询中...' : '批量查询'}
                      </Button>
                      <Button
                        variant='secondary'
                        onClick={() => loadExample('batch')}>
                        示例
                      </Button>
                      <Button
                        variant='secondary'
                        onClick={() => {
                          setBatchIps('')
                          setBatchResponse(null)
                        }}>
                        清空
                      </Button>
                    </div>
                  </div>
                </div>
              </div>

              {/* 右侧结果区 */}
              <div className='space-y-4'>
                <div className='bg-slate-50 dark:bg-slate-700/50 rounded-lg p-4 h-full'>
                  <h3 className='text-lg font-semibold text-slate-800 dark:text-slate-200 mb-4'>
                    查询结果
                  </h3>

                  <div className='flex-1 overflow-auto'>
                    {reverseError && (
                      <div className='p-4 mb-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg'>
                        <p className='text-red-700 dark:text-red-400'>
                          {reverseError}
                        </p>
                      </div>
                    )}

                    {reverseResponse && (
                      <div className='mb-4'>
                        <h4 className='font-medium text-slate-700 dark:text-slate-300 mb-2'>
                          单个 IP 查询结果
                        </h4>
                        <p className='text-sm text-slate-600 dark:text-slate-400 mb-2'>
                          IP 地址: {reverseResponse.ip}
                        </p>
                        {reverseResponse.domains.length > 0 ? (
                          <div>
                            <p className='text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                              对应域名:
                            </p>
                            <ul className='list-disc list-inside space-y-1'>
                              {reverseResponse.domains.map(
                                (domain: string, idx: number) => (
                                  <li
                                    key={idx}
                                    className='text-slate-800 dark:text-slate-200 font-mono'>
                                    {domain}
                                  </li>
                                ),
                              )}
                            </ul>
                          </div>
                        ) : (
                          <p className='text-slate-500 dark:text-slate-400'>
                            未找到对应的域名记录
                          </p>
                        )}
                      </div>
                    )}

                    {batchResponse && (
                      <div>
                        <h4 className='font-medium text-slate-700 dark:text-slate-300 mb-2'>
                          批量查询结果
                        </h4>
                        {batchResponse.results.length > 0 ? (
                          <div className='space-y-3'>
                            {batchResponse.results.map(
                              (result: any, idx: number) => (
                                <div
                                  key={idx}
                                  className='border border-slate-200 dark:border-slate-700 rounded-lg p-3'>
                                  <p className='font-medium text-slate-700 dark:text-slate-300 mb-2'>
                                    {result.ip}
                                  </p>
                                  {result.domains.length > 0 ? (
                                    <ul className='list-disc list-inside space-y-1'>
                                      {result.domains.map(
                                        (domain: string, domainIdx: number) => (
                                          <li
                                            key={domainIdx}
                                            className='text-sm text-slate-600 dark:text-slate-400 font-mono'>
                                            {domain}
                                          </li>
                                        ),
                                      )}
                                    </ul>
                                  ) : (
                                    <p className='text-sm text-slate-500 dark:text-slate-400'>
                                      未找到对应的域名记录
                                    </p>
                                  )}
                                </div>
                              ),
                            )}
                          </div>
                        ) : (
                          <p className='text-slate-500 dark:text-slate-400'>
                            未找到查询结果
                          </p>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </ToolLayout>
  )
}

export default DnsResolver
