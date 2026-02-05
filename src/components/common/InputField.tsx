import React from 'react'

interface InputFieldProps {
  value: string
  onChange: (value: string) => void
  placeholder?: string
  type?: 'text' | 'number' | 'password' | 'email' | 'url'
  disabled?: boolean
  className?: string
  onKeyDown?: (e: React.KeyboardEvent) => void
  label?: string
  error?: string
  success?: string
  id?: string
  name?: string
  required?: boolean
  autoComplete?: string
}

/**
 * InputField - 统一的输入框组件
 *
 * @description
 * 符合设计系统规范的输入框组件,提供统一的视觉和交互体验
 *
 * @features
 * - 统一边框: border-slate-300 dark:border-slate-600
 * - 统一圆角: rounded-lg
 * - 统一内边距: px-3 py-2
 * - Focus: ring-2 ring-primary-500 border-transparent
 * - 过渡: transition-all duration-200
 * - 支持 error 和 success 状态
 * - 完整的暗色模式支持
 *
 * @accessibility
 * - label 正确关联 input
 * - 支持 aria-describedby
 * - 焦点管理符合 WCAG 2.1 AA
 *
 * @example
 * ```tsx
 * // 基本用法
 * <InputField
 *   label="用户名"
 *   value={username}
 *   onChange={setUsername}
 *   placeholder="请输入用户名"
 * />
 *
 * // 错误状态
 * <InputField
 *   label="邮箱"
 *   value={email}
 *   onChange={setEmail}
 *   error="邮箱格式不正确"
 * />
 *
 * // 密码输入
 * <InputField
 *   label="密码"
 *   type="password"
 *   value={password}
 *   onChange={setPassword}
 *   required
 * />
 * ```
 *
 * @see DESIGN_SYSTEM.md - 设计系统规范
 */
const InputField: React.FC<InputFieldProps> = ({
  value,
  onChange,
  placeholder = '',
  type = 'text',
  disabled = false,
  className = '',
  onKeyDown,
  label,
  error,
  success,
  id: externalId,
  name,
  required,
  autoComplete,
}) => {
  // 使用 useId 生成唯一 ID,或使用外部提供的 ID
  const generatedId = React.useId()
  const inputId = externalId || generatedId
  const errorId = `${inputId}-error`
  const successId = `${inputId}-success`

  // 基础样式 - 统一使用 rounded-lg, duration-200
  const baseClasses =
    'w-full px-3 py-2 border rounded-lg ' +
    'transition-all duration-200 cursor-pointer ' +
    'focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent ' +
    'disabled:opacity-50 disabled:cursor-not-allowed'

  // 背景和文本颜色
  const colorClasses =
    'bg-white dark:bg-slate-800 ' +
    'text-slate-900 dark:text-slate-100 ' +
    'placeholder:text-slate-400 dark:placeholder:text-slate-500'

  // 边框和状态颜色
  const borderClasses = error
    ? 'border-red-500 focus:ring-red-500'
    : success
    ? 'border-green-500 focus:ring-green-500'
    : 'border-slate-300 dark:border-slate-600'

  // 禁用状态背景
  const disabledBg = disabled ? 'bg-slate-50 dark:bg-slate-900' : ''

  return (
    <div className={className}>
      {/* Label */}
      {label && (
        <label
          htmlFor={inputId}
          className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
          {label}
          {required && <span className='text-red-500 ml-1'>*</span>}
        </label>
      )}

      {/* Input */}
      <input
        id={inputId}
        name={name}
        type={type}
        className={`${baseClasses} ${colorClasses} ${borderClasses} ${disabledBg}`.trim()}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        onKeyDown={onKeyDown}
        required={required}
        autoComplete={autoComplete}
        aria-invalid={!!error}
        aria-describedby={
          error ? errorId : success ? successId : undefined
        }
      />

      {/* Error Message */}
      {error && (
        <div
          id={errorId}
          className='text-red-600 dark:text-red-400 text-sm mt-1 flex items-start space-x-1'
          role='alert'>
          <span className='flex-shrink-0'>❌</span>
          <span>{error}</span>
        </div>
      )}

      {/* Success Message */}
      {success && !error && (
        <div
          id={successId}
          className='text-green-600 dark:text-green-400 text-sm mt-1 flex items-start space-x-1'
          role='status'>
          <span className='flex-shrink-0'>✓</span>
          <span>{success}</span>
        </div>
      )}
    </div>
  )
}

export default InputField
