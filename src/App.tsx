import { memo, useEffect } from 'react'
import './App.css'
import Toolbox from './Toolbox'
import { globalShortcutManager } from './utils/globalShortcut'

const App = memo(() => {
  useEffect(() => {
    let isMounted = true

    const initializeShortcuts = async () => {
      if (isMounted) {
        try {
          // 添加延迟以确保 Tauri API 完全初始化
          await new Promise((resolve) => setTimeout(resolve, 2000))
          await globalShortcutManager.initialize()
        } catch (error) {
          console.error('Failed to initialize global shortcuts:', error)
          // 如果初始化失败，尝试在稍后重试
          if (isMounted) {
            setTimeout(() => {
              initializeShortcuts()
            }, 3000)
          }
        }
      }
    }

    initializeShortcuts()

    return () => {
      isMounted = false
      globalShortcutManager.cleanup().catch(console.error)
    }
  }, [])

  return (
    <main className='w-screen h-screen bg-gray-100 dark:bg-gray-900'>
      <Toolbox />
    </main>
  )
})

export default App
