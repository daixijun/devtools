import { useState } from 'react'
import { errorUtils } from '../utils'

/**
 * useToolAsyncProcessor - 异步数据处理 Hook
 *
 * @description
 * 为需要异步操作的工具(如网络请求)提供统一的数据处理模式
 *
 * @features
 * - 加载状态管理
 * - 错误处理
 * - 防止重复请求
 * - TypeScript 类型安全
 *
 * @example
 * ```tsx
 * const { input, setInput, output, loading, error, process } =
 *   useToolAsyncProcessor(async (url) => {
 *     const response = await fetch(url)
 *     return response.json()
 *   })
 * ```
 *
 * @param asyncProcessor - 异步处理函数
 * @returns 工具状态和处理函数
 */
export interface UseToolAsyncProcessorReturn<T> {
  /** 输入值 */
  input: string
  /** 设置输入值 */
  setInput: (value: string) => void
  /** 处理后的输出 */
  output: T | null
  /** 是否正在加载 */
  loading: boolean
  /** 错误消息 */
  error: string
  /** 执行处理 */
  process: () => Promise<void>
  /** 重置状态 */
  reset: () => void
}

export function useToolAsyncProcessor<T>(
  asyncProcessor: (input: string) => Promise<T>
): UseToolAsyncProcessorReturn<T> {
  const [input, setInput] = useState('')
  const [output, setOutput] = useState<T | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  /**
   * 执行异步处理
   */
  const process = async () => {
    // 验证输入
    if (!input.trim()) {
      setError('请输入内容')
      return
    }

    // 防止重复请求
    if (loading) {
      return
    }

    setLoading(true)
    setError('')

    try {
      const result = await asyncProcessor(input)
      setOutput(result)
      setError('')
    } catch (err) {
      setOutput(null)
      setError(errorUtils.formatError(err))
    } finally {
      setLoading(false)
    }
  }

  /**
   * 重置状态
   */
  const reset = () => {
    setInput('')
    setOutput(null)
    setLoading(false)
    setError('')
  }

  return {
    input,
    setInput,
    output,
    loading,
    error,
    process,
    reset,
  }
}
