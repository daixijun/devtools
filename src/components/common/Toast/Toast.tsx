import React from 'react'

export interface ToastProps {
  message: string
  show: boolean
  onClose?: () => void
  type?: 'success' | 'error' | 'warning' | 'info'
  duration?: number
  position?: 'top-right' | 'bottom-right' | 'top-center' | 'bottom-center'
}

/**
 * 统一的Toast通知组件
 * 用于显示临时消息提示
 */
export const Toast: React.FC<ToastProps> = ({
  message,
  show,
  onClose,
  type = 'info',
  position = 'bottom-right',
}) => {
  const getTypeStyles = () => {
    const baseStyles =
      'px-4 py-3 rounded-lg shadow-lg text-sm font-medium transition-all duration-300 max-w-sm'

    switch (type) {
      case 'success':
        return `${baseStyles} bg-green-600 text-white border border-green-700`
      case 'error':
        return `${baseStyles} bg-red-600 text-white border border-red-700`
      case 'warning':
        return `${baseStyles} bg-yellow-600 text-white border border-yellow-700`
      case 'info':
      default:
        return `${baseStyles} bg-gray-800 text-white border border-gray-700 dark:bg-gray-100 dark:text-gray-900 dark:border-gray-200`
    }
  }

  const getPositionStyles = () => {
    const basePosition = 'fixed z-50'

    switch (position) {
      case 'top-right':
        return `${basePosition} top-6 right-6`
      case 'bottom-right':
        return `${basePosition} bottom-6 right-6`
      case 'top-center':
        return `${basePosition} top-6 left-1/2 transform -translate-x-1/2`
      case 'bottom-center':
        return `${basePosition} bottom-6 left-1/2 transform -translate-x-1/2`
      default:
        return `${basePosition} bottom-6 right-6`
    }
  }

  if (!show) {
    return null
  }

  return (
    <div
      className={`${getPositionStyles()} ${
        show ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-2'
      }`}>
      <div className={getTypeStyles()}>
        <div className='flex items-center justify-between'>
          <span className='flex-1'>{message}</span>
          {onClose && (
            <button
              onClick={onClose}
              className='ml-3 text-current opacity-70 hover:opacity-100 transition-opacity'
              aria-label='关闭'>
              <svg
                className='w-4 h-4'
                fill='none'
                stroke='currentColor'
                viewBox='0 0 24 24'>
                <path
                  strokeLinecap='round'
                  strokeLinejoin='round'
                  strokeWidth={2}
                  d='M6 18L18 6M6 6l12 12'
                />
              </svg>
            </button>
          )}
        </div>
      </div>
    </div>
  )
}

export default Toast
