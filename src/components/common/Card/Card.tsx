import React from 'react'

interface CardProps {
  title?: string
  children: React.ReactNode
  className?: string
  padding?: 'none' | 'sm' | 'md' | 'lg'
  shadow?: 'none' | 'sm' | 'md' | 'lg'
  border?: boolean
  actions?: React.ReactNode
  onClick?: () => void
  hover?: boolean
}

/**
 * Card - 统一的卡片组件
 *
 * @description
 * 符合设计系统规范的卡片组件,提供统一的视觉风格
 *
 * @features
 * - 使用 slate 色系替代 gray
 * - 统一圆角 rounded-lg
 * - 标准过渡 duration-200
 * - 支持玻璃态效果
 * - 完整的暗色模式支持
 *
 * @see DESIGN_SYSTEM.md - 设计系统规范
 */
const Card: React.FC<CardProps> = ({
  title,
  children,
  className = '',
  padding = 'md',
  shadow = 'lg',
  border = true,
  actions,
  onClick,
  hover = false
}) => {
  const baseClasses = 'bg-white/80 dark:bg-slate-800/80 backdrop-blur-md rounded-lg transition-all duration-200'

  const paddingClasses = {
    none: '',
    sm: 'p-3',
    md: 'p-4',
    lg: 'p-6'
  }

  const shadowClasses = {
    none: '',
    sm: 'shadow-sm',
    md: 'shadow-md',
    lg: 'shadow-lg'
  }

  const borderClass = border ? 'border border-slate-200 dark:border-slate-600' : ''
  const hoverClass = hover ? 'hover:shadow-xl hover:border-slate-300 dark:hover:border-slate-500' : ''
  const cursorClass = onClick ? 'cursor-pointer' : ''

  const handleClick = () => {
    if (onClick) {
      onClick()
    }
  }

  return (
    <div
      className={`${baseClasses} ${shadowClasses[shadow]} ${borderClass} ${hoverClass} ${cursorClass} ${className}`.trim()}
      onClick={handleClick}
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
    >
      {/* Card Header */}
      {(title || actions) && (
        <div className={`${paddingClasses[padding]} pb-0 ${padding !== 'none' ? 'pb-0' : ''}`}>
          <div className="flex items-center justify-between mb-4">
            {title && (
              <h3 className="text-lg font-semibold text-slate-900 dark:text-slate-100">
                {title}
              </h3>
            )}
            {actions && (
              <div className="flex items-center space-x-2">
                {actions}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Card Content */}
      <div className={title || actions ? (padding !== 'none' ? `px-${padding === 'sm' ? '3' : padding === 'md' ? '4' : '6'} pb-${padding === 'sm' ? '3' : padding === 'md' ? '4' : '6'}` : '') : paddingClasses[padding]}>
        {children}
      </div>
    </div>
  )
}

export default Card
