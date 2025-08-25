import React, { useEffect, useState } from 'react'
import { ToolLayout } from '../components/layouts'

const SubnetCalculator: React.FC = () => {
  const [input, setInput] = useState('')
  const [ipVersion, setIpVersion] = useState<'auto' | 'ipv4' | 'ipv6'>('auto')
  const [result, setResult] = useState({
    version: '',
    cidr: '',
    mask: '',
    network: '',
    broadcast: '',
    firstIp: '',
    lastIp: '',
    totalIps: '',
    usableIps: '',
    // IPv6 specific
    prefix: '',
    networkPortion: '',
    hostPortion: '',
  })
  const [error, setError] = useState('')
  const [debouncedInput, setDebouncedInput] = useState('')

  // IPv4 functions
  const cidrToMask = (cidr: number): string => {
    return Array(4)
      .fill(0)
      .map((_, i) => {
        const bits = Math.min(8, Math.max(0, cidr - i * 8))
        return (0xff << (8 - bits)) & 0xff
      })
      .join('.')
  }

  const maskToCidr = (mask: string): number => {
    const parts = mask.split('.').map((p) => parseInt(p))
    return (
      parts
        .map((p) => (p >>> 0).toString(2).padStart(8, '0'))
        .join('')
        .split('1').length - 1
    )
  }

  const isValidIpv4 = (ip: string): boolean => {
    const ipPattern =
      /^((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/
    return ipPattern.test(ip)
  }

  const isValidCidrV4 = (cidr: number): boolean => {
    return Number.isInteger(cidr) && cidr >= 0 && cidr <= 32
  }

  // IPv6 functions
  const isValidIpv6 = (ip: string): boolean => {
    // 更全面的IPv6验证函数
    try {
      // 基本格式检查
      if (!ip || typeof ip !== 'string') return false
      
      // 移除开头和结尾的空格
      ip = ip.trim()
      
      // 检查是否包含无效字符（允许十六进制字符和冒号）
      if (!/^[0-9a-fA-F:]+$/.test(ip)) return false
      
      // 处理特殊情况 :: (全零地址)
      if (ip === '::') return true
      
      // 检查双冒号出现次数（最多一次）
      const doubleColonCount = (ip.match(/::/g) || []).length
      if (doubleColonCount > 1) return false
      
      // 检查是否以单个冒号开始或结束（双冒号是允许的）
      if ((ip.startsWith(':') && !ip.startsWith('::')) || 
          (ip.endsWith(':') && !ip.endsWith('::'))) return false
      
      let parts: string[]
      
      if (ip.includes('::')) {
        // 处理压缩格式 (::表示一个或多个零组)
        const segments = ip.split('::')
        if (segments.length !== 2) return false
        
        const leftParts = segments[0] ? segments[0].split(':').filter(part => part !== '') : []
        const rightParts = segments[1] ? segments[1].split(':').filter(part => part !== '') : []
        
        // 检查总部分数是否合理 (左右两部分加起来不能超过8组)
        if (leftParts.length + rightParts.length >= 8) return false
        
        // 验证左右两侧的每个部分
        parts = [...leftParts, ...rightParts]
      } else {
        // 处理完整格式（必须有8组）
        parts = ip.split(':')
        if (parts.length !== 8) return false
      }
      
      // 验证每个部分
      for (const part of parts) {
        // 每部分不能为空（在完整格式中）
        if (!ip.includes('::') && part === '') return false
        // 每部分最多4个字符
        if (part.length > 4) return false
        // 只允许十六进制字符
        if (part && !/^[0-9a-fA-F]+$/i.test(part)) return false
      }
      
      return true
    } catch (error) {
      return false
    }
  }

  const isValidCidrV6 = (cidr: number): boolean => {
    return Number.isInteger(cidr) && cidr >= 0 && cidr <= 128
  }

  const expandIpv6 = (ip: string): string => {
    // 扩展IPv6地址到完整格式
    if (ip.includes('::')) {
      const parts = ip.split('::')
      const leftParts = parts[0] ? parts[0].split(':') : []
      const rightParts = parts[1] ? parts[1].split(':') : []
      const missingParts = 8 - leftParts.length - rightParts.length
      const middleParts = Array(missingParts).fill('0000')
      return [...leftParts, ...middleParts, ...rightParts]
        .map((part) => part.padStart(4, '0'))
        .join(':')
    }
    return ip
      .split(':')
      .map((part) => part.padStart(4, '0'))
      .join(':')
  }

  const calculateIpv6Network = (ip: string, prefixLength: number): string => {
    const expanded = expandIpv6(ip)
    const parts = expanded.split(':')

    // 将IPv6地址转换为128位二进制
    const binaryIP = parts
      .map((part) => parseInt(part, 16).toString(2).padStart(16, '0'))
      .join('')

    // 应用前缀长度
    const networkBinary =
      binaryIP.substring(0, prefixLength) + '0'.repeat(128 - prefixLength)

    // 转换回IPv6格式
    const networkParts = []
    for (let i = 0; i < 8; i++) {
      const part = networkBinary.substring(i * 16, (i + 1) * 16)
      networkParts.push(parseInt(part, 2).toString(16).padStart(4, '0'))
    }

    return networkParts.join(':')
  }

  const detectIpVersion = (input: string): 'ipv4' | 'ipv6' | 'unknown' => {
    if (input.includes('.')) return 'ipv4'
    if (input.includes(':')) return 'ipv6'
    return 'unknown'
  }

  // IPv4 计算
  const calculateIpv4 = (ip: string, cidrInput: string) => {
    let cidr: number
    let mask: string

    if (cidrInput.includes('.')) {
      // 子网掩码格式
      if (!isValidIpv4(cidrInput)) {
        throw new Error('无效的子网掩码')
      }
      mask = cidrInput
      cidr = maskToCidr(mask)
    } else {
      // CIDR格式
      cidr = parseInt(cidrInput)
      if (!isValidCidrV4(cidr)) {
        throw new Error('无效的CIDR值 (IPv4: 0-32)')
      }
      mask = cidrToMask(cidr)
    }

    // 计算网络信息
    const ipParts = ip.split('.').map(Number)
    const maskParts = mask.split('.').map(Number)
    const network = ipParts.map((part, i) => part & maskParts[i]).join('.')
    const networkParts = network.split('.').map(Number)
    const broadcast = networkParts
      .map((part, i) => part | (255 - maskParts[i]))
      .join('.')

    const total = Math.pow(2, 32 - cidr)
    const usable = cidr === 32 ? 1 : Math.max(0, total - 2)

    // 计算第一个和最后一个可用IP
    const firstIpParts = [...networkParts]
    if (cidr < 31) {
      firstIpParts[3] += 1
    }
    const firstIp = firstIpParts.join('.')

    const lastIpParts = broadcast.split('.').map(Number)
    if (cidr < 31) {
      lastIpParts[3] -= 1
    }
    const lastIp = lastIpParts.join('.')

    return {
      version: 'IPv4',
      cidr: `/${cidr}`,
      mask,
      network,
      broadcast,
      firstIp: cidr < 31 ? firstIp : 'N/A',
      lastIp: cidr < 31 ? lastIp : 'N/A',
      totalIps: total.toLocaleString(),
      usableIps: usable.toLocaleString(),
      prefix: '',
      networkPortion: '',
      hostPortion: '',
    }
  }

  // IPv6 计算
  const calculateIpv6 = (ip: string, prefixLength: number) => {
    if (!isValidCidrV6(prefixLength)) {
      throw new Error('无效的前缀长度 (IPv6: 0-128)')
    }

    const expanded = expandIpv6(ip)
    const network = calculateIpv6Network(ip, prefixLength)

    const hostBits = 128 - prefixLength
    const totalAddresses = hostBits <= 53 ? Math.pow(2, hostBits) : Infinity
    const totalString =
      totalAddresses === Infinity
        ? '2^' + hostBits
        : totalAddresses.toLocaleString()

    // 网络部分和主机部分
    const networkPortion = expanded
      .split(':')
      .slice(0, Math.ceil(prefixLength / 16))
      .join(':')
    const hostPortion = expanded
      .split(':')
      .slice(Math.ceil(prefixLength / 16))
      .join(':')

    return {
      version: 'IPv6',
      cidr: `/${prefixLength}`,
      mask: '', // IPv6不使用子网掩码
      network,
      broadcast: '', // IPv6没有广播地址概念
      firstIp: network,
      lastIp: '计算复杂',
      totalIps: totalString,
      usableIps: totalString, // IPv6中通常所有地址都可用
      prefix: `/${prefixLength}`,
      networkPortion,
      hostPortion,
    }
  }

  const handleCalculate = () => {
    try {
      setError('')

      if (!input) {
        throw new Error('请输入IP地址和CIDR或子网掩码')
      }

      // 解析输入
      if (!input.includes('/')) {
        throw new Error('输入格式应为 IP/CIDR 或 IP/子网掩码')
      }

      const [ipPart, suffix] = input.split('/')
      const detectedVersion = detectIpVersion(ipPart)

      if (detectedVersion === 'unknown') {
        throw new Error('无法识别IP地址版本')
      }

      // 根据版本或用户选择进行验证
      if (ipVersion !== 'auto' && ipVersion !== detectedVersion) {
        throw new Error(
          `IP地址版本不匹配：检测到${
            detectedVersion === 'ipv4' ? 'IPv4' : 'IPv6'
          }，但选择了${ipVersion === 'ipv4' ? 'IPv4' : 'IPv6'}`,
        )
      }

      let calculatedResult

      if (detectedVersion === 'ipv4') {
        if (!isValidIpv4(ipPart)) {
          throw new Error('无效的IPv4地址')
        }
        calculatedResult = calculateIpv4(ipPart, suffix)
      } else {
        if (!isValidIpv6(ipPart)) {
          throw new Error('无效的IPv6地址')
        }
        const prefixLength = parseInt(suffix)
        if (isNaN(prefixLength)) {
          throw new Error('IPv6只支持CIDR格式 (例如: /64)')
        }
        calculatedResult = calculateIpv6(ipPart, prefixLength)
      }

      setResult(calculatedResult)
    } catch (err) {
      setError((err as Error).message)
      setResult({
        version: '',
        cidr: '',
        mask: '',
        network: '',
        broadcast: '',
        firstIp: '',
        lastIp: '',
        totalIps: '',
        usableIps: '',
        prefix: '',
        networkPortion: '',
        hostPortion: '',
      })
    }
  }

  // 添加防抖效果
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedInput(input)
    }, 500)
    return () => clearTimeout(timer)
  }, [input])

  // 当防抖输入变化时自动计算
  useEffect(() => {
    if (debouncedInput) {
      handleCalculate()
    }
  }, [debouncedInput])

  return (
    <ToolLayout
      title="子网计算器 (IPv4 & IPv6)"
      subtitle="支持IPv4和IPv6网络地址的子网计算，包括CIDR转换、子网划分和网络信息查询"
    >
      <div className='flex flex-col h-full'>
        <div className='flex-1 overflow-auto p-4'>
          {/* IP版本选择器 */}
          <div className='mb-4'>
            <label className='block text-sm font-medium text-gray-700 dark:text-gray-200 mb-2'>
              IP版本
            </label>
            <div className='flex space-x-4'>
              {['auto', 'ipv4', 'ipv6'].map((version) => (
                <label key={version} className='flex items-center'>
                  <input
                    type='radio'
                    value={version}
                    checked={ipVersion === version}
                    onChange={(e) =>
                      setIpVersion(e.target.value as 'auto' | 'ipv4' | 'ipv6')
                    }
                    className='mr-2'
                  />
                  <span className='text-gray-700 dark:text-gray-200'>
                    {version === 'auto' ? '自动检测' : version.toUpperCase()}
                  </span>
                </label>
              ))}
            </div>
          </div>

          {/* 输入区域 */}
          <div className='flex gap-3 mb-4'>
            <input
              className='flex-1 p-3 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
              placeholder={
                ipVersion === 'ipv6'
                  ? '输入IPv6和前缀 (例如: 2001:db8::/64)'
                  : ipVersion === 'ipv4'
                  ? '输入IPv4和CIDR或子网掩码 (例如: 192.168.1.0/24)'
                  : '输入IP和CIDR (例如: 192.168.1.0/24 或 2001:db8::/64)'
              }
              value={input}
              onChange={(e) => setInput(e.target.value)}
            />
            <button
              className='px-6 py-3 bg-blue-500 text-white rounded-md hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 whitespace-nowrap'
              onClick={handleCalculate}>
              计算
            </button>
          </div>

          {/* 示例提示 */}
          <div className='mb-4 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-md'>
            <h4 className='font-medium text-blue-800 dark:text-blue-200 mb-2'>
              示例格式:
            </h4>
            <div className='text-sm text-blue-700 dark:text-blue-300 space-y-1'>
              <div>
                <strong>IPv4:</strong> 192.168.1.0/24 或 10.0.0.0/255.0.0.0
              </div>
              <div>
                <strong>IPv6:</strong> 2001:db8::/64 或 fe80::/10
              </div>
              <div>
                <strong>简写IPv6:</strong> ::1/128 或 2001:db8::1/64
              </div>
            </div>
          </div>

          {error && (
            <div className='mb-4 p-4 bg-red-100 dark:bg-red-900 rounded-md'>
              <p className='text-red-700 dark:text-red-100'>
                <strong>错误:</strong> {error}
              </p>
            </div>
          )}

          {result.version && (
            <div className='mb-4 p-4 bg-gray-100 dark:bg-gray-700 rounded-md'>
              <h3 className='text-lg font-semibold mb-2 text-gray-800 dark:text-white'>
                计算结果 ({result.version})
              </h3>
              <div className='grid grid-cols-1 md:grid-cols-2 gap-2'>
                {/* 基本信息 */}
                <div className='flex justify-between'>
                  <span className='text-gray-700 dark:text-gray-200'>版本:</span>
                  <span className='font-mono text-gray-900 dark:text-white'>
                    {result.version}
                  </span>
                </div>

                <div className='flex justify-between'>
                  <span className='text-gray-700 dark:text-gray-200'>CIDR:</span>
                  <span className='font-mono text-gray-900 dark:text-white'>
                    {result.cidr}
                  </span>
                </div>

                {/* IPv4特有字段 */}
                {result.version === 'IPv4' && (
                  <>
                    <div className='flex justify-between'>
                      <span className='text-gray-700 dark:text-gray-200'>
                        子网掩码:
                      </span>
                      <span className='font-mono text-gray-900 dark:text-white'>
                        {result.mask}
                      </span>
                    </div>
                    <div className='flex justify-between'>
                      <span className='text-gray-700 dark:text-gray-200'>
                        广播地址:
                      </span>
                      <span className='font-mono text-gray-900 dark:text-white'>
                        {result.broadcast}
                      </span>
                    </div>
                  </>
                )}

                {/* 通用字段 */}
                <div className='flex justify-between'>
                  <span className='text-gray-700 dark:text-gray-200'>
                    网络地址:
                  </span>
                  <span className='font-mono text-gray-900 dark:text-white break-all'>
                    {result.network}
                  </span>
                </div>

                <div className='flex justify-between'>
                  <span className='text-gray-700 dark:text-gray-200'>
                    {result.version === 'IPv4' ? '第一个可用IP:' : '网络前缀:'}
                  </span>
                  <span className='font-mono text-gray-900 dark:text-white break-all'>
                    {result.firstIp}
                  </span>
                </div>

                {result.version === 'IPv4' && (
                  <div className='flex justify-between'>
                    <span className='text-gray-700 dark:text-gray-200'>
                      最后一个可用IP:
                    </span>
                    <span className='font-mono text-gray-900 dark:text-white'>
                      {result.lastIp}
                    </span>
                  </div>
                )}

                <div className='flex justify-between'>
                  <span className='text-gray-700 dark:text-gray-200'>
                    {result.version === 'IPv4' ? '总IP数:' : '总地址数:'}
                  </span>
                  <span className='font-mono text-gray-900 dark:text-white'>
                    {result.totalIps}
                  </span>
                </div>

                <div className='flex justify-between'>
                  <span className='text-gray-700 dark:text-gray-200'>
                    {result.version === 'IPv4' ? '可用IP数:' : '可用地址数:'}
                  </span>
                  <span className='font-mono text-gray-900 dark:text-white'>
                    {result.usableIps}
                  </span>
                </div>

                {/* IPv6特有字段 */}
                {result.version === 'IPv6' && (
                  <>
                    <div className='flex justify-between'>
                      <span className='text-gray-700 dark:text-gray-200'>
                        网络部分:
                      </span>
                      <span className='font-mono text-gray-900 dark:text-white break-all'>
                        {result.networkPortion}
                      </span>
                    </div>
                    <div className='flex justify-between'>
                      <span className='text-gray-700 dark:text-gray-200'>
                        主机部分:
                      </span>
                      <span className='font-mono text-gray-900 dark:text-white break-all'>
                        {result.hostPortion || 'N/A'}
                      </span>
                    </div>
                  </>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </ToolLayout>
  )
}

export default SubnetCalculator
