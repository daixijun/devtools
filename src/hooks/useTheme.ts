import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { useEffect, useState } from 'react'

export type ThemeMode = 'light' | 'dark' | 'system'

/** 跨窗口主题变更事件名 */
export const THEME_CHANGED_EVENT = 'devtools://theme-changed'

/**
 * 读取持久化的主题设置(权威来源)。
 * localStorage `devtools-settings` 由各窗口共享。
 */
export function getStoredThemeMode(): ThemeMode {
  try {
    const settingsRaw = localStorage.getItem('devtools-settings')
    if (settingsRaw) {
      const settings = JSON.parse(settingsRaw)
      const theme = settings?.theme as ThemeMode | undefined
      if (theme === 'light' || theme === 'dark' || theme === 'system') {
        return theme
      }
    }
  } catch {}
  return 'system'
}

/**
 * 将主题模式解析为实际的明/暗。
 * `system` 模式读取 OS 偏好。
 */
export function resolveIsDark(mode: ThemeMode): boolean {
  if (mode === 'dark') return true
  if (mode === 'light') return false
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

/**
 * 主题状态管理 Hook
 *
 * 权威来源为持久化设置 `devtools-settings.theme`(各窗口共享)。
 * - 首屏从 localStorage 读取,避免闪烁。
 * - 挂载时把原生窗口主题反向同步为与设置一致(避免原生窗口外观漂移)。
 * - 监听跨窗口事件 `devtools://theme-changed`,实现 Settings 改主题后
 *   所有已开窗口(含工具窗口)实时跟随。
 * - 仍监听 OS 系统主题变化,仅在 `system` 模式下更新。
 */
export const useTheme = () => {
  const getInitialTheme = () => {
    try {
      const settingsRaw = localStorage.getItem('devtools-settings')
      if (settingsRaw) {
        const settings = JSON.parse(settingsRaw)
        const theme = settings?.theme as ThemeMode | undefined
        if (theme === 'light' || theme === 'dark' || theme === 'system') {
          return resolveIsDark(theme)
        }
      }

      // 其次读取最近一次的实际主题值
      const saved = localStorage.getItem('devtools-theme')
      if (saved) return saved === 'dark'

      // 如果没有保存的主题,使用系统偏好作为初始猜测
      return window.matchMedia('(prefers-color-scheme: dark)').matches
    } catch {
      return false
    }
  }

  const [isDark, setIsDark] = useState(getInitialTheme)
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    const appWindow = getCurrentWindow()

    /** 根据持久化设置计算 isDark,并同步原生窗口主题 */
    const applyStoredTheme = async () => {
      const mode = getStoredThemeMode()
      const dark = resolveIsDark(mode)
      setIsDark(dark)
      localStorage.setItem('devtools-theme', dark ? 'dark' : 'light')
      // 反向同步原生窗口主题,让窗口外观与本设置一致
      try {
        await appWindow.setTheme(dark ? 'dark' : 'light')
      } catch {}
    }

    const setupTheme = async () => {
      try {
        await applyStoredTheme()

        // 监听跨窗口主题变更事件(Settings 改主题时广播)
        const unlistenEvent = await listen<{
          mode: ThemeMode
        }>(THEME_CHANGED_EVENT, async (event) => {
          const dark = resolveIsDark(event.payload.mode)
          setIsDark(dark)
          localStorage.setItem('devtools-theme', dark ? 'dark' : 'light')
          try {
            await appWindow.setTheme(dark ? 'dark' : 'light')
          } catch {}
        })

        // 监听 OS 系统主题变化,仅在 system 模式下生效
        const unlistenNative = await appWindow.onThemeChanged(
          ({ payload: theme }) => {
            const mode = getStoredThemeMode()
            if (mode !== 'system') return // 非 system 模式忽略原生窗口主题变化
            const newIsDark = theme === 'dark'
            setIsDark(newIsDark)
            if (theme) {
              localStorage.setItem('devtools-theme', theme)
            }
          },
        )

        setIsLoading(false)

        return () => {
          unlistenEvent()
          unlistenNative()
        }
      } catch (error) {
        console.warn('Failed to setup theme listener:', error)
        setIsLoading(false)
      }
    }

    let cleanup: (() => void) | undefined
    setupTheme().then((cleanupFn) => {
      cleanup = cleanupFn
    })

    return () => {
      cleanup?.()
    }
  }, [])

  // 在主题变化时,切换文档根节点的 dark 类,以启用 Tailwind 暗黑样式
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
