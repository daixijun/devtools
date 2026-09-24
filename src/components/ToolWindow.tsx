import React, { Suspense, useEffect } from 'react'
import { getToolById } from '../tools/registry'
import { useTheme } from '../hooks/useTheme'

import './ToolWindow.css'

interface ToolWindowProps {
  toolId: string
}

const ToolWindow: React.FC<ToolWindowProps> = ({ toolId }) => {
  const tool = getToolById(toolId)

  useTheme()

  useEffect(() => {
    if (tool) {
      document.title = `${tool.name} - DevTools`
    }
  }, [tool])

  if (!tool) {
    return (
      <div className="tool-error">
        <div className="tool-error-icon">
          <svg
            width="48"
            height="48"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          >
            <circle cx="12" cy="12" r="10" />
            <line x1="15" y1="9" x2="9" y2="15" />
            <line x1="9" y1="9" x2="15" y2="15" />
          </svg>
        </div>
        <h2>工具未找到</h2>
        <p>
          无法找到 ID 为 &quot;{toolId}&quot; 的工具
        </p>
      </div>
    )
  }

  return (
    <div className="tool-window-container">
      <Suspense
        fallback={
          <div className="tool-loading">
            <div className="loading-spinner" />
            <p>加载中...</p>
          </div>
        }
      >
        <tool.component />
      </Suspense>
    </div>
  )
}

export default ToolWindow
