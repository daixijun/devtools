import React, { useEffect, useState } from 'react'
import { Button } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard, useDebounce } from '../hooks'
import { errorUtils, validators } from '../utils'

/**
 * Base64 编解码综合工具
 * 整合编码和解码功能，提供统一的用户体验
 */
const Base64Converter: React.FC = () => {
  const [input, setInput] = useState('')
  const [output, setOutput] = useState('')
  const [mode, setMode] = useState<'encode' | 'decode'>('encode')
  const [error, setError] = useState('')
  const { copy, copied } = useCopyToClipboard()

  // 使用防抖处理，避免频繁的编解码操作
  const debouncedInput = useDebounce(input, 200)

  useEffect(() => {
    if (!debouncedInput) {
      setOutput('')
      setError('')
      return
    }

    try {
      if (mode === 'encode') {
        // Base64编码逻辑
        const encoded = btoa(unescape(encodeURIComponent(debouncedInput)))
        setOutput(encoded)
        setError('')
      } else {
        // 验证Base64格式
        if (!validators.isValidBase64(debouncedInput)) {
          throw new Error('输入的不是有效的Base64格式')
        }

        // Base64解码逻辑
        const decoded = decodeURIComponent(escape(atob(debouncedInput)))
        setOutput(decoded)
        setError('')
      }
    } catch (err) {
      setOutput('')
      setError(
        errorUtils.formatError(
          err,
          mode === 'encode' ? 'Base64编码失败' : 'Base64解码失败',
        ),
      )
    }
  }, [debouncedInput, mode])

  const handleCopyOutput = async () => {
    if (output) {
      await copy(output)
    }
  }

  const handleClearInput = () => {
    setInput('')
  }

  const handleSwap = () => {
    // 交换输入和输出
    setInput(output)
    setMode(mode === 'encode' ? 'decode' : 'encode')
  }

  const handleLoadExample = () => {
    const exampleText = 'Hello, World! 你好，世界！'
    if (mode === 'encode') {
      setInput(exampleText)
    } else {
      setInput(btoa(exampleText))
    }
  }

  const getPlaceholder = () => {
    if (mode === 'encode') {
      return '请输入要编码的文本...'
    }
    return '请输入要解码的Base64文本...'
  }

  const getOutputPlaceholder = () => {
    if (error) {
      return error
    }
    return mode === 'encode'
      ? '编码结果将在这里显示...'
      : '解码结果将在这里显示...'
  }

  const getModeTitle = () => {
    return mode === 'encode' ? 'Base64 编码' : 'Base64 解码'
  }

  const getModeSubtitle = () => {
    return mode === 'encode'
      ? '将文本编码为Base64格式'
      : '将Base64格式解码为普通文本'
  }

  return (
    <ToolLayout title={getModeTitle()} subtitle={getModeSubtitle()}>
      <div className='flex flex-col h-full'>
        {/* 模式切换区域 */}
        <div className='flex-shrink-0 p-4 border-b border-gray-200 dark:border-gray-700'>
          <div className='flex items-center justify-between'>
            <div className='flex items-center space-x-4'>
              <div className='flex items-center space-x-2'>
                <button
                  className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${
                    mode === 'encode'
                      ? 'bg-blue-500 text-white'
                      : 'bg-gray-200 text-gray-700 dark:bg-gray-600 dark:text-gray-300'
                  }`}
                  onClick={() => setMode('encode')}>
                  编码
                </button>
                <button
                  className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${
                    mode === 'decode'
                      ? 'bg-blue-500 text-white'
                      : 'bg-gray-200 text-gray-700 dark:bg-gray-600 dark:text-gray-300'
                  }`}
                  onClick={() => setMode('decode')}>
                  解码
                </button>
              </div>
            </div>
            <div className='flex items-center space-x-2'>
              <Button variant='secondary' size='sm' onClick={handleLoadExample}>
                示例
              </Button>
              <Button
                variant='secondary'
                size='sm'
                onClick={handleSwap}
                disabled={!output || !!error}>
                交换输入/输出
              </Button>
            </div>
          </div>
        </div>

        {/* 输入区域 */}
        <div className='flex-1 flex flex-col p-4'>
          <div className='mb-2 flex items-center justify-between'>
            <div className='text-sm text-gray-600 dark:text-gray-400'>
              输入长度: {input.length}
            </div>
            <Button
              variant='secondary'
              size='sm'
              onClick={handleClearInput}
              disabled={!input}>
              清空
            </Button>
          </div>
          <textarea
            className='w-full h-full resize-none border border-gray-300 dark:border-gray-600 rounded-md p-3 text-sm font-mono bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500'
            placeholder={getPlaceholder()}
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
            placeholder={getOutputPlaceholder()}
            value={output}
            readOnly
          />
        </div>
      </div>
    </ToolLayout>
  )
}

export default Base64Converter
