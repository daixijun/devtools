import React from 'react'

interface ErrorMessageProps {
  message: string | null
  className?: string
}

/**
 * ErrorMessage - 统一的错误提示组件
 *
 * @description
 * 符合设计系统规范的错误提示组件
 *
 * @features
 * - 统一使用 slate 色系
 * - 统一圆角 rounded-lg
 * - 标准过渡 duration-200
 * - 完整的暗色模式支持
 *
 * @see DESIGN_SYSTEM.md - 设计系统规范
 */
const ErrorMessage: React.FC<ErrorMessageProps> = ({ message, className = '' }) => {
  if (!message) return null

  return (
    <div
      className={`mt-4 p-3 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400
      rounded-lg border border-red-200 dark:border-red-800
      transition-all duration-200 flex items-start space-x-2 ${className}`}
      role='alert'>
      <span className='flex-shrink-0'>❌</span>
      <span className='text-sm'>{message}</span>
    </div>
  )
}

export default ErrorMessage
