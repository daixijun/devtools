import jsyaml from 'js-yaml'
import React, { useEffect, useState } from 'react'
import Split from 'react-split'
import { Button, CodeEditor } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard, useDebounce } from '../hooks'
import { errorUtils, validators } from '../utils'

/**
 * JSON 转 YAML 工具
 * 使用重构后的公共组件，提供统一的用户体验
 */
const JsonToYaml: React.FC = () => {
  const [input, setInput] = useState('')
  const [output, setOutput] = useState('')
  const [error, setError] = useState('')
  const { copy, copied } = useCopyToClipboard()

  // 使用防抖处理，避免频繁的转换操作
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

      // 解析JSON并转换为YAML
      const parsed = JSON.parse(debouncedInput)
      const yaml = jsyaml.dump(parsed, {
        indent: 2,
        lineWidth: 80,
        noRefs: true,
        skipInvalid: false,
      })
      setOutput(yaml)
      setError('')
    } catch (err) {
      setOutput('')
      setError(errorUtils.formatError(err, 'JSON转YAML失败'))
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
    const exampleJson = {
      name: '开发者工具箱',
      version: '1.0.0',
      description: '一个功能强大的开发者工具集合',
      features: ['JSON格式化', 'Base64编码/解码', 'JWT解析', '时间戳转换'],
      config: {
        theme: 'dark',
        language: 'zh-CN',
        autoSave: true,
      },
    }
    setInput(JSON.stringify(exampleJson, null, 2))
  }

  return (
    <ToolLayout
      title='JSON 转 YAML'
      subtitle='将JSON数据转换为YAML格式，支持格式验证和自动转换'>
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
                  YAML 输出
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
                  language='yaml'
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

export default JsonToYaml
