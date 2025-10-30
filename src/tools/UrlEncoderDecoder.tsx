import React, { useState } from 'react'
import { ToolLayout } from '../components/layouts'

const UrlEncoderDecoder: React.FC = () => {
  const [input, setInput] = useState('')
  const [output, setOutput] = useState('')
  const [mode, setMode] = useState<'encode' | 'decode'>('encode')

  const handleProcess = () => {
    try {
      if (mode === 'encode') {
        setOutput(encodeURIComponent(input))
      } else {
        setOutput(decodeURIComponent(input))
      }
    } catch (error) {
      setOutput(
        `错误: ${error instanceof Error ? error.message : String(error)}`,
      )
    }
  }

  const handleSwap = () => {
    setInput(output)
    setOutput(input)
    setMode(mode === 'encode' ? 'decode' : 'encode')
  }

  return (
    <ToolLayout
      title='URL 编码/解码'
      description='URL 编码和解码工具，支持 URI 组件编码'>
      <div className='space-y-4'>
        <div className='flex items-center space-x-4'>
          <label className='text-sm font-medium text-gray-700 dark:text-gray-300'>
            操作模式:
          </label>
          <div className='flex space-x-2'>
            <button
              onClick={() => setMode('encode')}
              className={`px-3 py-1 rounded ${
                mode === 'encode'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300'
              }`}>
              编码
            </button>
            <button
              onClick={() => setMode('decode')}
              className={`px-3 py-1 rounded ${
                mode === 'decode'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300'
              }`}>
              解码
            </button>
          </div>
          <button
            onClick={handleSwap}
            className='px-3 py-1 rounded bg-gray-300 dark:bg-gray-600 text-gray-700 dark:text-gray-300'>
            交换输入/输出
          </button>
        </div>

        <div>
          <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
            输入:
          </label>
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={
              mode === 'encode' ? '输入要编码的文本' : '输入要解码的 URL'
            }
            className='w-full h-32 p-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100'
          />
        </div>

        <div className='flex justify-center'>
          <button
            onClick={handleProcess}
            className='px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors'>
            {mode === 'encode' ? '编码' : '解码'}
          </button>
        </div>

        <div>
          <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
            输出:
          </label>
          <textarea
            value={output}
            readOnly
            className='w-full h-32 p-2 border border-gray-300 dark:border-gray-600 rounded-md bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-gray-100'
          />
        </div>

        <div className='text-sm text-gray-600 dark:text-gray-400'>
          <p>说明:</p>
          <ul className='list-disc list-inside mt-1'>
            <li>编码: 将文本转换为 URL 安全的格式</li>
            <li>解码: 将 URL 编码的文本还原为原始格式</li>
            <li>使用 encodeURIComponent/decodeURIComponent 函数</li>
          </ul>
        </div>
      </div>
    </ToolLayout>
  )
}

export default UrlEncoderDecoder
