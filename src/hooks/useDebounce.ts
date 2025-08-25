import { useState, useEffect } from 'react'

/**
 * 防抖Hook
 * 延迟更新值，避免频繁触发后续操作
 */
export const useDebounce = <T>(value: T, delay: number): T => {
  const [debouncedValue, setDebouncedValue] = useState<T>(value)

  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedValue(value)
    }, delay)

    return () => {
      clearTimeout(handler)
    }
  }, [value, delay])

  return debouncedValue
}

/**
 * 防抖回调Hook
 * 防抖执行回调函数
 */
export const useDebounceCallback = <T extends any[]>(
  callback: (...args: T) => void,
  delay: number
) => {
  const [timeoutId, setTimeoutId] = useState<NodeJS.Timeout | null>(null)

  const debouncedCallback = (...args: T) => {
    if (timeoutId) {
      clearTimeout(timeoutId)
    }

    const id = setTimeout(() => {
      callback(...args)
    }, delay)

    setTimeoutId(id)
  }

  useEffect(() => {
    return () => {
      if (timeoutId) {
        clearTimeout(timeoutId)
      }
    }
  }, [timeoutId])

  return debouncedCallback
}