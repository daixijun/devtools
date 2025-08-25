import React from 'react'
import { PageHeader } from '../common'

interface ToolLayoutProps {
  title: string
  subtitle?: string
  description?: string
  children: React.ReactNode
  actions?: React.ReactNode
  fullHeight?: boolean
  padding?: boolean
  className?: string
}

/**
 * 工具页面统一布局组件
 * 提供一致的页面头部和内容区域布局
 */
const ToolLayout: React.FC<ToolLayoutProps> = ({
  title,
  subtitle,
  description,
  children,
  actions,
  fullHeight = true,
  padding = true,
  className = ''
}) => {
  return (
    <div className={`flex flex-col ${fullHeight ? 'h-full' : 'min-h-screen'} ${className}`}>
      {/* 页面头部 */}
      <div className="flex-shrink-0 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
        <div className={`${padding ? 'p-4' : 'p-3'}`}>
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <PageHeader title={title} subtitle={subtitle} />
              {description && (
                <p className="mt-1 text-sm text-gray-600 dark:text-gray-400 max-w-3xl">
                  {description}
                </p>
              )}
            </div>
            {actions && (
              <div className="ml-4 flex-shrink-0">
                {actions}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* 页面内容区域 */}
      <div className={`flex-1 overflow-hidden ${padding ? 'p-4' : ''}`}>
        {children}
      </div>
    </div>
  )
}

export default ToolLayout