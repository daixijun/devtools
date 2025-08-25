/**
 * 字符串处理工具函数
 */
export const stringUtils = {
  /**
   * 格式化日期为本地字符串
   */
  formatDate: (dateStr: string): string => {
    try {
      return new Date(dateStr).toLocaleString('zh-CN')
    } catch {
      return dateStr
    }
  },

  /**
   * 从Distinguished Name中提取CN（通用名称）
   */
  extractCN: (dn: string): string => {
    const parts = dn.split(',')
    for (const part of parts) {
      const trimmed = part.trim()
      if (trimmed.startsWith('CN=') || trimmed.startsWith('cn=')) {
        return trimmed.substring(3)
      }
    }
    return dn
  },

  /**
   * 截断长文本并添加省略号
   */
  truncate: (text: string, maxLength: number): string => {
    if (text.length <= maxLength) return text
    return text.substring(0, maxLength - 3) + '...'
  },

  /**
   * 清理并格式化文本
   */
  cleanText: (text: string): string => {
    return text.trim().replace(/\s+/g, ' ')
  },

  /**
   * 首字母大写
   */
  capitalize: (text: string): string => {
    if (!text) return text
    return text.charAt(0).toUpperCase() + text.slice(1).toLowerCase()
  },

  /**
   * 驼峰转横线分隔
   */
  camelToKebab: (text: string): string => {
    return text.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase()
  },

  /**
   * 横线转驼峰
   */
  kebabToCamel: (text: string): string => {
    return text.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase())
  }
}

/**
 * 数据验证工具函数
 */
export const validators = {
  /**
   * 验证是否为有效的JSON格式
   */
  isValidJson: (str: string): boolean => {
    if (!str.trim()) return false
    try {
      JSON.parse(str)
      return true
    } catch {
      return false
    }
  },

  /**
   * 验证是否为有效的IP地址
   */
  isValidIp: (ip: string): boolean => {
    const ipv4Regex = /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/
    const ipv6Regex = /^(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$|^::1$|^::$/
    return ipv4Regex.test(ip) || ipv6Regex.test(ip)
  },

  /**
   * 验证是否为有效的域名
   */
  isValidDomain: (domain: string): boolean => {
    const domainRegex = /^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/i
    return domainRegex.test(domain.trim())
  },

  /**
   * 验证是否为有效的邮箱地址
   */
  isValidEmail: (email: string): boolean => {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
    return emailRegex.test(email.trim())
  },

  /**
   * 验证是否为有效的Base64字符串
   */
  isValidBase64: (str: string): boolean => {
    if (!str.trim()) return false
    try {
      return btoa(atob(str)) === str
    } catch {
      return false
    }
  },

  /**
   * 验证是否为有效的URL
   */
  isValidUrl: (url: string): boolean => {
    try {
      new URL(url)
      return true
    } catch {
      return false
    }
  }
}

/**
 * 格式化工具函数
 */
export const formatters = {
  /**
   * 格式化文件大小
   */
  formatFileSize: (bytes: number): string => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  },

  /**
   * 格式化数字为千分位格式
   */
  formatNumber: (num: number): string => {
    return num.toLocaleString('zh-CN')
  },

  /**
   * 格式化时间戳
   */
  formatTimestamp: (timestamp: number): string => {
    const date = new Date(timestamp * 1000)
    return date.toLocaleString('zh-CN')
  },

  /**
   * 格式化持续时间（毫秒转为可读格式）
   */
  formatDuration: (ms: number): string => {
    if (ms < 1000) return `${ms}ms`
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
    if (ms < 3600000) return `${(ms / 60000).toFixed(1)}min`
    return `${(ms / 3600000).toFixed(1)}h`
  },

  /**
   * 格式化百分比
   */
  formatPercentage: (value: number, total: number): string => {
    if (total === 0) return '0%'
    return `${((value / total) * 100).toFixed(1)}%`
  }
}

/**
 * 数据转换工具函数
 */
export const converters = {
  /**
   * 十六进制转RGB
   */
  hexToRgb: (hex: string): { r: number; g: number; b: number } | null => {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex)
    return result ? {
      r: parseInt(result[1], 16),
      g: parseInt(result[2], 16),
      b: parseInt(result[3], 16)
    } : null
  },

  /**
   * RGB转十六进制
   */
  rgbToHex: (r: number, g: number, b: number): string => {
    return "#" + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)
  },

  /**
   * 将字节数组转为十六进制字符串
   */
  bytesToHex: (bytes: Uint8Array): string => {
    return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('')
  },

  /**
   * 十六进制字符串转字节数组
   */
  hexToBytes: (hex: string): Uint8Array => {
    const bytes = new Uint8Array(hex.length / 2)
    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = parseInt(hex.substr(i * 2, 2), 16)
    }
    return bytes
  }
}

/**
 * 错误处理工具函数
 */
export const errorUtils = {
  /**
   * 格式化错误消息
   */
  formatError: (err: unknown, prefix = '操作失败'): string => {
    const message = err instanceof Error ? err.message : String(err)
    return `${prefix}: ${message}`
  },

  /**
   * 安全地解析JSON，返回错误信息
   */
  safeJsonParse: <T>(jsonString: string): { data: T | null; error: string | null } => {
    try {
      const data = JSON.parse(jsonString) as T
      return { data, error: null }
    } catch (err) {
      return { 
        data: null, 
        error: err instanceof Error ? err.message : 'JSON解析失败' 
      }
    }
  },

  /**
   * 重试执行异步操作
   */
  retry: async <T>(
    fn: () => Promise<T>, 
    attempts: number = 3, 
    delay: number = 1000
  ): Promise<T> => {
    for (let i = 0; i < attempts; i++) {
      try {
        return await fn()
      } catch (error) {
        if (i === attempts - 1) throw error
        await new Promise(resolve => setTimeout(resolve, delay))
      }
    }
    throw new Error('重试失败')
  }
}

/**
 * 性能工具函数
 */
export const perfUtils = {
  /**
   * 创建防抖函数
   */
  debounce: <T extends (...args: any[]) => void>(
    func: T,
    wait: number
  ): ((...args: Parameters<T>) => void) => {
    let timeout: NodeJS.Timeout | null = null
    return (...args: Parameters<T>) => {
      if (timeout) clearTimeout(timeout)
      timeout = setTimeout(() => func(...args), wait)
    }
  },

  /**
   * 创建节流函数
   */
  throttle: <T extends (...args: any[]) => void>(
    func: T,
    limit: number
  ): ((...args: Parameters<T>) => void) => {
    let inThrottle = false
    return (...args: Parameters<T>) => {
      if (!inThrottle) {
        func(...args)
        inThrottle = true
        setTimeout(() => inThrottle = false, limit)
      }
    }
  },

  /**
   * 测量函数执行时间
   */
  measureTime: async <T>(
    fn: () => Promise<T>
  ): Promise<{ result: T; duration: number }> => {
    const start = performance.now()
    const result = await fn()
    const duration = performance.now() - start
    return { result, duration }
  }
}