import { invoke, isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import React, { useEffect, useState } from 'react'
import { Card } from '../components/common'
import { ToolLayout } from '../components/layouts'

interface HotKeyConfig {
  modifier: 'option' | 'alt' | 'ctrl' | 'cmd'
  key: string
}

interface SettingsConfig {
  theme: 'light' | 'dark' | 'system'
  showTray: boolean
  autoStart: boolean
  startMinimized: boolean
  closeToTray: boolean
  hotkey: HotKeyConfig
}

const Settings: React.FC = () => {
  // 检测操作系统
  const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0

  const [settings, setSettings] = useState<SettingsConfig>({
    theme: 'system',
    showTray: true,
    autoStart: false,
    startMinimized: false,
    closeToTray: true,
    hotkey: {
      modifier: isMac ? 'option' : 'alt',
      key: 'Space',
    },
  })
  const [currentTheme, setCurrentTheme] = useState<'light' | 'dark'>('light')
  const [loading, setLoading] = useState(false)
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false)

  useEffect(() => {
    loadSettings()
    if (isTauri()) {
      getCurrentWindow()
        .theme()
        .then((theme) => setCurrentTheme(theme === 'dark' ? 'dark' : 'light'))
      // 获取系统状态
      loadSystemSettings()
    }
  }, [])

  const loadSettings = () => {
    try {
      const saved = localStorage.getItem('devtools-settings')
      if (saved) {
        const parsedSettings = JSON.parse(saved)
        // 确保 hotkey 字段存在，如果不存在则使用默认值
        const settingsWithDefaults = {
          ...parsedSettings,
          hotkey: parsedSettings.hotkey || {
            modifier: isMac ? 'option' : 'alt',
            key: 'Space',
          },
        }
        setSettings(settingsWithDefaults)
      }
    } catch (error) {
      console.error('Failed to load settings:', error)
    }
  }

  const loadSystemSettings = () => {
    if (!isTauri()) return

    // 获取托盘状态
    invoke<boolean>('get_tray_status')
      .then((trayStatus) => {
        // 获取自启动状态
        return invoke<boolean>('get_autostart_status').then(
          (autostartStatus) => {
            // 获取启动时最小化状态
            return invoke<boolean>('get_start_minimized_status').then(
              (startMinimizedStatus) => {
                // 获取关闭时最小化到托盘状态
                return invoke<boolean>('get_close_to_tray_status').then(
                  (closeToTrayStatus) => {
                    setSettings((prev) => ({
                      ...prev,
                      showTray: trayStatus,
                      autoStart: autostartStatus,
                      startMinimized: startMinimizedStatus,
                      closeToTray: closeToTrayStatus,
                      // 确保 hotkey 字段存在
                      hotkey: prev.hotkey || {
                        modifier: isMac ? 'option' : 'alt',
                        key: 'Space',
                      },
                    }))
                  },
                )
              },
            )
          },
        )
      })
      .catch((error) => {
        console.error('Failed to load system settings:', error)
      })
  }

  const saveSettings = (newSettings: SettingsConfig) => {
    try {
      localStorage.setItem('devtools-settings', JSON.stringify(newSettings))
      setSettings(newSettings)
    } catch (error) {
      console.error('Failed to save settings:', error)
    }
  }

  const handleThemeChange = (theme: 'light' | 'dark' | 'system') => {
    const newSettings = { ...settings, theme }
    saveSettings(newSettings)

    if (isTauri()) {
      const tauriWindow = getCurrentWindow()
      if (theme === 'system') {
        // 获取系统主题
        tauriWindow.theme().then((systemTheme) => {
          return tauriWindow.setTheme(systemTheme).then(() => {
            setCurrentTheme(systemTheme === 'dark' ? 'dark' : 'light')
          })
        })
      } else {
        tauriWindow.setTheme(theme).then(() => {
          setCurrentTheme(theme)
        })
      }
    }
  }

  const handleTrayToggle = (showTray: boolean) => {
    if (!isTauri()) return

    setLoading(true)

    invoke<boolean>('toggle_tray', { enabled: showTray })
      .then((result) => {
        const newSettings = { ...settings, showTray: result }
        saveSettings(newSettings)

        // 如果用户关闭了托盘，显示提示
        if (!showTray) {
          // 这里可以显示一个提示，告知用户托盘图标会在下次启动时不再显示
          console.log('托盘图标将在下次应用启动时不再显示')
        } else {
          // 如果启用了托盘，需要从后端重新获取这两个选项的实际值
          invoke<boolean>('get_start_minimized_status')
            .then((startMinimizedStatus) => {
              return invoke<boolean>('get_close_to_tray_status').then(
                (closeToTrayStatus) => {
                  setSettings((prev) => ({
                    ...prev,
                    startMinimized: startMinimizedStatus,
                    closeToTray: closeToTrayStatus,
                  }))
                },
              )
            })
            .catch((error) => {
              console.error('Failed to load tray-related settings:', error)
            })
        }
      })
      .catch((error) => {
        console.error('Failed to toggle tray:', error)
        // 如果失败，恢复原状态
        setSettings((prev) => ({ ...prev, showTray: !showTray }))
      })
      .finally(() => {
        setLoading(false)
      })
  }

  const handleAutoStartToggle = (autoStart: boolean) => {
    if (!isTauri()) return

    setLoading(true)

    invoke<boolean>('set_autostart', {
      enabled: autoStart,
    })
      .then((result) => {
        const newSettings = { ...settings, autoStart: result }
        saveSettings(newSettings)
      })
      .catch((error) => {
        console.error('Failed to set autostart:', error)
        // 如果失败，恢复原状态
        setSettings((prev) => ({ ...prev, autoStart: !autoStart }))
      })
      .finally(() => {
        setLoading(false)
      })
  }

  const handleStartMinimizedToggle = (startMinimized: boolean) => {
    if (!isTauri()) return

    // 如果托盘未启用，不执行任何操作，只更新UI状态
    if (!settings.showTray) {
      // 只更新本地状态，不调用后端API
      const newSettings = { ...settings, startMinimized }
      saveSettings(newSettings)
      return
    }

    setLoading(true)

    invoke<boolean>('set_start_minimized', {
      enabled: startMinimized,
    })
      .then((result) => {
        const newSettings = { ...settings, startMinimized: result }
        saveSettings(newSettings)
      })
      .catch((error) => {
        console.error('Failed to set start minimized:', error)
        // 如果失败，恢复原状态
        setSettings((prev) => ({ ...prev, startMinimized: !startMinimized }))
      })
      .finally(() => {
        setLoading(false)
      })
  }

  const handleCloseToTrayToggle = (closeToTray: boolean) => {
    if (!isTauri()) return

    // 如果托盘未启用，不执行任何操作，只更新UI状态
    if (!settings.showTray) {
      // 只更新本地状态，不调用后端API
      const newSettings = { ...settings, closeToTray }
      saveSettings(newSettings)
      return
    }

    setLoading(true)

    invoke<boolean>('set_close_to_tray', {
      enabled: closeToTray,
    })
      .then((result) => {
        const newSettings = { ...settings, closeToTray: result }
        saveSettings(newSettings)
      })
      .catch((error) => {
        console.error('Failed to set close to tray:', error)
        // 如果失败，恢复原状态
        setSettings((prev) => ({ ...prev, closeToTray: !closeToTray }))
      })
      .finally(() => {
        setLoading(false)
      })
  }

  const handleHotkeyRecord = () => {
    setIsRecordingHotkey(true)

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault()

      let modifier: 'option' | 'alt' | 'ctrl' | 'cmd'

      if (e.metaKey) {
        modifier = 'cmd'
      } else if (e.altKey) {
        modifier = isMac ? 'option' : 'alt'
      } else if (e.ctrlKey) {
        modifier = 'ctrl'
      } else {
        // 必须有修饰键
        return
      }

      // 排除单独的修饰键
      if (['Control', 'Alt', 'Meta', 'Shift'].includes(e.key)) {
        return
      }

      const newHotkey = {
        modifier,
        key: e.key === ' ' ? 'Space' : e.key,
      }

      // 调用后端注册快捷键
      registerHotkeyWithBackend(newHotkey)

      document.removeEventListener('keydown', handleKeyDown)
    }

    document.addEventListener('keydown', handleKeyDown)

    // 10秒后自动取消录制
    const timeoutId = setTimeout(() => {
      setIsRecordingHotkey(false)
      document.removeEventListener('keydown', handleKeyDown)
    }, 10000)

    // 保存timeout ID以便在需要时清除
    const cleanup = () => {
      clearTimeout(timeoutId)
      document.removeEventListener('keydown', handleKeyDown)
    }

    // 在组件卸载时清理
    return cleanup
  }

  const registerHotkeyWithBackend = (newHotkey: HotKeyConfig) => {
    if (!isTauri()) {
      // 如果不在 Tauri 环境中，只更新本地状态
      const newSettings = {
        ...settings,
        hotkey: newHotkey,
        theme: settings.theme || 'system',
        showTray: settings.showTray !== undefined ? settings.showTray : true,
        autoStart:
          settings.autoStart !== undefined ? settings.autoStart : false,
        startMinimized:
          settings.startMinimized !== undefined
            ? settings.startMinimized
            : false,
      }
      saveSettings(newSettings)
      setIsRecordingHotkey(false)
      return
    }

    setLoading(true)

    // 调用后端 API 注册全局快捷键
    invoke<boolean>('register_global_shortcut', { config: newHotkey })
      .then(() => {
        // 更新本地设置
        const newSettings = {
          ...settings,
          hotkey: newHotkey,
          theme: settings.theme || 'system',
          showTray: settings.showTray !== undefined ? settings.showTray : true,
          autoStart:
            settings.autoStart !== undefined ? settings.autoStart : false,
          startMinimized:
            settings.startMinimized !== undefined
              ? settings.startMinimized
              : false,
        }
        saveSettings(newSettings)
        console.log('全局快捷键注册成功')
      })
      .catch((error) => {
        console.error('Failed to register global shortcut:', error)
        // 如果注册失败，显示错误但仍更新本地设置以保存用户选择
        const newSettings = {
          ...settings,
          hotkey: newHotkey,
          theme: settings.theme || 'system',
          showTray: settings.showTray !== undefined ? settings.showTray : true,
          autoStart:
            settings.autoStart !== undefined ? settings.autoStart : false,
          startMinimized:
            settings.startMinimized !== undefined
              ? settings.startMinimized
              : false,
        }
        saveSettings(newSettings)
        // 显示错误提示
        alert(`快捷键注册失败: ${error}. 请检查快捷键是否已被其他应用占用。`)
      })
      .finally(() => {
        setLoading(false)
        setIsRecordingHotkey(false)
      })
  }

  // 在组件卸载时清理事件监听器
  useEffect(() => {
    return () => {
      setIsRecordingHotkey(false)
    }
  }, [])

  const getHotkeyDisplayText = (hotkey: HotKeyConfig) => {
    // 添加安全检查
    if (!hotkey || !hotkey.modifier || !hotkey.key) {
      return isMac ? '⌥ + 空格' : 'Alt + 空格'
    }

    const modifierDisplay = {
      option: '⌥',
      alt: 'Alt',
      ctrl: isMac ? '⌃' : 'Ctrl',
      cmd: '⌘',
    }

    const keyDisplay = hotkey.key === 'Space' ? '空格' : hotkey.key
    return `${modifierDisplay[hotkey.modifier]} + ${keyDisplay}`
  }

  return (
    <ToolLayout title='设置' description='配置应用程序偏好设置'>
      <div className='space-y-6'>
        {/* 外观设置 */}
        <Card>
          <div className='p-6'>
            <h3 className='text-lg font-semibold text-slate-900 dark:text-slate-100 mb-4'>
              外观设置
            </h3>

            <div className='space-y-4'>
              <div>
                <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                  主题模式
                </label>
                <div className='flex space-x-4'>
                  {(['light', 'dark', 'system'] as const).map((themeOption) => (
                    <button
                      key={themeOption}
                      onClick={() => handleThemeChange(themeOption)}
                      className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                        settings.theme === themeOption
                          ? 'bg-primary-500 text-white'
                          : 'bg-slate-100 text-slate-700 hover:bg-slate-200 dark:bg-slate-700 dark:text-slate-300 dark:hover:bg-slate-600'
                      }`}>
                      {themeOption === 'light' && '☀️ 浅色'}
                      {themeOption === 'dark' && '🌙 深色'}
                      {themeOption === 'system' && '🖥️ 跟随系统'}
                    </button>
                  ))}
                </div>
                <p className='text-xs text-slate-500 dark:text-slate-400 mt-2'>
                  当前主题: {currentTheme === 'dark' ? '深色' : '浅色'}
                </p>
              </div>
            </div>
          </div>
        </Card>

        {/* 系统设置 */}
        <Card>
          <div className='p-6'>
            <h3 className='text-lg font-semibold text-slate-900 dark:text-slate-100 mb-4'>
              系统设置
            </h3>

            <div className='space-y-4'>
              {/* 托盘设置 */}
              <div className='flex items-center justify-between'>
                <div>
                  <label className='text-sm font-medium text-slate-700 dark:text-slate-300'>
                    显示系统托盘图标
                  </label>
                  <p className='text-xs text-slate-500 dark:text-slate-400'>
                    在系统托盘中显示应用图标，便于快速访问
                  </p>
                </div>
                <label className='relative inline-flex items-center cursor-pointer'>
                  <input
                    type='checkbox'
                    className='sr-only'
                    checked={settings.showTray}
                    disabled={loading}
                    onChange={(e) => handleTrayToggle(e.target.checked)}
                  />
                  <div
                    className={`w-11 h-6 rounded-full transition-colors ${
                      settings.showTray
                        ? 'bg-primary-500'
                        : 'bg-slate-300 dark:bg-slate-600'
                    } ${loading ? 'opacity-50' : ''}`}>
                    <div
                      className={`w-5 h-5 bg-white rounded-full shadow transform transition-transform ${
                        settings.showTray ? 'translate-x-5' : 'translate-x-0'
                      } mt-0.5 ml-0.5`}
                    />
                  </div>
                </label>
              </div>

              {/* 开机自启 */}
              <div className='flex items-center justify-between'>
                <div>
                  <label className='text-sm font-medium text-slate-700 dark:text-slate-300'>
                    开机自动启动
                  </label>
                  <p className='text-xs text-slate-500 dark:text-slate-400'>
                    系统启动时自动运行此应用程序
                  </p>
                </div>
                <label className='relative inline-flex items-center cursor-pointer'>
                  <input
                    type='checkbox'
                    className='sr-only'
                    checked={settings.autoStart}
                    disabled={loading}
                    onChange={(e) => handleAutoStartToggle(e.target.checked)}
                  />
                  <div
                    className={`w-11 h-6 rounded-full transition-colors ${
                      settings.autoStart
                        ? 'bg-primary-500'
                        : 'bg-slate-300 dark:bg-slate-600'
                    } ${loading ? 'opacity-50' : ''}`}>
                    <div
                      className={`w-5 h-5 bg-white rounded-full shadow transform transition-transform ${
                        settings.autoStart ? 'translate-x-5' : 'translate-x-0'
                      } mt-0.5 ml-0.5`}
                    />
                  </div>
                </label>
              </div>

              {/* 启动时最小化到托盘 */}
              <div className='flex items-center justify-between'>
                <div>
                  <label
                    className={`text-sm font-medium ${
                      settings.showTray
                        ? 'text-slate-700 dark:text-slate-300'
                        : 'text-slate-400 dark:text-slate-500'
                    }`}>
                    启动时最小化到托盘
                  </label>
                  <p
                    className={`text-xs ${
                      settings.showTray
                        ? 'text-slate-500 dark:text-slate-400'
                        : 'text-slate-400 dark:text-slate-500'
                    }`}>
                    应用程序启动时直接最小化到系统托盘
                  </p>
                </div>
                <label className='relative inline-flex items-center cursor-pointer'>
                  <input
                    type='checkbox'
                    className='sr-only'
                    checked={settings.startMinimized}
                    disabled={loading || !settings.showTray}
                    onChange={(e) =>
                      handleStartMinimizedToggle(e.target.checked)
                    }
                  />
                  <div
                    className={`w-11 h-6 rounded-full transition-colors ${
                      settings.startMinimized && settings.showTray
                        ? 'bg-primary-500'
                        : 'bg-slate-300 dark:bg-slate-600'
                    } ${loading || !settings.showTray ? 'opacity-50' : ''}`}>
                    <div
                      className={`w-5 h-5 bg-white rounded-full shadow transform transition-transform ${
                        settings.startMinimized && settings.showTray
                          ? 'translate-x-5'
                          : 'translate-x-0'
                      } mt-0.5 ml-0.5`}
                    />
                  </div>
                </label>
              </div>

              {/* 关闭时最小化到托盘 */}
              <div className='flex items-center justify-between'>
                <div>
                  <label
                    className={`text-sm font-medium ${
                      settings.showTray
                        ? 'text-slate-700 dark:text-slate-300'
                        : 'text-slate-400 dark:text-slate-500'
                    }`}>
                    关闭时最小化到托盘
                  </label>
                  <p
                    className={`text-xs ${
                      settings.showTray
                        ? 'text-slate-500 dark:text-slate-400'
                        : 'text-slate-400 dark:text-slate-500'
                    }`}>
                    关闭窗口时最小化到系统托盘而非退出程序
                  </p>
                </div>
                <label className='relative inline-flex items-center cursor-pointer'>
                  <input
                    type='checkbox'
                    className='sr-only'
                    checked={settings.closeToTray}
                    disabled={loading || !settings.showTray}
                    onChange={(e) => handleCloseToTrayToggle(e.target.checked)}
                  />
                  <div
                    className={`w-11 h-6 rounded-full transition-colors ${
                      settings.closeToTray && settings.showTray
                        ? 'bg-primary-500'
                        : 'bg-slate-300 dark:bg-slate-600'
                    } ${loading || !settings.showTray ? 'opacity-50' : ''}`}>
                    <div
                      className={`w-5 h-5 bg-white rounded-full shadow transform transition-transform ${
                        settings.closeToTray && settings.showTray
                          ? 'translate-x-5'
                          : 'translate-x-0'
                      } mt-0.5 ml-0.5`}
                    />
                  </div>
                </label>
              </div>
            </div>
          </div>
        </Card>

        {/* 快捷键设置 */}
        <Card>
          <div className='p-6'>
            <h3 className='text-lg font-semibold text-slate-900 dark:text-slate-100 mb-4'>
              快捷键设置
            </h3>

            <div className='space-y-4'>
              {/* 全局快捷键设置 */}
              <div className='flex items-center justify-between'>
                <div>
                  <label className='text-sm font-medium text-slate-700 dark:text-slate-300'>
                    快速调出工具
                  </label>
                  <p className='text-xs text-slate-500 dark:text-slate-400'>
                    设置全局快捷键来快速显示/隐藏应用程序
                  </p>
                </div>
                <div className='flex items-center space-x-3'>
                  <div className='px-3 py-2 bg-slate-100 dark:bg-slate-700 rounded-lg border border-slate-300 dark:border-slate-600 min-w-[120px] text-center'>
                    <span className='text-sm font-mono text-slate-800 dark:text-slate-200'>
                      {getHotkeyDisplayText(settings.hotkey)}
                    </span>
                  </div>
                  <button
                    onClick={handleHotkeyRecord}
                    disabled={isRecordingHotkey}
                    className={`px-3 py-2 text-sm rounded-lg transition-colors ${
                      isRecordingHotkey
                        ? 'bg-orange-500 text-white cursor-not-allowed'
                        : 'bg-primary-500 hover:bg-primary-600 text-white'
                    }`}>
                    {isRecordingHotkey ? '按下新组合键...' : '修改'}
                  </button>
                </div>
              </div>

              {isRecordingHotkey && (
                <div className='p-3 bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded-lg'>
                  <p className='text-sm text-orange-700 dark:text-orange-300'>
                    🎯
                    请按下新的快捷键组合（必须包含修饰键：Ctrl、Alt/Option、Cmd）
                  </p>
                  <p className='text-xs text-orange-600 dark:text-orange-400 mt-1'>
                    10秒后自动取消录制
                  </p>
                </div>
              )}

              <div className='p-3 bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 rounded-lg'>
                <p className='text-sm text-primary-700 dark:text-primary-300'>
                  💡 <strong>使用说明：</strong>
                </p>
                <ul className='text-xs text-primary-600 dark:text-primary-400 mt-1 space-y-1'>
                  <li>• 全局快捷键可以在任何应用程序中使用</li>
                  <li>
                    • 默认快捷键：{isMac ? 'Option + 空格' : 'Alt + 空格'}
                  </li>
                  <li>• 建议使用不与其他应用冲突的组合键</li>
                </ul>
              </div>
            </div>
          </div>
        </Card>

        {/* 关于信息 */}
        <Card>
          <div className='p-6'>
            <h3 className='text-lg font-semibold text-slate-900 dark:text-slate-100 mb-4'>
              关于应用
            </h3>

            <div className='space-y-3'>
              <div className='flex justify-between'>
                <span className='text-sm text-slate-600 dark:text-slate-400'>
                  应用名称
                </span>
                <span className='text-sm font-medium text-slate-900 dark:text-slate-100'>
                  开发者工具箱
                </span>
              </div>
              <div className='flex justify-between'>
                <span className='text-sm text-slate-600 dark:text-slate-400'>
                  版本
                </span>
                <span className='text-sm font-medium text-slate-900 dark:text-slate-100'>
                  1.0.0
                </span>
              </div>
              <div className='flex justify-between'>
                <span className='text-sm text-slate-600 dark:text-slate-400'>
                  技术栈
                </span>
                <span className='text-sm font-medium text-slate-900 dark:text-slate-100'>
                  Tauri + React + TypeScript
                </span>
              </div>
            </div>
          </div>
        </Card>
      </div>
    </ToolLayout>
  )
}

export default Settings
