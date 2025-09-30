import React, { useEffect, useState } from 'react'
import Split from 'react-split'
import { Button, CodeEditor } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard, useDebounce } from '../hooks'
import { errorUtils, validators } from '../utils'

/**
 * JSON 格式化工具
 * 使用重构后的公共组件，提供统一的用户体验
 */
const JsonFormatter: React.FC = () => {
  const [input, setInput] = useState('')
  const [output, setOutput] = useState('')
  const [error, setError] = useState('')
  const { copy, copied } = useCopyToClipboard()

  // 使用防抖处理，避免频繁的JSON解析
  const debouncedInput = useDebounce(input, 300)

  useEffect(() => {
    if (!debouncedInput.trim()) {
      setOutput('')
      setError('')
      return
    }

    try {
      // 验证JSON格式
      if (!validators.isValidJson(debouncedInput)) {
        throw new Error('输入的不是有效的JSON格式')
      }

      // 解析并格式化JSON
      const parsed = JSON.parse(debouncedInput)
      const formatted = JSON.stringify(parsed, null, 2)
      setOutput(formatted)
      setError('')
    } catch (err) {
      setOutput('')
      setError(errorUtils.formatError(err, 'JSON格式化失败'))
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

  const handleUnescape = () => {
    if (!input.trim()) return

    try {
      // 尝试解析为JSON字符串，去除转义
      const parsed = JSON.parse(input)
      if (typeof parsed === 'string') {
        // 如果是字符串，再解析一次去除转义
        const unescaped = JSON.parse(parsed)
        setInput(JSON.stringify(unescaped))
      } else {
        // 如果不是字符串，直接格式化
        setInput(JSON.stringify(parsed, null, 2))
      }
    } catch (err) {
      // 如果第一次解析失败，尝试直接去除转义字符
      try {
        const unescaped = input.replace(/\\"/g, '"').replace(/\\\\/g, '\\')
        setInput(unescaped)
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
  }

  return (
    <ToolLayout
      title='JSON 格式化器'
      subtitle='格式化和美化JSON数据，提供语法验证和错误检测'>
      <div className='flex flex-col h-full'>
        {/* 错误提示 */}
        {error && (
          <div className='flex-shrink-0 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg mb-4'>
            <p className='text-red-700 dark:text-red-400 text-sm'>{error}</p>
          </div>
        )}

        {/* 分屏编辑区域 */}
        <div className='flex-1 min-h-0'>
          <Split
            sizes={[50, 50]}
            minSize={200}
            expandToMin={true}
            gutterSize={10}
            gutterAlign='center'
            snapOffset={30}
            dragInterval={1}
            direction='horizontal'
            cursor='col-resize'
            className='flex flex-row gap-4 h-full'>
            {/* 左侧输入区域 */}
            <div className='flex flex-col h-full'>
              <div className='p-2 bg-gray-100 dark:bg-gray-700 border-b dark:border-gray-600 flex items-center justify-between flex-shrink-0'>
                <h2 className='font-semibold text-gray-800 dark:text-gray-200'>
                  JSON 输入
                </h2>
                <div className='flex items-center space-x-2'>
                  <div className='text-sm text-gray-600 dark:text-gray-400'>
                    输入长度: {input.length}
                  </div>
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
                    onClick={handleClearInput}
                    disabled={!input}>
                    清空
                  </Button>
                </div>
              </div>
              <div className='flex-1 min-h-0 h-full'>
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

            {/* 右侧输出区域 */}
            <div className='flex flex-col h-full'>
              <div className='p-2 bg-gray-100 dark:bg-gray-700 border-b dark:border-gray-600 flex items-center justify-between flex-shrink-0'>
                <h2 className='font-semibold text-gray-800 dark:text-gray-200'>
                  格式化输出
                </h2>
                <div className='flex items-center space-x-2'>
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
              </div>
              <div className='flex-1 min-h-0 h-full'>
                <CodeEditor
                  language='json'
                  value={output}
                  readOnly={true}
                  options={{
                    minimap: { enabled: false },
                    wordWrap: 'off',
                  }}
                />
              </div>
            </div>
          </Split>
        </div>
      </div>
    </ToolLayout>
  )
}

export default JsonFormatter
