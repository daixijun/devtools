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
 * 通用卡片组件
 * 用于包装内容区域，提供统一的视觉风格
 */
const Card: React.FC<CardProps> = ({
  title,
  children,
  className = '',
  padding = 'md',
  shadow = 'sm',
  border = true,
  actions,
  onClick,
  hover = false
}) => {
  const baseClasses = 'bg-white dark:bg-gray-800 rounded-lg transition-all duration-200'
  
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
  
  const borderClass = border ? 'border border-gray-200 dark:border-gray-700' : ''
  const hoverClass = hover ? 'hover:shadow-lg hover:border-gray-300 dark:hover:border-gray-600' : ''
  const cursorClass = onClick ? 'cursor-pointer' : ''

  const handleClick = () => {
    if (onClick) {
      onClick()
    }
  }

  return (
    <div
      className={`${baseClasses} ${shadowClasses[shadow]} ${borderClass} ${hoverClass} ${cursorClass} ${className}`}
      onClick={handleClick}
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
    >
      {/* Card Header */}
      {(title || actions) && (
        <div className={`${paddingClasses[padding]} pb-0 ${padding !== 'none' ? 'pb-0' : ''}`}>
          <div className="flex items-center justify-between mb-4">
            {title && (
              <h3 className="text-lg font-medium text-gray-900 dark:text-white">
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