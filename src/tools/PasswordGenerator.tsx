import React, { useState, useCallback } from 'react'
import { Button } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'

interface PasswordOptions {
  length: number
  includeUppercase: boolean
  includeLowercase: boolean
  includeNumbers: boolean
  includeSymbols: boolean
  customSymbols: string
}

const defaultOptions: PasswordOptions = {
  length: 16,
  includeUppercase: true,
  includeLowercase: true,
  includeNumbers: true,
  includeSymbols: true,
  customSymbols: '!@#$%^&*()_+-=[]{}|;:,.<>?',
}

const characterSets = {
  uppercase: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
  lowercase: 'abcdefghijklmnopqrstuvwxyz',
  numbers: '0123456789',
}

// 预定义的特殊字符组
const predefinedSymbolGroups = [
  { name: '常用符号', value: '!@#$%^&*()_+-=' },
  { name: '括号符号', value: '[]{}()<>' },
  { name: '标点符号', value: '.,:;?!' },
  { name: '数学符号', value: '+-*/=<>' },
  { name: '货币符号', value: '$€£¥¢' },
  { name: '其他符号', value: '|~`\'"' },
]

/**
 * 密码生成器工具
 * 可以生成指定长度和字符类型的强密码
 */
const PasswordGenerator: React.FC = () => {
  const [password, setPassword] = useState('')
  const [options, setOptions] = useState<PasswordOptions>(defaultOptions)
  const [strength, setStrength] = useState('')
  const { copy, copied } = useCopyToClipboard()

  // 计算密码强度
  const calculateStrength = useCallback((pwd: string): string => {
    if (pwd.length === 0) return ''

    let score = 0

    // 长度评分
    if (pwd.length >= 12) score += 2
    else if (pwd.length >= 8) score += 1

    // 字符类型多样性评分
    const hasUppercase = /[A-Z]/.test(pwd)
    const hasLowercase = /[a-z]/.test(pwd)
    const hasNumbers = /[0-9]/.test(pwd)
    const hasSymbols = /[^A-Za-z0-9]/.test(pwd)

    const typeCount = [hasUppercase, hasLowercase, hasNumbers, hasSymbols].filter(Boolean).length
    score += typeCount

    // 根据总分确定强度
    if (score >= 5) return '非常强'
    if (score >= 4) return '强'
    if (score >= 3) return '中等'
    return '弱'
  }, [])

  // 生成密码
  const generatePassword = useCallback(() => {
    const {
      length,
      includeUppercase,
      includeLowercase,
      includeNumbers,
      includeSymbols,
      customSymbols
    } = options

    // 确保至少选择一种字符类型
    if (!includeUppercase && !includeLowercase && !includeNumbers && !includeSymbols) {
      setPassword('请至少选择一种字符类型')
      setStrength('')
      return
    }

    // 构建可用字符集
    let availableChars = ''
    if (includeUppercase) availableChars += characterSets.uppercase
    if (includeLowercase) availableChars += characterSets.lowercase
    if (includeNumbers) availableChars += characterSets.numbers
    if (includeSymbols) availableChars += customSymbols

    // 生成密码
    const charArray = new Uint32Array(length)
    crypto.getRandomValues(charArray)
    let generatedPassword = ''
    for (let i = 0; i < length; i++) {
      generatedPassword += availableChars[charArray[i] % availableChars.length]
    }

    setPassword(generatedPassword)
    setStrength(calculateStrength(generatedPassword))
  }, [options, calculateStrength])

  // 复制密码
  const handleCopyPassword = async () => {
    if (password && !password.includes('请至少选择一种字符类型')) {
      await copy(password)
    }
  }

  // 更新选项
  const updateOption = (key: keyof PasswordOptions, value: any) => {
    setOptions(prev => ({ ...prev, [key]: value }))
  }

  // 组件挂载时生成初始密码
  React.useEffect(() => {
    generatePassword()
  }, [generatePassword])

  return (
    <ToolLayout
      title="密码生成器"
      subtitle="生成安全的随机密码"
    >
      <div className='flex flex-col h-full space-y-6 overflow-y-auto'>
        {/* 密码显示区域 */}
        <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-600 p-4'>
          <div className='flex items-center justify-between mb-2'>
            <span className='text-sm text-gray-600 dark:text-gray-400'>生成的密码</span>
            <div className='flex items-center space-x-2'>
              {strength && (
                <span className={`text-xs px-2 py-1 rounded-full ${
                  strength === '非常强' ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' :
                  strength === '强' ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200' :
                  strength === '中等' ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200' :
                  'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                }`}>
                  强度: {strength}
                </span>
              )}
              <Button
                variant='primary'
                size='sm'
                onClick={handleCopyPassword}
                disabled={!password || password.includes('请至少选择一种字符类型')}
                className={copied ? 'bg-green-600 hover:bg-green-700' : ''}>
                {copied ? '已复制' : '复制密码'}
              </Button>
            </div>
          </div>
          <div className='bg-gray-50 dark:bg-gray-700 rounded p-3 font-mono text-lg text-center break-all'>
            {password || '点击生成按钮创建密码'}
          </div>
        </div>

        {/* 密码长度设置 */}
        <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-600 p-4'>
          <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2'>
            密码长度: {options.length}
          </label>
          <input
            type='range'
            min='4'
            max='64'
            value={options.length}
            onChange={(e) => updateOption('length', parseInt(e.target.value))}
            className='w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer dark:bg-gray-700'
          />
          <div className='flex justify-between text-xs text-gray-500 dark:text-gray-400 mt-1'>
            <span>4</span>
            <span>64</span>
          </div>
        </div>

        {/* 字符类型控制 */}
        <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-600 p-4'>
          <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-3'>
            字符类型控制
          </label>
          <div className='flex flex-wrap gap-4'>
            {[
              {
                typeKey: 'includeUppercase',
                typeLabel: '大写字母 (A-Z)'
              },
              {
                typeKey: 'includeLowercase',
                typeLabel: '小写字母 (a-z)'
              },
              {
                typeKey: 'includeNumbers',
                typeLabel: '数字 (0-9)'
              },
              {
                typeKey: 'includeSymbols',
                typeLabel: '特殊符号'
              },
            ].map(({ typeKey, typeLabel }) => {
              const isTypeEnabled = options[typeKey as keyof PasswordOptions] as boolean

              return (
                <div key={typeKey} className='flex items-center space-x-2'>
                  <input
                    type='checkbox'
                    id={typeKey}
                    checked={isTypeEnabled}
                    onChange={(e) => updateOption(typeKey as keyof PasswordOptions, e.target.checked)}
                    className='w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600'
                  />
                  <label htmlFor={typeKey} className='text-sm text-gray-700 dark:text-gray-300 whitespace-nowrap'>
                    {typeLabel}
                  </label>
                </div>
              )
            })}
          </div>
        </div>

          {/* 特殊字符自定义选择 */}
          {options.includeSymbols && (
            <div className='mt-4 pt-4 border-t border-gray-200 dark:border-gray-600'>
              <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2'>
                自定义特殊字符
              </label>

              {/* 预定义字符组快速选择 */}
              <div className='mb-3'>
                <label className='block text-xs text-gray-600 dark:text-gray-400 mb-2'>快速选择：</label>
                <div className='flex flex-wrap gap-2'>
                  {predefinedSymbolGroups.map((group, index) => (
                    <button
                      key={index}
                      type='button'
                      onClick={() => updateOption('customSymbols', group.value)}
                      className='px-2 py-1 text-xs bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 rounded border border-gray-300 dark:border-gray-500 transition-colors'
                    >
                      {group.name}
                    </button>
                  ))}
                </div>
              </div>

              {/* 自定义字符输入 */}
              <div>
                <label className='block text-xs text-gray-600 dark:text-gray-400 mb-1'>
                  自定义字符（直接输入）：
                </label>
                <input
                  type='text'
                  value={options.customSymbols}
                  onChange={(e) => updateOption('customSymbols', e.target.value)}
                  placeholder='输入您想要使用的特殊字符...'
                  className='w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500'
                />
                <div className='mt-1 text-xs text-gray-500 dark:text-gray-400'>
                  当前字符数: {options.customSymbols.length}
                </div>
              </div>
            </div>
          )}

        {/* 操作按钮 */}
        <div className='flex space-x-3'>
          <Button
            variant='primary'
            onClick={generatePassword}
            className='flex-1'>
            生成新密码
          </Button>
          <Button
            variant='secondary'
            onClick={() => setOptions(defaultOptions)}
            className='flex-1'>
            重置选项
          </Button>
        </div>

        {/* 密码强度说明 */}
        <div className='bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800 p-4'>
          <h4 className='text-sm font-medium text-blue-800 dark:text-blue-200 mb-2'>密码强度说明</h4>
          <ul className='text-xs text-blue-700 dark:text-blue-300 space-y-1'>
            <li>• 长度 ≥ 12 且包含 4 种字符类型：非常强</li>
            <li>• 长度 ≥ 8 且包含 3-4 种字符类型：强</li>
            <li>• 长度 ≥ 8 且包含 2 种字符类型：中等</li>
            <li>• 其他情况：弱</li>
          </ul>
        </div>
      </div>
    </ToolLayout>
  )
}

export default PasswordGenerator