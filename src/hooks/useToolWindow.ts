import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { getToolById } from '../tools/registry'
import { getStoredThemeMode, resolveIsDark } from './useTheme'

const WINDOW_PREFIX = 'tool-'

const DEFAULT_WINDOW_SIZE = { width: 1000, height: 700 }

/** 计算创建工具窗口时应传入的实际主题(light/dark) */
function resolveWindowTheme(): 'light' | 'dark' {
  return resolveIsDark(getStoredThemeMode()) ? 'dark' : 'light'
}

export function useToolWindow() {
  const openTool = async (toolId: string) => {
    const tool = getToolById(toolId)
    if (!tool) {
      console.error(`Tool not found: ${toolId}`)
      return
    }

    const windowLabel = `${WINDOW_PREFIX}${toolId}`

    try {
      const existing = await WebviewWindow.getByLabel(windowLabel)
      if (existing) {
        await existing.setFocus()
        return
      }
    } catch {
      // Window doesn't exist
    }

    const toolWindow = new WebviewWindow(windowLabel, {
      url: `index.html#/tool/${toolId}`,
      title: `${tool.name} - DevTools`,
      width: tool.windowSize?.width ?? DEFAULT_WINDOW_SIZE.width,
      height: tool.windowSize?.height ?? DEFAULT_WINDOW_SIZE.height,
      center: true,
      resizable: true,
      theme: resolveWindowTheme(),
    })

    toolWindow.once('tauri://created', () => {
      console.log(`Tool window created: ${toolId}`)
    })

    toolWindow.once('tauri://error', (e) => {
      console.error(`Error creating tool window: ${toolId}`, e)
    })
  }

  const closeTool = async (toolId: string) => {
    const windowLabel = `${WINDOW_PREFIX}${toolId}`
    try {
      const existing = await WebviewWindow.getByLabel(windowLabel)
      if (existing) {
        await existing.close()
      }
    } catch {
      // Window doesn't exist
    }
  }

  return { openTool, closeTool }
}
