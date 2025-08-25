import React, { useEffect, useState } from 'react'
import { Button } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard, useDebounce } from '../hooks'
import { errorUtils, validators } from '../utils'

/**
 * Base64 解码工具
 * 使用重构后的公共组件，提供统一的用户体验
 */
const Base64Decode: React.FC = () => {
  const [input, setInput] = useState('')
  const [output, setOutput] = useState('')
  const [error, setError] = useState('')
  const { copy, copied } = useCopyToClipboard()

  // 使用防抖处理，避免频繁的解码操作
  const debouncedInput = useDebounce(input, 200)

  useEffect(() => {
    if (!debouncedInput) {
      setOutput('')
      setError('')
      return
    }

    try {
      // 验证Base64格式
      if (!validators.isValidBase64(debouncedInput)) {
        throw new Error('输入的不是有效的Base64格式')
      }

      // Base64解码逻辑
      const decoded = decodeURIComponent(escape(atob(debouncedInput)))
      setOutput(decoded)
      setError('')
    } catch (err) {
      setOutput('')
      setError(errorUtils.formatError(err, 'Base64解码失败'))
    }
  }, [debouncedInput])

  const handleCopyOutput = async () => {
    if (output) {
      await copy(output)
    }
  }

  const handleClearInput = () => {
    setInput('')
  }

  const handleLoadExample = () => {
    const exampleBase64 = btoa('Hello, World! 你好，世界！')
    setInput(exampleBase64)
  }

  return (
    <ToolLayout 
      title="Base64 解码"
      subtitle="将Base64格式解码为普通文本"
    >
      <div className='flex flex-col h-full'>
        {/* 输入区域 */}
        <div className='flex-1 flex flex-col p-4'>
          <div className='mb-2 flex items-center justify-between'>
            <div className='text-sm text-gray-600 dark:text-gray-400'>
              输入长度: {input.length}
            </div>
            <div className='flex items-center space-x-2'>
              <Button variant='secondary' size='sm' onClick={handleLoadExample}>
                示例
              </Button>
              <Button
                variant='secondary'
                size='sm'
                onClick={handleClearInput}
                disabled={!input}>
                清空
              </Button>
            </div>
          </div>
          <textarea
            className='w-full h-full resize-none border border-gray-300 dark:border-gray-600 rounded-md p-3 text-sm font-mono bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500'
            placeholder='请输入要解码的Base64文本...'
            value={input}
            onChange={(e) => setInput(e.target.value)}
          />
        </div>

        {/* 输出区域 */}
        <div className='flex-1 flex flex-col p-4'>
          <div className='mb-2 flex items-center justify-between'>
            <div className='text-sm text-gray-600 dark:text-gray-400'>
              输出长度: {output.length}
            </div>
            <Button
              variant='primary'
              size='sm'
              onClick={handleCopyOutput}
              disabled={!output || !!error}
              className={copied ? 'bg-green-600 hover:bg-green-700' : ''}>
              {copied ? '已复制' : '复制结果'}
            </Button>
          </div>
          <textarea
            className='w-full h-full resize-none border border-gray-300 dark:border-gray-600 rounded-md p-3 text-sm font-mono bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none'
            placeholder={error || '解码结果将在这里显示...'}
            value={output}
            readOnly
          />
        </div>

        {/* 错误提示 */}
        {error && (
          <div className='flex-shrink-0 p-4 bg-red-50 dark:bg-red-900/20 border-t border-red-200 dark:border-red-800'>
            <p className='text-red-700 dark:text-red-400 text-sm'>{error}</p>
          </div>
        )}
      </div>
    </ToolLayout>
  )
}

export default Base64Decode
