import React, { useState } from 'react'
import { Toast } from '../components/common'
import { useCopyToClipboard, useToast } from '../hooks'

const UrlEncoderDecoder: React.FC = () => {
  const [input, setInput] = useState('')
  const [output, setOutput] = useState('')
  const [mode, setMode] = useState<'encode' | 'decode'>('encode')
  const { copy } = useCopyToClipboard()
  const { toast, show, showToast } = useToast()

  const handleCopy = async () => {
    if (output && output !== '') {
      const success = await copy(output)
      if (success) {
        showToast('已复制到剪贴板')
      }
    }
  }

  return (
    <div className='url-encoder-decoder p-4 h-full flex flex-col'>
      <div className='mb-4'>
        <h2 className='text-xl font-bold mb-2 text-gray-800 dark:text-white'>
          URL 编码/解码
        </h2>
        <p className='text-sm text-gray-600 dark:text-gray-400'>
          URL 编码是将特殊字符转换为可在 URL
          中安全传输的格式的过程。解码则相反。
        </p>
      </div>

      <div className='mb-4 flex-1 flex flex-col space-y-4'>
        <div className='flex-1 flex flex-col'>
          <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
            输入
          </label>
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={
              mode === 'encode' ? '输入要编码的文本' : '输入要解码的 URL'
            }
            className='flex-1 w-full p-3 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
            rows={8}
          />
        </div>

        <div className='flex justify-center space-x-4'>
          <button
            onClick={() => {
              setMode('encode')
              try {
                const result = encodeURIComponent(input)
                setOutput(result)
              } catch (error) {
                setOutput(
                  `错误: ${
                    error instanceof Error ? error.message : String(error)
                  }`,
                )
              }
            }}
            disabled={!input.trim()}
            className={`px-4 py-2 rounded-md font-medium text-sm transition-colors ${
              mode === 'encode'
                ? 'bg-blue-500 text-white'
                : 'bg-gray-200 text-gray-700 hover:bg-gray-300 dark:bg-gray-600 dark:text-gray-300 dark:hover:bg-gray-500'
            } disabled:bg-gray-400 disabled:cursor-not-allowed`}>
            编码
          </button>
          <button
            onClick={() => {
              setMode('decode')
              try {
                const result = decodeURIComponent(input)
                setOutput(result)
              } catch (error) {
                setOutput(
                  `错误: ${
                    error instanceof Error ? error.message : String(error)
                  }`,
                )
              }
            }}
            disabled={!input.trim()}
            className={`px-4 py-2 rounded-md font-medium text-sm transition-colors ${
              mode === 'decode'
                ? 'bg-blue-500 text-white'
                : 'bg-gray-200 text-gray-700 hover:bg-gray-300 dark:bg-gray-600 dark:text-gray-300 dark:hover:bg-gray-500'
            } disabled:bg-gray-400 disabled:cursor-not-allowed`}>
            解码
          </button>
        </div>

        <div className='flex-1 flex flex-col'>
          <div className='flex justify-between items-center mb-1'>
            <label className='block text-sm font-medium text-gray-700 dark:text-gray-300'>
              结果
            </label>
            {output && (
              <button
                onClick={handleCopy}
                className='px-3 py-1 text-xs bg-gray-200 hover:bg-gray-300 text-gray-700 dark:bg-gray-600 dark:hover:bg-gray-500 dark:text-gray-200 rounded transition-colors'>
                复制
              </button>
            )}
          </div>
          <textarea
            value={output}
            readOnly
            className='flex-1 w-full p-3 border border-gray-300 rounded-md bg-gray-50 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
            rows={8}
          />
        </div>
      </div>

      <div className='mt-4 p-3 bg-blue-50 dark:bg-blue-900/20 rounded-md'>
        <h3 className='text-sm font-medium text-blue-800 dark:text-blue-300 mb-1'>
          说明
        </h3>
        <ul className='text-xs text-blue-700 dark:text-blue-400 space-y-1'>
          <li>• 编码：将特殊字符转换为 %XX 格式，使其可以在 URL 中安全传输</li>
          <li>• 解码：将 %XX 格式的编码字符还原为原始字符</li>
          <li>• 常见编码字符：空格 → %20，+ → %2B，? → %3F，& → %26</li>
        </ul>
      </div>

      <Toast message={toast} show={show} type='success' />
    </div>
  )
}

export default UrlEncoderDecoder
