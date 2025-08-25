import { useState, useCallback } from 'react'

/**
 * Toast 通知管理Hook
 * 统一管理消息提示显示逻辑，避免在多个组件中重复实现
 */
export const useToast = () => {
  const [toast, setToast] = useState('')
  const [show, setShow] = useState(false)

  const showToast = useCallback((message: string, duration = 2000) => {
    setToast(message)
    setShow(true)
    
    // 清除之前的定时器
    const timer = setTimeout(() => {
      setShow(false)
      // 在动画结束后清空消息内容
      setTimeout(() => setToast(''), 300)
    }, duration)

    return () => clearTimeout(timer)
  }, [])

  const hideToast = useCallback(() => {
    setShow(false)
    setTimeout(() => setToast(''), 300)
  }, [])

  return {
    toast,
    show,
    showToast,
    hideToast
  }
}