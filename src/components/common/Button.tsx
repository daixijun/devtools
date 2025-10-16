import React from 'react'

interface ButtonProps {
  onClick: () => void
  children: React.ReactNode
  loading?: boolean
  disabled?: boolean
  variant?: 'primary' | 'secondary' | 'danger' | 'success'
  size?: 'sm' | 'md' | 'lg'
  className?: string
  type?: 'button' | 'submit'
  icon?: React.ReactNode
  iconPosition?: 'left' | 'right'
}

/**
 * 通用按钮组件
 * 支持多种样式、尺寸和状态
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
}) => {
  const baseClasses =
    'inline-flex items-center justify-center rounded-md font-medium focus:outline-none focus:ring-2 transition-all duration-200 disabled:cursor-not-allowed'

  const sizeClasses = {
    sm: 'px-3 py-1.5 text-sm',
    md: 'px-4 py-2 text-sm',
    lg: 'px-6 py-3 text-base',
  }

  const variantClasses = {
    primary:
      'bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500 disabled:opacity-50 disabled:hover:bg-blue-600 shadow-sm',
    secondary:
      'bg-gray-200 text-gray-700 hover:bg-gray-300 focus:ring-gray-500 disabled:opacity-50 disabled:hover:bg-gray-200 dark:bg-gray-700 dark:text-white dark:hover:bg-gray-600 dark:disabled:hover:bg-gray-700 border border-gray-300 dark:border-gray-600',
    danger:
      'bg-red-600 text-white hover:bg-red-700 focus:ring-red-500 disabled:opacity-50 disabled:hover:bg-red-600 shadow-sm',
    success:
      'bg-green-600 text-white hover:bg-green-700 focus:ring-green-500 disabled:opacity-50 disabled:hover:bg-green-600 shadow-sm',
  }

  const renderContent = () => {
    if (loading) {
      return (
        <>
          <svg
            className='animate-spin -ml-1 mr-2 h-4 w-4 text-current'
            fill='none'
            viewBox='0 0 24 24'>
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
        }`}>
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
      className={`${baseClasses} ${sizeClasses[size]} ${variantClasses[variant]} ${className}`}
      onClick={onClick}
      disabled={disabled || loading}>
      {renderContent()}
    </button>
  )
}

export default Button
