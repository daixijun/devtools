import React from 'react'

interface InputFieldProps {
  value: string
  onChange: (value: string) => void
  placeholder?: string
  type?: 'text' | 'number' | 'password' | 'email'
  disabled?: boolean
  className?: string
  onKeyDown?: (e: React.KeyboardEvent) => void
  label?: string
  error?: string
}

const InputField: React.FC<InputFieldProps> = ({
  value,
  onChange,
  placeholder = '',
  type = 'text',
  disabled = false,
  className = '',
  onKeyDown,
  label,
  error
}) => {
  const inputId = React.useId()

  return (
    <div className={className}>
      {label && (
        <label htmlFor={inputId} className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2'>
          {label}
        </label>
      )}
      <input
        id={inputId}
        type={type}
        className={`w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 transition-colors ${
          error 
            ? 'border-red-300 focus:border-red-500 focus:ring-red-500' 
            : 'border-gray-300 focus:border-blue-500 dark:border-gray-600'
        } ${disabled ? 'bg-gray-50 cursor-not-allowed' : 'bg-white dark:bg-gray-700'} dark:text-white`}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        onKeyDown={onKeyDown}
      />
      {error && (
        <div className='text-red-500 text-sm mt-1'>{error}</div>
      )}
    </div>
  )
}

export default InputField