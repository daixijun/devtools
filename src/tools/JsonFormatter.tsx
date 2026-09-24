import React, { useState } from 'react'
import { Button, CodeEditor } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'
import { errorUtils, validators } from '../utils'

/**
 * JSON 格式化工具
 * 使用重构后的公共组件，提供统一的用户体验
 */
const JsonFormatter: React.FC = () => {
  const [input, setInput] = useState('')
  const [error, setError] = useState('')
  const { copy, copied } = useCopyToClipboard()

  const handleFormat = () => {
    if (!input.trim()) return

    try {
      if (!validators.isValidJson(input)) {
        throw new Error('输入的不是有效的JSON格式')
      }

      const parsed = JSON.parse(input)
      setInput(JSON.stringify(parsed, null, 2))
      setError('')
    } catch (err) {
      setError(errorUtils.formatError(err, 'JSON格式化失败'))
    }
  }

  const handleMinify = () => {
    if (!input.trim()) return

    try {
      if (!validators.isValidJson(input)) {
        throw new Error('输入的不是有效的JSON格式')
      }

      const parsed = JSON.parse(input)
      setInput(JSON.stringify(parsed))
      setError('')
    } catch (err) {
      setError(errorUtils.formatError(err, 'JSON压缩失败'))
    }
  }

  const handleCopy = async () => {
    if (input) {
      await copy(input)
    }
  }

  const handleClear = () => {
    setInput('')
    setError('')
  }

  const handleUnescape = () => {
    if (!input.trim()) return

    try {
      const parsed = JSON.parse(input)
      if (typeof parsed === 'string') {
        const unescaped = JSON.parse(parsed)
        setInput(JSON.stringify(unescaped))
      } else {
        setInput(JSON.stringify(parsed, null, 2))
      }
      setError('')
    } catch (err) {
      try {
        const unescaped = input.replace(/\\"/g, '"').replace(/\\\\/g, '\\')
        setInput(unescaped)
        setError('')
      } catch (innerErr) {
        setError(errorUtils.formatError(innerErr, '去除转义失败'))
      }
    }
  }

  const handleLoadExample = () => {
    const exampleJson = {
      name: '张三',
      age: 30,
      city: '北京',
      hobbies: ['读书', '电影', '旅游'],
      address: {
        street: '朝阳区某某路',
        zipcode: '100000',
      },
      active: true,
    }
    setInput(JSON.stringify(exampleJson))
    setError('')
  }

  return (
    <ToolLayout
      title='JSON 格式化器'
      subtitle='格式化和美化JSON数据，提供语法验证和错误检测'>
      <div className='flex flex-col h-full'>
        {error && (
          <div className='flex-shrink-0 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg mb-4'>
            <p className='text-red-700 dark:text-red-400 text-sm'>{error}</p>
          </div>
        )}

        <div className='p-2 bg-slate-100 dark:bg-slate-700 border-b dark:border-slate-600 flex items-center justify-between flex-shrink-0'>
          <div className='flex items-center space-x-2'>
            <Button
              variant='secondary'
              size='sm'
              onClick={handleFormat}
              disabled={!input}>
              格式化
            </Button>
            <Button
              variant='secondary'
              size='sm'
              onClick={handleMinify}
              disabled={!input}>
              压缩
            </Button>
            <div className='w-px h-5 bg-slate-300 dark:bg-slate-500 mx-1' />
            <Button
              variant='secondary'
              size='sm'
              onClick={handleLoadExample}>
              示例
            </Button>
            <Button
              variant='secondary'
              size='sm'
              onClick={handleUnescape}
              disabled={!input}>
              去除转义
            </Button>
            <Button
              variant='secondary'
              size='sm'
              onClick={handleClear}
              disabled={!input}>
              清空
            </Button>
          </div>
          <div className='flex items-center space-x-2'>
            <div className='text-sm text-slate-600 dark:text-slate-400'>
              长度: {input.length}
            </div>
            <Button
              variant={copied ? 'success' : 'secondary'}
              size='sm'
              onClick={handleCopy}
              disabled={!input}>
              {copied ? '已复制 ✓' : '复制'}
            </Button>
          </div>
        </div>

        <div className='flex-1 min-h-0'>
          <CodeEditor
            language='json'
            value={input}
            onChange={setInput}
            options={{
              minimap: { enabled: false },
              wordWrap: 'off',
            }}
          />
        </div>
      </div>
    </ToolLayout>
  )
}

export default JsonFormatter
