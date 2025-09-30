import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { register, unregister } from '@tauri-apps/plugin-global-shortcut'

export class GlobalShortcutManager {
  private currentShortcut: string | null = null
  private initialized = false
  private fallbackAttempted = false

  // 尝试替代的快捷键组合
  private getAlternativeHotkeys(original: string): string[] {
    if (original === 'Option+Space') {
      return [
        'Option+T', // Option+T 作为备选
        'Option+D', // Option+D 作为备选
        'Option+X', // Option+X 作为备选
        'Cmd+Space', // 如果 Option+Space 被占用，Cmd+Space 可能可用
      ]
    }
    return []
  }

  async initialize() {
    if (this.initialized) {
      return
    }

    try {
      // 监听来自后端的快捷键事件
      await listen('register-shortcut', (event) => {
        this.registerShortcut(event.payload as string)
      })

      await listen('unregister-shortcut', (event) => {
        this.unregisterShortcut(event.payload as string)
      })

      await listen('init-shortcut', (event) => {
        this.registerShortcut(event.payload as string)
      })

      this.initialized = true

      // 主动请求当前配置并注册快捷键
      try {
        const result = (await invoke('get_global_shortcut_config')) as any

        if (result?.success && result?.data?.enabled && result?.data?.hotkey) {
          const { modifier, key } = result.data.hotkey
          let accelerator: string

          // 构建加速器字符串
          if (modifier === 'option') {
            accelerator = `Option+${key}`
          } else if (modifier === 'cmd') {
            accelerator = `Cmd+${key}`
          } else if (modifier === 'alt') {
            accelerator = `Alt+${key}`
          } else if (modifier === 'ctrl') {
            accelerator = `Ctrl+${key}`
          } else {
            accelerator = `${
              modifier.charAt(0).toUpperCase() + modifier.slice(1)
            }+${key}`
          }

          await this.registerShortcut(accelerator)
        }
      } catch (error) {
        console.error('获取全局快捷键配置失败:', error)
      }
    } catch (error) {
      console.error('全局快捷键初始化失败:', error)
      // 如果初始化失败，重置状态以便重试
      this.initialized = false
    }
  }

  private async registerShortcut(accelerator: string, retryCount = 0) {
    try {
      // 先取消注册旧的快捷键
      if (this.currentShortcut) {
        await this.unregisterShortcut(this.currentShortcut)
        // 取消注册后等待一小段时间
        await new Promise((resolve) => setTimeout(resolve, 100))
      }

      // 注册新的快捷键
      await register(accelerator, async (event) => {
        if (event.state === 'Pressed') {
          try {
            // 调用后端处理器
            await invoke('handle_global_shortcut_triggered')
          } catch (error) {
            console.error('调用后端处理器失败:', error)
          }
        }
      })

      this.currentShortcut = accelerator
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error)

      // 检查是否是权限问题
      if (
        errorMessage.includes('permission') ||
        errorMessage.includes('accessibility')
      ) {
        console.warn(
          '全局快捷键权限问题：请在系统偏好设置 > 安全性与隐私 > 辅助功能中授权应用',
        )
        return
      }

      // 检查是否是快捷键冲突问题
      if (errorMessage.includes('RegisterEventHotKey failed')) {
        // 如果是第一次失败且还没尝试过备选方案，尝试备选快捷键
        if (!this.fallbackAttempted && retryCount === 0) {
          this.fallbackAttempted = true
          const alternatives = this.getAlternativeHotkeys(accelerator)

          if (alternatives.length > 0) {
            for (const alternative of alternatives) {
              try {
                await register(alternative, async (event) => {
                  if (event.state === 'Pressed') {
                    try {
                      await invoke('handle_global_shortcut_triggered')
                    } catch (error) {
                      console.error('调用后端处理器失败:', error)
                    }
                  }
                })

                this.currentShortcut = alternative
                console.log(
                  `快捷键 ${accelerator} 被占用，已自动切换到 ${alternative}`,
                )
                return
              } catch (altError) {
                continue
              }
            }
          }
        }
      }

      // 常规重试机制
      if (retryCount < 2 && errorMessage.includes('failed')) {
        setTimeout(() => {
          this.registerShortcut(accelerator, retryCount + 1)
        }, (retryCount + 1) * 1000)
      } else {
        console.error('全局快捷键注册失败:', errorMessage)
      }
    }
  }

  private async unregisterShortcut(accelerator: string) {
    try {
      if (this.currentShortcut === accelerator) {
        await unregister(accelerator)
        this.currentShortcut = null
      }
    } catch (error) {
      console.error('取消注册全局快捷键失败:', error)
    }
  }

  async cleanup() {
    if (this.currentShortcut) {
      await this.unregisterShortcut(this.currentShortcut)
    }
  }
}

// 全局实例
export const globalShortcutManager = new GlobalShortcutManager()
