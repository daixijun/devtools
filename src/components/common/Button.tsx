import React from 'react'

interface ButtonProps {
  onClick?: () => void
  children: React.ReactNode
  loading?: boolean
  disabled?: boolean
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'success'
  size?: 'sm' | 'md' | 'lg'
  className?: string
  type?: 'button' | 'submit'
  icon?: React.ReactNode
  iconPosition?: 'left' | 'right'
  'aria-label'?: string
}

/**
 * Button - 统一的按钮组件
 *
 * @description
 * 符合设计系统规范的按钮组件,提供统一的视觉和交互体验
 *
 * @features
 * - 使用 Slate 色系替代 gray
 * - 统一圆角 rounded-lg
 * - 标准过渡 duration-200
 * - 完整的 focus ring
 * - 支持 loading 和 disabled 状态
 * - 支持 icon 和不同位置
 *
 * @accessibility
 * - 完整键盘导航支持
 * - 焦点管理符合 WCAG 2.1 AA
 * - 支持自定义 aria-label
 *
 * @example
 * ```tsx
 * // Primary 按钮
 * <Button variant="primary" size="md" onClick={handleClick}>
 *   确定
 * </Button>
 *
 * // 带 icon 的按钮
 * <Button variant="secondary" icon={<Icon />} iconPosition="left">
 *   返回
 * </Button>
 *
 * // Loading 按钮
 * <Button variant="primary" loading={isLoading}>
 *   提交
 * </Button>
 * ```
 *
 * @see DESIGN_SYSTEM.md - 设计系统规范
 */
const Button: React.FC<ButtonProps> = ({
  onClick,
  children,
  loading = false,
  disabled = false,
  variant = 'primary',
  size = 'md',
  className = '',
  type = 'button',
  icon,
  iconPosition = 'left',
  'aria-label': ariaLabel,
}) => {
  // 基础样式 - 统一使用 rounded-lg, duration-200
  const baseClasses =
    'inline-flex items-center justify-center rounded-lg font-medium ' +
    'transition-all duration-200 cursor-pointer ' +
    'focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 ' +
    'disabled:opacity-50 disabled:cursor-not-allowed ' +
    'active:scale-[0.98]'

  // 尺寸样式
  const sizeClasses = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-base',
    lg: 'px-6 py-3 text-lg',
  }

  // 变体样式 - 使用 primary-500/600 和 slate 色系
  const variantClasses = {
    primary:
      'bg-primary-500 text-white hover:bg-primary-600 ' +
      'disabled:bg-primary-500 disabled:hover:bg-primary-500 ' +
      'shadow-md',
    secondary:
      'bg-slate-100 text-slate-700 hover:bg-slate-200 ' +
      'dark:bg-slate-700 dark:text-slate-200 dark:hover:bg-slate-600 ' +
      'disabled:bg-slate-100 disabled:hover:bg-slate-100 ' +
      'dark:disabled:bg-slate-700 dark:disabled:hover:bg-slate-700',
    ghost:
      'bg-transparent text-slate-700 hover:bg-slate-100 ' +
      'dark:text-slate-300 dark:hover:bg-slate-800 ' +
      'disabled:bg-transparent disabled:hover:bg-transparent',
    danger:
      'bg-red-500 text-white hover:bg-red-600 ' +
      'disabled:bg-red-500 disabled:hover:bg-red-500 ' +
      'shadow-md',
    success:
      'bg-green-100 text-green-700 hover:bg-green-200 ' +
      'dark:bg-green-900 dark:text-green-200 dark:hover:bg-green-800 ' +
      'border border-green-300 dark:border-green-700 ' +
      'disabled:bg-green-100 disabled:hover:bg-green-100 ' +
      'dark:disabled:bg-green-900 dark:disabled:hover:bg-green-900',
  }

  // 渲染内容 (loading 或 children + icon)
  const renderContent = () => {
    if (loading) {
      return (
        <>
          <svg
            className='animate-spin -ml-1 mr-2 h-4 w-4 text-current'
            fill='none'
            viewBox='0 0 24 24'
            aria-hidden='true'>
            <circle
              className='opacity-25'
              cx='12'
              cy='12'
              r='10'
              stroke='currentColor'
              strokeWidth='4'></circle>
            <path
              className='opacity-75'
              fill='currentColor'
              d='M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z'></path>
          </svg>
          加载中...
        </>
      )
    }

    const iconElement = icon && (
      <span
        className={`${
          children ? (iconPosition === 'left' ? 'mr-2' : 'ml-2') : ''
        }`}
        aria-hidden='true'>
        {icon}
      </span>
    )

    return (
      <>
        {iconPosition === 'left' && iconElement}
        {children}
        {iconPosition === 'right' && iconElement}
      </>
    )
  }

  return (
    <button
      type={type}
      className={`${baseClasses} ${sizeClasses[size]} ${variantClasses[variant]} ${className}`.trim()}
      onClick={onClick}
      disabled={disabled || loading}
      aria-label={ariaLabel}
      aria-busy={loading}>
      {renderContent()}
    </button>
  )
}

export default Button
