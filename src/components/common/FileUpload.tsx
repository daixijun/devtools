import { listen, TauriEvent } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { readFile, readTextFile } from '@tauri-apps/plugin-fs'
import React, { useEffect, useState } from 'react'

interface FileUploadProps {
  value: string
  onChange: (value: string) => void
  onError: (error: string | null) => void
  error: string | null
  // 文件类型配置
  accept?: string
  placeholder?: string
  className?: string
  textareaClassName?: string
  disabled?: boolean
  // 二进制文件支持
  fileType?: 'text' | 'binary'
  onBinaryFileData?: (fileName: string, data: Uint8Array) => void
}

/**
 * Tauri 专用文件上传组件
 * 支持文本文件和二进制文件
 */
const FileUpload: React.FC<FileUploadProps> = ({
  value,
  onChange,
  accept = '*',
  placeholder = '请输入内容...',
  error = '',
  onError,
  className = '',
  textareaClassName = '',
  disabled = false,
  fileType = 'text',
  onBinaryFileData,
}) => {
  const [inputMode, setInputMode] = useState<'file' | 'text'>('file')
  const [isDragOver, setIsDragOver] = useState(false)
  const [uploadedFileName, setUploadedFileName] = useState<string>('')

  // 使用 Tauri 的 dialog API 选择文件
  const handleFileSelect = async () => {
    try {
      onError?.('')

      // 构建文件过滤器
      const filters =
        accept && accept !== '*'
          ? [
              {
                name: '支持的文件',
                extensions: accept
                  .split(',')
                  .map((ext) => ext.trim().replace(/^\./, ''))
                  .filter((ext) => ext && !ext.includes('/')), // 过滤掉空值和 MIME 类型
              },
            ]
          : []

      const selected = await open({
        multiple: false,
        filters: filters.length > 0 ? filters : undefined,
        directory: false,
      })

      if (!selected || selected === null) {
        return // 用户取消选择
      }

      const filePath = Array.isArray(selected) ? selected[0] : selected
      const fileName = filePath.split(/[/\\]/).pop() || '未知文件'

      if (fileType === 'binary' && onBinaryFileData) {
        // 处理二进制文件
        try {
          // 使用readFile直接读取二进制文件
          const fileContent = await readFile(filePath)
          
          onBinaryFileData(fileName, fileContent)
          onError?.('')
          setInputMode('file')
          setUploadedFileName(fileName)
        } catch (readErr) {
          console.error('Failed to read binary file:', readErr)
          let errorMsg = '读取二进制文件失败'
          onError?.(errorMsg)
        }
      } else {
        // 处理文本文件
        const content = await readTextFile(filePath)

        onChange(content)
        onError?.('')
        setInputMode('text')
        setUploadedFileName(fileName)
      }
    } catch (err: any) {
      console.error('Failed to open file:', err)

      let errorMsg = '打开文件失败'
      if (err?.message) {
        if (err.message.includes('cancelled')) {
          return // 用户取消，不显示错误
        } else if (err.message.includes('permission')) {
          errorMsg = '文件访问权限不足'
        } else if (err.message.includes('not found')) {
          errorMsg = '文件不存在'
        } else {
          errorMsg = `打开文件失败: ${err.message}`
        }
      }

      onError?.(errorMsg)
    }
  }

  // 处理拖拽文件
  const handleFileDrop = async (filePath: string) => {
    try {
      onError?.('')

      const fileName = filePath.split(/[/\\]/).pop() || '未知文件'

      if (fileType === 'binary' && onBinaryFileData) {
        // 处理二进制文件
        try {
          const fileContent = await readFile(filePath)
          
          onBinaryFileData(fileName, fileContent)
          onError?.('')
          setInputMode('file')
          setUploadedFileName(fileName)
        } catch (readErr) {
          console.error('Failed to read binary file:', readErr)
          let errorMsg = '读取二进制文件失败'
          onError?.(errorMsg)
        }
      } else {
        // 处理文本文件
        const content = await readTextFile(filePath)

        onChange(content)
        onError?.('')
        setInputMode('text')
        setUploadedFileName(fileName)
      }
    } catch (err: any) {
      console.error('Failed to read dropped file:', err)

      let errorMsg = '读取文件失败'
      if (err?.message) {
        if (err.message.includes('permission')) {
          errorMsg = '文件访问权限不足'
        } else if (err.message.includes('not found')) {
          errorMsg = '文件不存在'
        } else {
          errorMsg = `读取文件失败: ${err.message}`
        }
      }

      onError?.(errorMsg)
    }
  }

  // 拖拽事件处理
  const handleDragOver = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(true)
  }

  const handleDragLeave = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(false)
  }

  const handleDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(false)
    // 在 Tauri 中，文件拖拽由事件监听器处理，这里不需要处理
  }

  // 处理点击上传区域
  const handleUploadAreaClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (inputMode === 'file') {
      const target = e.target as HTMLElement

      // 检查是否点击了上传按钮区域
      const isUploadButtonClick = target.closest('.upload-button-area')

      if (isUploadButtonClick) {
        // 点击上传按钮 - 打开文件选择对话框
        e.preventDefault()
        e.stopPropagation()
        handleFileSelect()
      } else {
        // 点击空白区域 - 切换到文本输入模式
        e.preventDefault()
        e.stopPropagation()
        setInputMode('text')

        // 延迟聚焦到文本输入框
        setTimeout(() => {
          const textarea = document.querySelector(
            'textarea',
          ) as HTMLTextAreaElement
          if (textarea) {
            textarea.focus()
          }
        }, 100)
      }
    }
  }

  // 处理文本输入框失去焦点事件
  const handleTextareaBlur = () => {
    // 如果输入框内容为空，恢复到文件上传模式
    if (!value.trim()) {
      setInputMode('file')
      setUploadedFileName('')
    }
  }

  // 清除内容
  const clearContent = () => {
    onChange('')
    onError?.('')
    setInputMode('file')
    setUploadedFileName('')
  }

  // 获取文件类型描述
  const getFileTypeDescription = (): string => {
    if (accept && accept !== '*') {
      const extensions = accept
        .split(',')
        .map((ext) => ext.trim().replace(/^\./, ''))
        .filter((ext) => ext && !ext.includes('/'))

      if (extensions.length > 0) {
        return `支持 ${extensions.join(', ')} 格式文件`
      }
    }
    return '支持文本格式文件'
  }

  // 监听 Tauri 文件拖拽事件
  useEffect(() => {
    let unlisten: (() => void) | null = null

    const setupDragListener = async () => {
      try {
        unlisten = await listen(TauriEvent.DRAG_DROP, async (event: any) => {
          if (
            event.payload &&
            event.payload.paths &&
            event.payload.paths.length > 0
          ) {
            await handleFileDrop(event.payload.paths[0])
          }
        })
      } catch (err) {
        console.error('Failed to register Tauri drag and drop listener:', err)
      }
    }

    setupDragListener()

    return () => {
      if (unlisten) {
        unlisten()
      }
    }
  }, [])

  return (
    <div className={`relative ${className}`}>
      {inputMode === 'file' ? (
        <div className='flex items-center justify-center w-full relative'>
          <div
            className={`flex flex-col items-center justify-center w-full h-32 border-2 border-dashed rounded-lg transition-colors duration-200 cursor-pointer ${
              isDragOver
                ? 'border-primary-500 bg-primary-50 dark:bg-primary-900/20'
                : 'border-slate-300 bg-slate-50 hover:bg-slate-100 dark:bg-slate-700 dark:border-slate-600 dark:hover:border-slate-500 dark:hover:bg-slate-600'
            }`}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
            onClick={handleUploadAreaClick}>
            {/* 上传按钮区域 */}
            <div className='flex flex-col items-center justify-center w-2/5 h-full upload-button-area'>
              <svg
                className='w-8 h-8 mb-4 text-slate-500 dark:text-slate-400'
                aria-hidden='true'
                xmlns='http://www.w3.org/2000/svg'
                fill='none'
                viewBox='0 0 20 16'>
                <path
                  stroke='currentColor'
                  strokeLinecap='round'
                  strokeLinejoin='round'
                  strokeWidth='2'
                  d='M13 13h3a3 3 0 0 0 0-6h-.025A5.56 5.56 0 0 0 16 6.5 5.5 5.5 0 0 0 5.207 5.021C5.137 5.017 5.071 5 5 5a4 4 0 0 0 0 8h2.167M10 15V6m0 0L8 8m2-2 2 2'
                />
              </svg>
              <p className='mb-2 text-sm text-slate-500 dark:text-slate-400'>
                <span className='font-semibold'>点击上传</span>
                {' 或 拖拽文件'}
              </p>
              <p className='text-xs text-slate-500 dark:text-slate-400'>
                {getFileTypeDescription()}
              </p>
              <p className='text-xs text-primary-500 dark:text-primary-400 mt-1'>
                💡 点击空白区域切换到文本输入模式
              </p>
            </div>
          </div>
        </div>
      ) : (
        <div>
          <div className='flex items-center justify-between mb-2'>
            {uploadedFileName && (
              <span className='text-sm text-slate-600 dark:text-slate-400'>
                当前文件: {uploadedFileName}
              </span>
            )}
            <div className='flex gap-2'>
              <button
                onClick={() => handleFileSelect()}
                className='text-xs text-primary-600 hover:text-primary-800 dark:text-primary-400 dark:hover:text-primary-300'
                disabled={disabled}>
                重新选择文件
              </button>
              <button
                onClick={clearContent}
                className='text-xs text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-300'
                disabled={disabled}>
                清空内容
              </button>
            </div>
          </div>
          <textarea
            className={`w-full h-48 p-3 border border-slate-300 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white font-mono resize-none ${textareaClassName}`}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onBlur={handleTextareaBlur}
            onKeyDown={(e) => {
              if (e.key === 'Escape' && !value.trim()) {
                setInputMode('file')
                setUploadedFileName('')
              }
            }}
            placeholder={placeholder}
            disabled={disabled}
          />
        </div>
      )}

      {/* 错误信息 */}
      {error && (
        <div className='mt-2 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg'>
          <p className='text-red-700 dark:text-red-400 text-sm'>{error}</p>
        </div>
      )}
    </div>
  )
}

export default FileUpload
