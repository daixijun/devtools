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
          await globalShortcutManager.initialize()
        } catch (error) {
          console.error('Failed to initialize global shortcuts:', error)
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
