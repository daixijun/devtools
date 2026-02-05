import { useState, useEffect } from 'react'
import { useDebounce } from './useDebounce'
import { errorUtils } from '../utils'

/**
 * useToolProcessor - 同步数据处理 Hook
 *
 * @description
 * 为工具组件提供统一的数据处理模式,包括防抖、验证、错误处理
 *
 * @features
 * - 自动防抖处理
 * - 输入验证
 * - 统一错误处理
 * - TypeScript 类型安全
 *
 * @example
 * ```tsx
 * const { input, setInput, output, error } = useToolProcessor(
 *   (str) => str.toUpperCase(),
 *   {
 *     debounceMs: 200,
 *     validate: (str) => str.length > 0 ? null : '输入不能为空'
 *   }
 * )
 * ```
 *
 * @param processor - 处理函数,接收输入返回输出
 * @param options - 配置选项
 * @returns 工具状态和处理函数
 */
export interface UseToolProcessorOptions<T> {
  /** 防抖延迟(毫秒),默认 200 */
  debounceMs?: number
  /** 验证函数,返回错误消息或 null */
  validate?: (input: string) => string | null
}

export interface UseToolProcessorReturn<T> {
  /** 输入值 */
  input: string
  /** 设置输入值 */
  setInput: (value: string) => void
  /** 处理后的输出 */
  output: T | null
  /** 错误消息 */
  error: string
  /** 是否正在处理 */
  isProcessing: boolean
}

export function useToolProcessor<T>(
  processor: (input: string) => T,
  options: UseToolProcessorOptions<T> = {}
): UseToolProcessorReturn<T> {
  const { debounceMs = 200, validate } = options

  const [input, setInput] = useState('')
  const [output, setOutput] = useState<T | null>(null)
  const [error, setError] = useState('')
  const [isProcessing, setIsProcessing] = useState(false)

  // 防抖处理
  const debouncedInput = useDebounce(input, debounceMs)

  useEffect(() => {
    // 空输入处理
    if (!debouncedInput) {
      setOutput(null)
      setError('')
      setIsProcessing(false)
      return
    }

    // 验证输入
    if (validate) {
      const validationError = validate(debouncedInput)
      if (validationError) {
        setOutput(null)
        setError(validationError)
        setIsProcessing(false)
        return
      }
    }

    // 处理输入
    setIsProcessing(true)
    try {
      const result = processor(debouncedInput)
      setOutput(result)
      setError('')
    } catch (err) {
      setOutput(null)
      setError(errorUtils.formatError(err))
    } finally {
      setIsProcessing(false)
    }
  }, [debouncedInput, processor, validate])

  return {
    input,
    setInput,
    output,
    error,
    isProcessing,
  }
}
