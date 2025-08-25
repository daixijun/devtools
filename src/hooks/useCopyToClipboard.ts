import { useState, useCallback } from 'react'

interface UseCopyToClipboardReturn {
  copied: boolean
  copy: (text: string) => Promise<boolean>
  copyToClipboard: (text: string) => Promise<boolean>
  error: string | null
}

/**
 * 剪贴板复制功能Hook
 * 统一管理复制到剪贴板功能，提供状态反馈
 */
const useCopyToClipboard = (): UseCopyToClipboardReturn => {
  const [copied, setCopied] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const copy = useCallback(async (text: string): Promise<boolean> => {
    if (!text) {
      setError('复制内容不能为空')
      return false
    }

    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setError(null)
      
      // 2秒后重置复制状态
      setTimeout(() => setCopied(false), 2000)
      
      return true
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : '复制失败'
      setError(errorMessage)
      setCopied(false)
      return false
    }
  }, [])

  // 保持向后兼容
  const copyToClipboard = copy

  return {
    copied,
    copy,
    copyToClipboard,
    error
  }
}

export default useCopyToClipboard