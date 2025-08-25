import { useState, useCallback } from 'react'

interface UseAsyncStateReturn<T> {
  data: T | null
  loading: boolean
  error: string
  execute: (asyncFn: () => Promise<T>) => Promise<void>
  setError: (error: string) => void
  clearError: () => void
  reset: () => void
}

const useAsyncState = <T>(): UseAsyncStateReturn<T> => {
  const [data, setData] = useState<T | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const execute = useCallback(async (asyncFn: () => Promise<T>) => {
    try {
      setLoading(true)
      setError('')
      const result = await asyncFn()
      setData(result)
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err)
      setError(errorMessage)
      setData(null)
    } finally {
      setLoading(false)
    }
  }, [])

  const clearError = useCallback(() => {
    setError('')
  }, [])

  const reset = useCallback(() => {
    setData(null)
    setError('')
    setLoading(false)
  }, [])

  const setErrorDirectly = useCallback((errorMessage: string) => {
    setError(errorMessage)
  }, [])

  return {
    data,
    loading,
    error,
    execute,
    setError: setErrorDirectly,
    clearError,
    reset
  }
}

export default useAsyncState