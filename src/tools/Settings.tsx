import { invoke, isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import React, { useEffect, useState } from 'react'
import Card from '../components/common/Card/Card'
import PageHeader from '../components/common/PageHeader'

interface HotKeyConfig {
  modifier: 'option' | 'alt' | 'ctrl' | 'cmd'
  key: string
}

interface SettingsConfig {
  theme: 'light' | 'dark' | 'system'
  showTray: boolean
  autoStart: boolean
  startMinimized: boolean
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
    hotkey: {
      modifier: isMac ? 'option' : 'alt',
      key: 'Space'
    }
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
            key: 'Space'
          }
        }
        setSettings(settingsWithDefaults)
      }
    } catch (error) {
      console.error('Failed to load settings:', error)
    }
  }

  const loadSystemSettings = async () => {
    if (!isTauri()) return

    try {
      // 获取托盘状态
      const trayStatus = await invoke<boolean>('get_tray_status')
      // 获取自启动状态
      const autostartStatus = await invoke<boolean>('get_autostart_status')
      // 获取启动时最小化状态
      const startMinimizedStatus = await invoke<boolean>('get_start_minimized_status')

      setSettings((prev) => ({
        ...prev,
        showTray: trayStatus,
        autoStart: autostartStatus,
        startMinimized: startMinimizedStatus,
        // 确保 hotkey 字段存在
        hotkey: prev.hotkey || {
          modifier: isMac ? 'option' : 'alt',
          key: 'Space'
        }
      }))
    } catch (error) {
      console.error('Failed to load system settings:', error)
    }
  }

  const saveSettings = (newSettings: SettingsConfig) => {
    try {
      localStorage.setItem('devtools-settings', JSON.stringify(newSettings))
      setSettings(newSettings)
    } catch (error) {
      console.error('Failed to save settings:', error)
    }
  }

  const handleThemeChange = async (theme: 'light' | 'dark' | 'system') => {
    const newSettings = { ...settings, theme }
    saveSettings(newSettings)

    if (isTauri()) {
      const tauriWindow = getCurrentWindow()
      if (theme === 'system') {
        // 获取系统主题
        const systemTheme = await tauriWindow.theme()
        await tauriWindow.setTheme(systemTheme)
        setCurrentTheme(systemTheme === 'dark' ? 'dark' : 'light')
      } else {
        await tauriWindow.setTheme(theme)
        setCurrentTheme(theme)
      }
    }
  }

  const handleTrayToggle = async (showTray: boolean) => {
    if (!isTauri()) return

    setLoading(true)
    try {
      const result = await invoke<boolean>('toggle_tray', { enabled: showTray })
      const newSettings = { ...settings, showTray: result }
      saveSettings(newSettings)

      // 如果用户关闭了托盘，显示提示
      if (!showTray) {
        // 这里可以显示一个提示，告知用户托盘图标会在下次启动时不再显示
        console.log('托盘图标将在下次应用启动时不再显示')
      }
    } catch (error) {
      console.error('Failed to toggle tray:', error)
      // 如果失败，恢复原状态
      setSettings((prev) => ({ ...prev, showTray: !showTray }))
    } finally {
      setLoading(false)
    }
  }

  const handleAutoStartToggle = async (autoStart: boolean) => {
    if (!isTauri()) return

    setLoading(true)
    try {
      const result = await invoke<boolean>('set_autostart', {
        enabled: autoStart,
      })
      const newSettings = { ...settings, autoStart: result }
      saveSettings(newSettings)
    } catch (error) {
      console.error('Failed to set autostart:', error)
      // 如果失败，恢复原状态
      setSettings((prev) => ({ ...prev, autoStart: !autoStart }))
    } finally {
      setLoading(false)
    }
  }

  const handleStartMinimizedToggle = async (startMinimized: boolean) => {
    if (!isTauri()) return

    setLoading(true)
    try {
      const result = await invoke<boolean>('set_start_minimized', {
        enabled: startMinimized,
      })
      const newSettings = { ...settings, startMinimized: result }
      saveSettings(newSettings)
    } catch (error) {
      console.error('Failed to set start minimized:', error)
      // 如果失败，恢复原状态
      setSettings((prev) => ({ ...prev, startMinimized: !startMinimized }))
    } finally {
      setLoading(false)
    }
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
        key: e.key === ' ' ? 'Space' : e.key
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

  const registerHotkeyWithBackend = async (newHotkey: HotKeyConfig) => {
    if (!isTauri()) {
      // 如果不在 Tauri 环境中，只更新本地状态
      const newSettings = { 
        ...settings, 
        hotkey: newHotkey,
        theme: settings.theme || 'system',
        showTray: settings.showTray !== undefined ? settings.showTray : true,
        autoStart: settings.autoStart !== undefined ? settings.autoStart : false,
        startMinimized: settings.startMinimized !== undefined ? settings.startMinimized : false
      }
      saveSettings(newSettings)
      setIsRecordingHotkey(false)
      return
    }

    try {
      setLoading(true)
      
      // 调用后端 API 注册全局快捷键
      const result = await invoke('register_global_shortcut', { config: newHotkey })
      
      if (result) {
        // 更新本地设置
        const newSettings = { 
          ...settings, 
          hotkey: newHotkey,
          theme: settings.theme || 'system',
          showTray: settings.showTray !== undefined ? settings.showTray : true,
          autoStart: settings.autoStart !== undefined ? settings.autoStart : false,
          startMinimized: settings.startMinimized !== undefined ? settings.startMinimized : false
        }
        saveSettings(newSettings)
        console.log('全局快捷键注册成功')
      }
    } catch (error) {
      console.error('Failed to register global shortcut:', error)
      // 如果注册失败，显示错误但仍更新本地设置以保存用户选择
      const newSettings = { 
        ...settings, 
        hotkey: newHotkey,
        theme: settings.theme || 'system',
        showTray: settings.showTray !== undefined ? settings.showTray : true,
        autoStart: settings.autoStart !== undefined ? settings.autoStart : false,
        startMinimized: settings.startMinimized !== undefined ? settings.startMinimized : false
      }
      saveSettings(newSettings)
    } finally {
      setLoading(false)
      setIsRecordingHotkey(false)
    }
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
      'option': '⌥',
      'alt': 'Alt',
      'ctrl': isMac ? '⌃' : 'Ctrl',
      'cmd': '⌘'
    }
    
    const keyDisplay = hotkey.key === 'Space' ? '空格' : hotkey.key
    return `${modifierDisplay[hotkey.modifier]} + ${keyDisplay}`
  }

  return (
    <div className='h-full flex flex-col'>
      <PageHeader title='设置' subtitle='配置应用程序偏好设置' />

      <div className='flex-1 space-y-6 p-6'>
        {/* 外观设置 */}
        <Card>
          <div className='p-6'>
            <h3 className='text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4'>
              外观设置
            </h3>

            <div className='space-y-4'>
              <div>
                <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2'>
                  主题模式
                </label>
                <div className='flex space-x-4'>
                  {(['light', 'dark', 'system'] as const).map((themeOption) => (
                    <button
                      key={themeOption}
                      onClick={() => handleThemeChange(themeOption)}
                      className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${
                        settings.theme === themeOption
                          ? 'bg-blue-500 text-white'
                          : 'bg-gray-100 text-gray-700 hover:bg-gray-200 dark:bg-gray-700 dark:text-gray-300 dark:hover:bg-gray-600'
                      }`}>
                      {themeOption === 'light' && '☀️ 浅色'}
                      {themeOption === 'dark' && '🌙 深色'}
                      {themeOption === 'system' && '🖥️ 跟随系统'}
                    </button>
                  ))}
                </div>
                <p className='text-xs text-gray-500 dark:text-gray-400 mt-2'>
                  当前主题: {currentTheme === 'dark' ? '深色' : '浅色'}
                </p>
              </div>
            </div>
          </div>
        </Card>

        {/* 系统设置 */}
        <Card>
          <div className='p-6'>
            <h3 className='text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4'>
              系统设置
            </h3>

            <div className='space-y-4'>
              {/* 托盘设置 */}
              <div className='flex items-center justify-between'>
                <div>
                  <label className='text-sm font-medium text-gray-700 dark:text-gray-300'>
                    显示系统托盘图标
                  </label>
                  <p className='text-xs text-gray-500 dark:text-gray-400'>
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
                        ? 'bg-blue-500'
                        : 'bg-gray-300 dark:bg-gray-600'
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
                  <label className='text-sm font-medium text-gray-700 dark:text-gray-300'>
                    开机自动启动
                  </label>
                  <p className='text-xs text-gray-500 dark:text-gray-400'>
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
                        ? 'bg-blue-500'
                        : 'bg-gray-300 dark:bg-gray-600'
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
                  <label className='text-sm font-medium text-gray-700 dark:text-gray-300'>
                    启动时最小化到托盘
                  </label>
                  <p className='text-xs text-gray-500 dark:text-gray-400'>
                    应用程序启动时直接最小化到系统托盘
                  </p>
                </div>
                <label className='relative inline-flex items-center cursor-pointer'>
                  <input
                    type='checkbox'
                    className='sr-only'
                    checked={settings.startMinimized}
                    disabled={loading}
                    onChange={(e) => handleStartMinimizedToggle(e.target.checked)}
                  />
                  <div
                    className={`w-11 h-6 rounded-full transition-colors ${
                      settings.startMinimized
                        ? 'bg-blue-500'
                        : 'bg-gray-300 dark:bg-gray-600'
                    } ${loading ? 'opacity-50' : ''}`}>
                    <div
                      className={`w-5 h-5 bg-white rounded-full shadow transform transition-transform ${
                        settings.startMinimized ? 'translate-x-5' : 'translate-x-0'
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
            <h3 className='text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4'>
              快捷键设置
            </h3>

            <div className='space-y-4'>
              {/* 全局快捷键设置 */}
              <div className='flex items-center justify-between'>
                <div>
                  <label className='text-sm font-medium text-gray-700 dark:text-gray-300'>
                    快速调出工具
                  </label>
                  <p className='text-xs text-gray-500 dark:text-gray-400'>
                    设置全局快捷键来快速显示/隐藏应用程序
                  </p>
                </div>
                <div className='flex items-center space-x-3'>
                  <div className='px-3 py-2 bg-gray-100 dark:bg-gray-700 rounded-md border border-gray-300 dark:border-gray-600 min-w-[120px] text-center'>
                    <span className='text-sm font-mono text-gray-800 dark:text-gray-200'>
                      {getHotkeyDisplayText(settings.hotkey)}
                    </span>
                  </div>
                  <button
                    onClick={handleHotkeyRecord}
                    disabled={isRecordingHotkey}
                    className={`px-3 py-2 text-sm rounded-md transition-colors ${
                      isRecordingHotkey
                        ? 'bg-orange-500 text-white cursor-not-allowed'
                        : 'bg-blue-500 hover:bg-blue-600 text-white'
                    }`}>
                    {isRecordingHotkey ? '按下新组合键...' : '修改'}
                  </button>
                </div>
              </div>
              
              {isRecordingHotkey && (
                <div className='p-3 bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded-md'>
                  <p className='text-sm text-orange-700 dark:text-orange-300'>
                    🎯 请按下新的快捷键组合（必须包含修饰键：Ctrl、Alt/Option、Cmd）
                  </p>
                  <p className='text-xs text-orange-600 dark:text-orange-400 mt-1'>
                    10秒后自动取消录制
                  </p>
                </div>
              )}
              
              <div className='p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-md'>
                <p className='text-sm text-blue-700 dark:text-blue-300'>
                  💡 <strong>使用说明：</strong>
                </p>
                <ul className='text-xs text-blue-600 dark:text-blue-400 mt-1 space-y-1'>
                  <li>• 全局快捷键可以在任何应用程序中使用</li>
                  <li>• 默认快捷键：{isMac ? 'Option + 空格' : 'Alt + 空格'}</li>
                  <li>• 建议使用不与其他应用冲突的组合键</li>
                </ul>
              </div>
            </div>
          </div>
        </Card>

        {/* 关于信息 */}
        <Card>
          <div className='p-6'>
            <h3 className='text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4'>
              关于应用
            </h3>

            <div className='space-y-3'>
              <div className='flex justify-between'>
                <span className='text-sm text-gray-600 dark:text-gray-400'>
                  应用名称
                </span>
                <span className='text-sm font-medium text-gray-900 dark:text-gray-100'>
                  开发者工具箱
                </span>
              </div>
              <div className='flex justify-between'>
                <span className='text-sm text-gray-600 dark:text-gray-400'>
                  版本
                </span>
                <span className='text-sm font-medium text-gray-900 dark:text-gray-100'>
                  1.0.0
                </span>
              </div>
              <div className='flex justify-between'>
                <span className='text-sm text-gray-600 dark:text-gray-400'>
                  技术栈
                </span>
                <span className='text-sm font-medium text-gray-900 dark:text-gray-100'>
                  Tauri + React + TypeScript
                </span>
              </div>
            </div>
          </div>
        </Card>
      </div>
    </div>
  )
}

export default Settings
