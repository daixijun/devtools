import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect, useState } from 'react'

/**
 * 主题状态管理Hook
 * 统一管理暗色/明亮主题状态，避免在多个组件中重复实现
 */
export const useTheme = () => {
  const [isDark, setIsDark] = useState(false)
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    const setupTheme = async () => {
      try {
        const window = getCurrentWindow()

        // 获取初始主题
        const theme = await window.theme()
        setIsDark(theme === 'dark')

        // 监听主题变化
        const unlisten = await window.onThemeChanged(({ payload: theme }) => {
          setIsDark(theme === 'dark')
        })

        setIsLoading(false)

        // 返回清理函数
        return unlisten
      } catch (error) {
        console.warn('Failed to setup theme listener:', error)
        setIsLoading(false)
      }
    }

    let cleanup: (() => void) | undefined

    setupTheme().then((unlistenFn) => {
      cleanup = unlistenFn
    })

    return () => {
      if (cleanup) {
        cleanup()
      }
    }
  }, [])

  return {
    isDark,
    isLoading,
    theme: isDark ? 'dark' : 'light',
  }
}
