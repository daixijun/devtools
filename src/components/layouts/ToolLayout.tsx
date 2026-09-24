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
 * 使用现代设计系统: 玻璃态效果、平滑过渡、高对比度
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
    <div className={`flex flex-col ${fullHeight ? 'h-full' : 'min-h-screen'} bg-white dark:bg-slate-900 ${className}`}>
      {/* 页面头部 - 玻璃态效果 */}
      <div className="flex-shrink-0 border-b border-slate-200 dark:border-slate-700 bg-white/60 dark:bg-slate-800/60 backdrop-blur-sm transition-colors duration-200">
        <div className={`${padding ? 'p-4' : 'p-3'}`}>
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <PageHeader title={title} subtitle={subtitle} />
              {description && (
                <p className="mt-1 text-sm text-slate-600 dark:text-slate-400 max-w-3xl leading-relaxed">
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
      <div className={`flex-1 overflow-y-auto bg-white dark:bg-slate-900 ${padding ? 'p-4' : ''} transition-colors duration-200`}>
        {children}
      </div>
    </div>
  )
}

export default ToolLayout