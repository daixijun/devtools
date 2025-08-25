import React from 'react'

interface ErrorMessageProps {
  message: string | null
  className?: string
}

const ErrorMessage: React.FC<ErrorMessageProps> = ({ message, className = '' }) => {
  if (!message) return null

  return (
    <div className={`mt-4 p-3 bg-red-100 text-red-700 rounded-md dark:bg-red-900 dark:text-red-100 ${className}`}>
      {message}
    </div>
  )
}

export default ErrorMessage