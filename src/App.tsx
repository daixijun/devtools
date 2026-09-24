import { useEffect, useState } from 'react'
import './App.css'
import SpotlightSearch from './components/SpotlightSearch'
import ToolWindow from './components/ToolWindow'
import { globalShortcutManager } from './utils/globalShortcut'

const App = () => {
  const [route, setRoute] = useState(() => {
    const hash = window.location.hash
    if (hash.startsWith('#/tool/')) {
      return { type: 'tool' as const, toolId: hash.replace('#/tool/', '') }
    }
    return { type: 'spotlight' as const }
  })

  useEffect(() => {
    const handleHashChange = () => {
      const hash = window.location.hash
      if (hash.startsWith('#/tool/')) {
        setRoute({ type: 'tool', toolId: hash.replace('#/tool/', '') })
      } else {
        setRoute({ type: 'spotlight' })
      }
    }
    window.addEventListener('hashchange', handleHashChange)
    return () => window.removeEventListener('hashchange', handleHashChange)
  }, [])

  useEffect(() => {
    if (route.type !== 'spotlight') return

    let isMounted = true
    const initializeShortcuts = async () => {
      if (isMounted) {
        try {
          await new Promise((resolve) => setTimeout(resolve, 2000))
          await globalShortcutManager.initialize()
        } catch (error) {
          console.error('Failed to initialize global shortcuts:', error)
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
  }, [route.type])

  if (route.type === 'tool') {
    return <ToolWindow toolId={route.toolId} />
  }

  return (
    <main className="spotlight-main">
      <SpotlightSearch />
    </main>
  )
}

export default App
