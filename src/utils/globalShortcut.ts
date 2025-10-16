import { invoke } from '@tauri-apps/api/core'

export class GlobalShortcutManager {
  private initialized = false
  private initPromise: Promise<void> | null = null

  async initialize() {
    if (this.initialized) {
      return
    }

    if (this.initPromise) {
      return this.initPromise
    }

    this.initPromise = this.doInitialize()
    return this.initPromise
  }

  private async doInitialize() {
    try {
      // 添加延迟以确保 Tauri API 完全初始化
      await new Promise((resolve) => setTimeout(resolve, 500))

      // 主动请求当前配置并注册快捷键
      try {
        const config = (await invoke('get_global_shortcut_config')) as any

        if (config?.enabled && config?.hotkey) {
          // 如果快捷键已启用，则在后端已经注册了
          console.log('Global shortcut is enabled:', config.hotkey)
        }
      } catch (error) {
        console.error('获取全局快捷键配置失败:', error)
        // 如果获取配置失败，可能是后端还没准备好，稍后重试
        await new Promise((resolve) => setTimeout(resolve, 1000))
        try {
          const config = (await invoke('get_global_shortcut_config')) as any
          if (config?.enabled && config?.hotkey) {
            console.log('Global shortcut is enabled (retry):', config.hotkey)
          }
        } catch (retryError) {
          console.error('重试获取全局快捷键配置失败:', retryError)
        }
      }

      this.initialized = true
    } catch (error) {
      console.error('全局快捷键初始化失败:', error)
      // 如果初始化失败，重置状态以便重试
      this.initialized = false
      this.initPromise = null
      throw error
    }
  }

  async registerShortcut(hotkey: { modifier: string; key: string }) {
    try {
      // 调用后端注册快捷键
      await invoke('register_global_shortcut', { config: hotkey })
      console.log('Global shortcut registered:', hotkey)
    } catch (error) {
      console.error('注册全局快捷键失败:', error)
      throw error
    }
  }

  async unregisterShortcut() {
    try {
      // 调用后端取消注册快捷键
      await invoke('unregister_global_shortcut')
      console.log('Global shortcut unregistered')
    } catch (error) {
      console.error('取消注册全局快捷键失败:', error)
      throw error
    }
  }

  async setEnabled(enabled: boolean) {
    try {
      await invoke('set_global_shortcut_enabled', { enabled })
      console.log('Global shortcut enabled:', enabled)
    } catch (error) {
      console.error('设置全局快捷键状态失败:', error)
      throw error
    }
  }

  async cleanup() {
    // 清理工作由后端处理
  }
}

// 全局实例
export const globalShortcutManager = new GlobalShortcutManager()
