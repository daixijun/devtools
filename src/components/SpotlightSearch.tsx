import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { LogicalSize } from '@tauri-apps/api/dpi'
import { toolCategories, searchTools, ToolDefinition } from '../tools/registry'
import { useToolWindow } from '../hooks/useToolWindow'
import { useTheme } from '../hooks/useTheme'

import './SpotlightSearch.css'

const HEADER_FALLBACK_HEIGHT = 60
const MAX_LIST_HEIGHT = 440
const WINDOW_WIDTH = 680

const SpotlightSearch: React.FC = () => {
  const [query, setQuery] = useState('')
  const [showAll, setShowAll] = useState(false)
  const [isListVisible, setIsListVisible] = useState(false)
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [isClosing, setIsClosing] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const resultsRef = useRef<HTMLDivElement>(null)
  const headerRef = useRef<HTMLDivElement>(null)
  const resizeRafRef = useRef<number | null>(null)
  const { openTool } = useToolWindow()

  useTheme()

  useEffect(() => {
    return () => {
      if (resizeRafRef.current !== null) {
        cancelAnimationFrame(resizeRafRef.current)
      }
    }
  }, [])

  // 淡出动画结束后隐藏窗口(用 hide 而非 close:不依赖托盘配置,也不销毁窗口,
  // 保证快捷键/托盘的再次呼出路径都能正常工作)
  const handleCloseTransitionEnd = useCallback(
    (e: React.TransitionEvent<HTMLDivElement>) => {
      // 仅响应容器自身的 opacity 过渡(忽略子元素冒泡)
      if (e.target !== e.currentTarget || e.propertyName !== 'opacity') return
      if (!isClosing) return
      const appWindow = getCurrentWebviewWindow()
      void appWindow.hide()
      // 同步重置:opacity 在窗口隐藏期间过渡回 1,用户不可见,下次呼出即为正常状态
      setIsClosing(false)
    },
    [isClosing],
  )

  const filteredTools = useMemo(() => {
    if (showAll && !query.trim()) return searchTools('')
    if (!query.trim()) return []
    return searchTools(query)
  }, [query, showAll])

  const flatIndexMap = useMemo(() => {
    const map: { tool: ToolDefinition; globalIndex: number }[] = []
    toolCategories.forEach((category) => {
      const categoryTools = filteredTools.filter((t) => t.category === category.id)
      categoryTools.forEach((tool) => {
        map.push({ tool, globalIndex: map.length })
      })
    })
    return map
  }, [filteredTools])

  const resizeWindow = useCallback(
    async (listVisible: boolean) => {
      const appWindow = getCurrentWebviewWindow()
      // 收起前取消任何挂起的展开 rAF,避免其在收起后把窗口重新撑高
      if (resizeRafRef.current !== null) {
        cancelAnimationFrame(resizeRafRef.current)
        resizeRafRef.current = null
      }
      const headerHeight =
        headerRef.current?.offsetHeight ?? HEADER_FALLBACK_HEIGHT
      if (!listVisible) {
        await appWindow.setSize(new LogicalSize(WINDOW_WIDTH, headerHeight))
        return
      }

      resizeRafRef.current = requestAnimationFrame(async () => {
        resizeRafRef.current = null
        if (!resultsRef.current) return
        const measuredHeight = resultsRef.current.scrollHeight
        const listHeight = Math.min(measuredHeight, MAX_LIST_HEIGHT)
        const totalHeight = headerHeight + listHeight
        await appWindow.setSize(new LogicalSize(WINDOW_WIDTH, totalHeight))
      })
    },
    [],
  )

  useEffect(() => {
    const shouldShow = showAll || query.trim().length > 0
    setIsListVisible(shouldShow)
    if (!shouldShow) setSelectedIndex(0)
  }, [showAll, query])

  useEffect(() => {
    resizeWindow(isListVisible)
  }, [isListVisible, filteredTools, resizeWindow])

  // Bug 1: 窗口打开/获得焦点时自动聚焦输入框
  useEffect(() => {
    // 首次挂载立即聚焦
    inputRef.current?.focus()

    let unlisten: (() => void) | undefined
    const setupFocusListener = async () => {
      try {
        const appWindow = getCurrentWebviewWindow()
        unlisten = await appWindow.onFocusChanged(({ payload: focused }) => {
          if (focused) {
            // 窗口每次获得焦点(如快捷键呼出)时重新聚焦输入框
            setTimeout(() => inputRef.current?.focus(), 0)
          }
        })
      } catch (error) {
        console.warn('Failed to setup focus listener:', error)
      }
    }
    setupFocusListener()

    return () => {
      unlisten?.()
    }
  }, [])

  const handleSelect = useCallback(
    async (tool: ToolDefinition) => {
      await openTool(tool.id)
      setQuery('')
      setShowAll(false)
      inputRef.current?.focus()
    },
    [openTool],
  )

  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setQuery(e.target.value)
    setSelectedIndex(0)
  }, [])

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (query.trim()) {
          setQuery('')
          return
        }
        if (showAll) {
          setShowAll(false)
          return
        }
        // 淡出后再关闭
        if (isClosing) return
        setIsClosing(true)
        return
      }

      if (e.key === 'ArrowDown') {
        e.preventDefault()
        setSelectedIndex((prev) => {
          const next = prev + 1
          return next >= filteredTools.length ? 0 : next
        })
        return
      }

      if (e.key === 'ArrowUp') {
        e.preventDefault()
        setSelectedIndex((prev) => {
          const next = prev - 1
          return next < 0 ? filteredTools.length - 1 : next
        })
        return
      }

      if (e.key === 'Enter') {
        const selectedTool = filteredTools[selectedIndex]
        if (selectedTool) {
          handleSelect(selectedTool)
        }
      }
    },
    [filteredTools, selectedIndex, handleSelect, showAll, query, isClosing],
  )

  return (
    <div
      className={`spotlight-container ${isClosing ? 'closing' : ''}`}
      onTransitionEnd={handleCloseTransitionEnd}
    >
      <div
        className="spotlight-header"
        ref={headerRef}
      >
        <div className="search-wrapper">
          <svg className="search-icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
          <input
            ref={inputRef}
            type="text"
            className="search-input"
            placeholder="搜索工具..."
            value={query}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            spellCheck={false}
          />
          <button
            className={`toggle-btn ${showAll ? 'active' : ''}`}
            onClick={() => setShowAll((v) => !v)}
            title="所有工具"
          >
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <rect x="3" y="3" width="7" height="7" rx="1" />
              <rect x="14" y="3" width="7" height="7" rx="1" />
              <rect x="3" y="14" width="7" height="7" rx="1" />
              <rect x="14" y="14" width="7" height="7" rx="1" />
            </svg>
          </button>
          <img src="/devtools-logo.svg" alt="DevTools" className="app-icon" />
        </div>
      </div>

      <div
        className={`results-container ${isListVisible ? 'open' : 'closed'}`}
        ref={resultsRef}
      >
          {filteredTools.length === 0 ? (
            <div className="no-results">
              <div className="no-results-icon">
                <svg
                  width="48"
                  height="48"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                >
                  <circle cx="11" cy="11" r="8" />
                  <line x1="21" y1="21" x2="16.65" y2="16.65" />
                  <line x1="8" y1="11" x2="14" y2="11" />
                </svg>
              </div>
              <p>未找到匹配的工具</p>
              <p className="hint">尝试其他关键词</p>
            </div>
          ) : (
            <div className="tool-list">
              {toolCategories.map((category) => {
                const categoryTools = filteredTools.filter((t) => t.category === category.id)
                if (categoryTools.length === 0) return null

                return (
                  <React.Fragment key={category.id}>
                    <div className="category-header">
                      {React.createElement(category.icon, { className: 'category-icon' })}
                      <span className="category-name">{category.name}</span>
                    </div>
                    {categoryTools.map((tool) => {
                      const globalIndex =
                        flatIndexMap.find((item) => item.tool.id === tool.id)?.globalIndex ?? -1
                      return (
                        <div
                          key={tool.id}
                          className={`tool-item ${selectedIndex === globalIndex ? 'selected' : ''}`}
                          onClick={() => handleSelect(tool)}
                          onMouseEnter={() => setSelectedIndex(globalIndex)}
                        >
                          {React.createElement(tool.icon, { className: 'tool-icon' })}
                          <div className="tool-info">
                            <span className="tool-name">{tool.name}</span>
                            <span className="tool-desc">{tool.description}</span>
                          </div>
                        </div>
                      )
                    })}
                  </React.Fragment>
                )
              })}
            </div>
          )}
        </div>
    </div>
  )
}

export default SpotlightSearch
