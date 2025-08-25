import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import React, { useState } from 'react'
import { Button } from '../components/common'
import FileUpload from '../components/common/FileUpload'
import { ToolLayout } from '../components/layouts'

interface PfxConversionResult {
  certificates: string[]
  private_keys: string[]
  combined_pem: string
  success: boolean
  error: string | null
}

const PfxToPemConverter: React.FC = () => {
  const [password, setPassword] = useState('')
  const [showPassword, setShowPassword] = useState(false)
  const [error, setError] = useState('')
  const [fileName, setFileName] = useState('')
  const [fileBuffer, setFileBuffer] = useState<ArrayBuffer | null>(null)
  const [result, setResult] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [showOpensslInfo, setShowOpensslInfo] = useState(false)

  // 处理二进制文件数据
  const handleBinaryFileData = (fileName: string, data: Uint8Array) => {
    // 验证文件类型
    if (!fileName.endsWith('.pfx') && !fileName.endsWith('.p12')) {
      setError('请选择PFX或P12格式的文件')
      return
    }

    setFileBuffer(data.buffer as ArrayBuffer)
    setFileName(fileName)
    setError('')
  }

  // 清除选择的文件
  const clearFile = () => {
    setFileBuffer(null)
    setFileName('')
    setError('')
  }

  // 错误处理适配器
  const handleError = (errorMsg: string | null) => {
    setError(errorMsg || '')
  }

  // 执行转换
  const performConversion = async () => {
    if (!fileBuffer) {
      setError('请选择PFX文件')
      return
    }

    setIsLoading(true)
    setError('')
    setResult('')

    try {
      // 将ArrayBuffer转换为Uint8Array
      const uint8Array = new Uint8Array(fileBuffer)

      // 使用Rust后端进行转换
      const result: PfxConversionResult = await invoke('convert_pfx_to_pem', {
        pfxData: Array.from(uint8Array),
        password: password || null,
      })

      if (result.success) {
        setResult(result.combined_pem)
      } else {
        setError(result.error || '转换失败')
      }
    } catch (err) {
      console.error('转换失败:', err)
      if (err instanceof Error) {
        let errorMessage = err.message

        // 根据错误类型提供具体的解决方案
        if (
          errorMessage.includes('密码') ||
          errorMessage.includes('password')
        ) {
          errorMessage =
            '密码错误或文件损坏。请检查密码是否正确，或尝试使用其他工具验证文件完整性。'
        } else if (
          errorMessage.includes('格式') ||
          errorMessage.includes('format')
        ) {
          errorMessage =
            '文件格式错误，可能不是有效的PFX文件。请确认文件未损坏且为正确的PFX/P12格式。'
        } else if (
          errorMessage.includes('未找到') ||
          errorMessage.includes('not found')
        ) {
          errorMessage =
            '未在PFX文件中找到证书和私钥。请检查以下几点：\n1. 确保选择的文件是有效的PFX/P12文件\n2. 如果文件有密码保护，请输入正确的密码\n3. 确认文件中确实包含证书和私钥内容'
        }

        setError(errorMessage)
      } else {
        setError('转换失败: ' + String(err))
      }
    } finally {
      setIsLoading(false)
    }
  }

  // 复制结果到剪贴板
  const copyToClipboard = async () => {
    try {
      await navigator.clipboard.writeText(result)
      setError('已复制到剪贴板')
      setTimeout(() => setError(''), 2000)
    } catch (err) {
      setError('复制失败')
    }
  }

  // 下载PEM文件
  const downloadPem = async () => {
    if (!result) return

    try {
      // 建议文件名
      const suggestedName =
        fileName.replace(/\.(pfx|p12)$/i, '.pem') || 'certificate.pem'

      // 打开保存对话框
      const filePath = await save({
        filters: [
          {
            name: 'PEM Files',
            extensions: ['pem'],
          },
        ],
        defaultPath: suggestedName,
      })

      if (filePath) {
        // 使用Tauri的文件系统API保存文件
        await writeTextFile(filePath, result)
        setError('文件保存成功')
        setTimeout(() => setError(''), 2000)
      }
    } catch (err) {
      console.error('保存文件失败:', err)
      if (err instanceof Error) {
        setError(`保存文件失败: ${err.message}`)
      } else {
        setError('保存文件失败: 未知错误')
      }
    }
  }

  return (
    <ToolLayout 
      title="PFX 转 PEM 转换器"
      subtitle="将PFX/P12格式的证书文件转换为PEM格式"
      actions={
        <Button 
          variant='secondary'
          size='sm'
          onClick={() => setShowOpensslInfo(!showOpensslInfo)}>
          {showOpensslInfo ? '隐藏' : '查看'}OpenSSL命令
        </Button>
      }
    >
      <div className='flex flex-col h-full space-y-4 overflow-y-auto'>

      {/* OpenSSL命令提示 */}
      {showOpensslInfo && (
        <div className='mb-4 p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-md'>
          <div className='flex items-center justify-between mb-3'>
            <h3 className='text-sm font-medium text-blue-800 dark:text-blue-200'>
              📋 OpenSSL命令参考
            </h3>
            <button
              onClick={() => setShowOpensslInfo(false)}
              className='text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-200'>
              <svg
                className='w-4 h-4'
                fill='none'
                stroke='currentColor'
                viewBox='0 0 24 24'>
                <path
                  strokeLinecap='round'
                  strokeLinejoin='round'
                  strokeWidth={2}
                  d='M6 18L18 6M6 6l12 12'
                />
              </svg>
            </button>
          </div>
          <div className='space-y-3 text-xs'>
            <div>
              <p className='font-medium text-blue-800 dark:text-blue-200 mb-1'>
                基本转换（无密码）：
              </p>
              <code className='block p-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded font-mono text-blue-700 dark:text-blue-300'>
                openssl pkcs12 -in certificate.pfx -out certificate.pem -nodes
              </code>
            </div>
            <div>
              <p className='font-medium text-blue-800 dark:text-blue-200 mb-1'>
                带密码转换：
              </p>
              <code className='block p-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded font-mono text-blue-700 dark:text-blue-300'>
                openssl pkcs12 -in certificate.pfx -out certificate.pem -nodes
                -password pass:yourpassword
              </code>
            </div>
            <div>
              <p className='font-medium text-blue-800 dark:text-blue-200 mb-1'>
                仅导出证书（不含私钥）：
              </p>
              <code className='block p-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded font-mono text-blue-700 dark:text-blue-300'>
                openssl pkcs12 -in certificate.pfx -out certificate.pem -nokeys
              </code>
            </div>
            <div>
              <p className='font-medium text-blue-800 dark:text-blue-200 mb-1'>
                仅导出私钥：
              </p>
              <code className='block p-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded font-mono text-blue-700 dark:text-blue-300'>
                openssl pkcs12 -in certificate.pfx -out privatekey.pem -nocerts
                -nodes
              </code>
            </div>
          </div>
          <p className='mt-3 text-xs text-blue-700 dark:text-blue-300 italic'>
            💡
            此工具为可视化版本，上述命令仅供参考。转换结果与命令行工具完全一致。
          </p>
        </div>
      )}

      {/* 使用说明 */}
      <div className='mb-4 p-3 bg-blue-50 dark:bg-blue-900/20 rounded-md'>
        <h3 className='text-sm font-medium text-blue-800 dark:text-blue-200 mb-2'>
          💡 使用提示
        </h3>
        <div className='text-xs text-blue-700 dark:text-blue-300 space-y-1'>
          <p>• 选择PFX/P12文件，如有密码请输入</p>
          <p>• 点击"开始转换"获取PEM格式证书</p>
          <p>• 转换后可复制或下载PEM文件</p>
        </div>
      </div>

      {/* PFX文件输入 */}
      <div className='mb-4'>
        <FileUpload
          value=''
          onChange={() => {}}
          onError={handleError}
          error={error}
          accept='.pfx,.p12'
          fileType='binary'
          onBinaryFileData={handleBinaryFileData}
        />
        {fileName && (
          <div className='mt-2 flex items-center justify-between'>
            <div className='text-sm text-gray-600 dark:text-gray-400'>
              已选择: {fileName}
            </div>
            <button
              onClick={clearFile}
              className='text-sm text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-300'>
              清除
            </button>
          </div>
        )}
      </div>

      {/* 密码输入 */}
      <div className='mb-4 flex flex-col sm:flex-row sm:items-center sm:space-x-3 space-y-2 sm:space-y-0'>
        <label className='sm:w-36 w-full text-sm font-medium text-gray-700 dark:text-gray-300 mb-0'>
          PFX 密码 (可选):
        </label>
        <div className='w-full sm:flex-1 flex'>
          <input
            type={showPassword ? 'text' : 'password'}
            className='w-full p-2 border border-gray-300 rounded-l-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder='输入PFX文件密码（如果有的话）'
          />
          <button
            type='button'
            className='px-3 py-2 bg-gray-200 text-gray-700 border border-l-0 border-gray-300 rounded-r-md hover:bg-gray-300 hover:text-gray-700 dark:bg-gray-600 dark:text-gray-200 dark:border-gray-600 dark:hover:bg-gray-500 dark:hover:text-gray-100 flex items-center justify-center'
            onClick={() => setShowPassword(!showPassword)}>
            {showPassword ? (
              <svg
                className='w-4 h-4'
                fill='none'
                stroke='currentColor'
                viewBox='0 0 24 24'>
                <path
                  strokeLinecap='round'
                  strokeLinejoin='round'
                  strokeWidth={2}
                  d='M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21'
                />
              </svg>
            ) : (
              <svg
                className='w-4 h-4'
                fill='none'
                stroke='currentColor'
                viewBox='0 0 24 24'>
                <path
                  strokeLinecap='round'
                  strokeLinejoin='round'
                  strokeWidth={2}
                  d='M15 12a3 3 0 11-6 0 3 3 0 016 0z'
                />
                <path
                  strokeLinecap='round'
                  strokeLinejoin='round'
                  strokeWidth={2}
                  d='M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z'
                />
              </svg>
            )}
          </button>
        </div>
      </div>

      {/* 操作按钮区域 */}
      <div className='mb-6'>
        <Button
          variant='primary'
          size='lg'
          onClick={performConversion}
          disabled={isLoading || !fileBuffer}
          className='w-full'>
          {isLoading ? '转换中...' : '开始转换'}
        </Button>
      </div>

      {/* 输出结果 */}
      {result && (
        <div className='mb-4'>
          <div className='flex justify-between items-center mb-2'>
            <label className='block text-sm font-medium text-gray-700 dark:text-gray-300'>
              转换结果:
            </label>
            <div className='space-x-2'>
              <Button
                variant='primary'
                size='sm'
                onClick={copyToClipboard}>
                复制结果
              </Button>
              <Button
                variant='primary'
                size='sm'
                onClick={downloadPem}>
                下载PEM
              </Button>
            </div>
          </div>
          <textarea
            value={result}
            readOnly
            className='w-full h-64 p-3 border border-gray-300 rounded-md font-mono text-sm bg-gray-50 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
            placeholder='转换结果将显示在这里...'
          />
        </div>
      )}

      {/* 错误信息 */}
      {error && error !== '已复制到剪贴板' && (
        <div className='mb-4'>
          <div className='p-3 bg-red-100 text-red-700 rounded-md text-sm'>
            {error}
          </div>
        </div>
      )}
      </div>
    </ToolLayout>
  )
}

export default PfxToPemConverter
