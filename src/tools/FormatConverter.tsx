import * as yaml from 'js-yaml'
import React, { useEffect, useState } from 'react'
import { parse as parseToml } from 'smol-toml'
import { CodeEditor } from '../components/common'
import { ToolLayout } from '../components/layouts'

const FormatConverter: React.FC = () => {
  const [input, setInput] = useState('')
  const [output, setOutput] = useState('')
  const [error, setError] = useState('')
  const [inputFormat, setInputFormat] = useState<'json' | 'yaml' | 'toml'>(
    'json',
  )
  const [outputFormat, setOutputFormat] = useState<'json' | 'yaml' | 'toml'>(
    'json',
  )

  // 前端格式转换函数
  const convertFormat = (content: string, from: string, to: string): string => {
    if (!content.trim()) return ''

    try {
      // 解析输入格式
      let parsedData: any
      switch (from) {
        case 'json':
          parsedData = JSON.parse(content)
          break
        case 'yaml':
          parsedData = yaml.load(content)
          break
        case 'toml':
          parsedData = parseToml(content)
          break
        default:
          throw new Error(`不支持的输入格式: ${from}`)
      }

      // 转换为输出格式
      switch (to) {
        case 'json':
          return JSON.stringify(parsedData, null, 2)
        case 'yaml':
          return yaml.dump(parsedData, { indent: 2 })
        case 'toml':
          // 由于smol-toml只提供解析功能，我们需要手动实现TOML序列化
          // 这里使用一个简单的TOML序列化函数
          return serializeToToml(parsedData)
        default:
          throw new Error(`不支持的输出格式: ${to}`)
      }
    } catch (err) {
      throw new Error(
        `转换失败: ${err instanceof Error ? err.message : String(err)}`,
      )
    }
  }

  // 完整的TOML序列化函数
  const serializeToToml = (data: any, currentTable = ''): string => {
    if (typeof data !== 'object' || data === null) {
      return formatTomlValue(data)
    }

    if (Array.isArray(data)) {
      // 处理数组
      if (data.length === 0) return '[]'

      // 检查数组元素类型
      const firstType = typeof data[0]
      const isHomogeneous = data.every((item) => typeof item === firstType)

      if (isHomogeneous && firstType !== 'object') {
        // 基本类型数组
        return `[${data.map((item) => formatTomlValue(item)).join(', ')}]`
      } else {
        // 对象数组或混合类型数组
        return `[\n${data
          .map((item) => `  ${serializeToToml(item)}`)
          .join(',\n')}\n]`
      }
    }

    // 处理对象
    const lines: string[] = []
    const simpleEntries: [string, any][] = []
    const tableEntries: [string, any][] = []

    // 分离简单值和嵌套对象
    for (const [key, value] of Object.entries(data)) {
      if (
        typeof value === 'object' &&
        value !== null &&
        !Array.isArray(value)
      ) {
        tableEntries.push([key, value])
      } else {
        simpleEntries.push([key, value])
      }
    }

    // 输出简单值
    if (simpleEntries.length > 0) {
      if (currentTable) {
        lines.push(`[${currentTable}]`)
      }

      for (const [key, value] of simpleEntries) {
        const formattedKey = formatTomlKey(key)
        const formattedValue = formatTomlValue(value)
        lines.push(`${formattedKey} = ${formattedValue}`)
      }

      if (simpleEntries.length > 0 && tableEntries.length > 0) {
        lines.push('') // 空行分隔
      }
    }

    // 输出嵌套表
    for (const [key, value] of tableEntries) {
      const tablePath = currentTable ? `${currentTable}.${key}` : key
      const tableContent = serializeToToml(value, tablePath)
      if (tableContent.trim()) {
        lines.push(tableContent)
      }
    }

    return lines.join('\n')
  }

  // 格式化TOML键
  const formatTomlKey = (key: string): string => {
    // 如果键包含特殊字符，需要用引号包围
    if (/[^a-zA-Z0-9_-]/.test(key)) {
      return `"${key.replace(/"/g, '\\"')}"`
    }
    return key
  }

  // 格式化TOML值
  const formatTomlValue = (value: any): string => {
    if (value === null) return 'null'
    if (value === undefined) return 'null'

    switch (typeof value) {
      case 'string':
        // 字符串需要用引号包围，并转义特殊字符
        return `"${value.replace(/"/g, '\\"').replace(/\\/g, '\\\\')}"`
      case 'number':
        return String(value)
      case 'boolean':
        return String(value)
      case 'object':
        if (Array.isArray(value)) {
          return serializeToToml(value)
        }
        return serializeToToml(value)
      default:
        return String(value)
    }
  }

  useEffect(() => {
    const performConversion = () => {
      if (!input.trim()) {
        setOutput('')
        setError('')
        return
      }

      try {
        if (inputFormat === outputFormat) {
          throw new Error('输入和输出格式相同，无需转换')
        }

        const result = convertFormat(input, inputFormat, outputFormat)
        setOutput(result)
        setError('')
      } catch (err) {
        setOutput('')
        setError(err instanceof Error ? err.message : String(err))
      }
    }

    performConversion()
  }, [input, inputFormat, outputFormat])

  const handleCopyOutput = async () => {
    if (output) {
      await navigator.clipboard.writeText(output)
    }
  }

  const handleClearInput = () => {
    setInput('')
  }

  const getMonacoLanguage = (format: string): string => {
    switch (format) {
      case 'json':
        return 'json'
      case 'yaml':
        return 'yaml'
      case 'toml':
        return 'toml'
      default:
        return 'plaintext'
    }
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
      title='格式转换器'
      description='支持 JSON、YAML、TOML 之间的双向转换'>
      {/* 格式选择器 */}
      <div className='flex-shrink-0 p-4 bg-gray-50 dark:bg-gray-800 border-b dark:border-gray-700 mb-6'>
        <div className='flex items-center space-x-4'>
          <div className='flex items-center space-x-2'>
            <span className='text-sm font-medium text-gray-700 dark:text-gray-300'>
              输入格式:
            </span>
            <select
              value={inputFormat}
              onChange={(e) => setInputFormat(e.target.value as any)}
              className='px-3 py-1 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 text-sm'>
              <option value='json'>JSON</option>
              <option value='yaml'>YAML</option>
              <option value='toml'>TOML</option>
            </select>
          </div>

          <div className='flex items-center space-x-2'>
            <span className='text-sm font-medium text-gray-700 dark:text-gray-300'>
              输出格式:
            </span>
            <select
              value={outputFormat}
              onChange={(e) => {
                setOutputFormat(e.target.value as any)
                // 转换逻辑现在由useEffect处理
              }}
              className='px-3 py-1 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 text-sm'>
              <option value='json'>JSON</option>
              <option value='yaml'>YAML</option>
              <option value='toml'>TOML</option>
            </select>
          </div>
        </div>
      </div>

      {/* 错误提示 */}
      {error && (
        <div className='p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg mb-4'>
          <p className='text-red-700 dark:text-red-400 text-sm'>{error}</p>
        </div>
      )}

      <div className='grid grid-cols-1 md:grid-cols-2 gap-6 flex-1'>
        {/* 左侧输入区域 */}
        <div className='flex flex-col'>
          <div className='p-2 bg-gray-100 dark:bg-gray-700 border-b dark:border-gray-600 flex items-center justify-between'>
            <h2 className='font-semibold text-gray-800 dark:text-gray-200'>
              输入内容
            </h2>
            <div className='flex items-center space-x-2'>
              <span className='text-sm text-gray-600 dark:text-gray-400'>
                长度: {input.length}
              </span>
              <button
                onClick={handleLoadExample}
                className='px-3 py-1 text-sm bg-blue-500 text-white rounded-md hover:bg-blue-600'>
                示例
              </button>
              <button
                onClick={handleClearInput}
                disabled={!input}
                className='px-3 py-1 text-sm bg-gray-500 text-white rounded-md hover:bg-gray-600 disabled:opacity-50'>
                清空
              </button>
            </div>
          </div>
          <div className='flex-1 min-h-0 h-full'>
            <CodeEditor
              value={input}
              onChange={setInput}
              language={getMonacoLanguage(inputFormat)}
              placeholder='粘贴 JSON、YAML 或 TOML 内容...'
              options={{
                minimap: { enabled: false },
                wordWrap: 'on',
              }}
            />
          </div>
        </div>

        {/* 右侧输出区域 */}
        <div className='flex flex-col'>
          <div className='p-2 bg-gray-100 dark:bg-gray-700 border-b dark:border-gray-600 flex items-center justify-between'>
            <h2 className='font-semibold text-gray-800 dark:text-gray-200'>
              转换结果
            </h2>
            <div className='flex items-center space-x-2'>
              <span className='text-sm text-gray-600 dark:text-gray-400'>
                长度: {output.length}
              </span>
              <button
                onClick={handleCopyOutput}
                disabled={!output || !!error}
                className='px-3 py-1 text-sm bg-green-500 text-white rounded-md hover:bg-green-600 disabled:opacity-50'>
                复制结果
              </button>
            </div>
          </div>
          <div className='flex-1 min-h-0 h-full'>
            <CodeEditor
              value={output}
              readOnly={true}
              language={getMonacoLanguage(outputFormat)}
              placeholder='转换结果将显示在这里...'
              options={{
                minimap: { enabled: false },
                wordWrap: 'on',
              }}
            />
          </div>
        </div>
      </div>
    </ToolLayout>
  )
}

export default FormatConverter
