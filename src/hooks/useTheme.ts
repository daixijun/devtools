import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect, useState } from 'react'

/**
 * 主题状态管理Hook
 * 统一管理暗色/明亮主题状态，避免在多个组件中重复实现
 * 支持 localStorage 缓存，提供更好的首次加载体验
 */
export const useTheme = () => {
  // 尝试从 localStorage 获取上次的主题作为初始值
  const getInitialTheme = () => {
    try {
      // 优先读取设置页保存的配置
      const settingsRaw = localStorage.getItem('devtools-settings')
      if (settingsRaw) {
        const settings = JSON.parse(settingsRaw)
        const theme = settings?.theme as 'light' | 'dark' | 'system' | undefined
        if (theme === 'dark') return true
        if (theme === 'light') return false
        // system 模式走系统偏好
        if (theme === 'system') {
          return window.matchMedia('(prefers-color-scheme: dark)').matches
        }
      }

      // 其次读取最近一次的实际主题值
      const saved = localStorage.getItem('devtools-theme')
      if (saved) return saved === 'dark'

      // 如果没有保存的主题，使用系统偏好作为初始猜测
      return window.matchMedia('(prefers-color-scheme: dark)').matches
    } catch {
      return false
    }
  }

  const [isDark, setIsDark] = useState(getInitialTheme)
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    const setupTheme = async () => {
      try {
        const window = getCurrentWindow()

        // 获取实际主题
        const theme = await window.theme()
        const actualIsDark = theme === 'dark'

        // 保存到 localStorage 以供下次使用
        if (theme) {
          localStorage.setItem('devtools-theme', theme)
        }

        setIsDark(actualIsDark)

        // 监听主题变化
        const unlisten = await window.onThemeChanged(({ payload: theme }) => {
          const newIsDark = theme === 'dark'
          setIsDark(newIsDark)
          if (theme) {
            localStorage.setItem('devtools-theme', theme)
          }
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

  // 在主题变化时，切换文档根节点的 dark 类，以启用 Tailwind 暗黑样式
  useEffect(() => {
    try {
      const root = document.documentElement
      if (isDark) root.classList.add('dark')
      else root.classList.remove('dark')
    } catch {}
  }, [isDark])

  return {
    isDark,
    isLoading,
    theme: isDark ? 'dark' : 'light',
  }
}
