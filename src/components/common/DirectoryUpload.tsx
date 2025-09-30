import { listen, TauriEvent } from '@tauri-apps/api/event'
import { join } from '@tauri-apps/api/path'
import { open } from '@tauri-apps/plugin-dialog'
import { readDir, size } from '@tauri-apps/plugin-fs'
import React, { useCallback, useEffect, useState } from 'react'

// 通用文件信息接口
interface FileInfo {
  name: string
  path: string
  size: number
  extension: string
  lastModified: string
}

interface DirectoryUploadProps {
  value?: string // 目录路径
  onChange: (files: FileInfo[]) => void // 当找到文件时调用
  onError: (error: string) => void // 当发生错误时调用
  error?: string // 错误信息
  placeholder?: string // 占位符文本
  disabled?: boolean // 是否禁用
  className?: string // 自定义类名
  acceptedExtensions?: string[] // 接受的文件扩展名，如 ['.mov', '.mp4']
}

/**
 * Tauri 专用目录上传组件
 * 支持选择目录并筛选特定类型的文件
 */
const DirectoryUpload: React.FC<DirectoryUploadProps> = ({
  value = '', // 默认值为空字符串
  onChange,
  onError,
  error = '', // 默认值为空字符串
  disabled = false,
  className = '',
  acceptedExtensions = [], // 默认接受所有文件
}) => {
  const [isDragOver, setIsDragOver] = useState(false)
  const [selectedDirectory, setSelectedDirectory] = useState<string>(value) // 使用value作为初始值
  const [foundFiles, setFoundFiles] = useState<FileInfo[]>([])

  // 扫描目录中的文件
  const scanDirectoryForFiles = useCallback(
    async (directoryPath: string) => {
      console.info(`Scanning directory: ${directoryPath}`)
      try {
        const files: FileInfo[] = []

        // 读取目录内容
        const entries = await readDir(directoryPath)

        // 筛选文件
        for (const entry of entries) {
          // 如果指定了接受的文件扩展名，则进行筛选
          const fileExtension = `.${entry.name.split('.').pop()?.toLowerCase()}`
          if (
            acceptedExtensions.length > 0 &&
            !acceptedExtensions
              .map((ext) => ext.toLowerCase())
              .includes(fileExtension)
          ) {
            continue
          }

          // 获取文件路径
          const filePath = await join(directoryPath, entry.name)

          // 使用Tauri 2的fs插件获取文件大小
          let fileSize = 0
          try {
            // 使用从@tauri-apps/plugin-fs导入的size函数获取文件大小
            fileSize = await size(filePath)
            console.log(`File ${entry.name} size: ${fileSize} bytes`)
          } catch (err) {
            console.error(`Failed to get size for file ${filePath}:`, err)
            // 如果获取文件大小失败，记录错误但继续处理
            // 文件大小将保持为0，但文件仍会被添加到列表中
          }

          files.push({
            name: entry.name,
            path: filePath,
            size: fileSize,
            extension: fileExtension,
            lastModified: new Date().toISOString(),
          })
        }

        setFoundFiles(files)
        onChange(files)
        onError('')
      } catch (err: any) {
        console.error('Failed to scan directory:', err)

        let errorMsg = '扫描目录失败'
        if (err?.message) {
          if (err.message.includes('permission')) {
            errorMsg = '目录访问权限不足'
          } else if (err.message.includes('not found')) {
            errorMsg = '目录不存在'
          } else {
            errorMsg = `扫描目录失败: ${err.message}`
          }
        }

        onError(errorMsg)
      }
    },
    [acceptedExtensions, onChange, onError],
  )

  // 使用 Tauri 的 dialog API 选择目录
  const handleDirectorySelect = useCallback(async () => {
    try {
      onError('')

      const selected = await open({
        directory: true,
        multiple: false,
        title: '选择包含MOV文件的目录',
      })

      if (!selected || selected === null) {
        return // 用户取消选择
      }

      const directoryPath = Array.isArray(selected) ? selected[0] : selected
      setSelectedDirectory(directoryPath)

      // 读取目录内容并筛选文件
      await scanDirectoryForFiles(directoryPath)
    } catch (err: any) {
      console.error('Failed to open directory:', err)

      let errorMsg = '打开目录失败'
      if (err?.message) {
        if (err.message.includes('cancelled')) {
          return // 用户取消，不显示错误
        } else if (err.message.includes('permission')) {
          errorMsg = '目录访问权限不足'
        } else if (err.message.includes('not found')) {
          errorMsg = '目录不存在'
        } else {
          errorMsg = `打开目录失败: ${err.message}`
        }
      }

      onError(errorMsg)
    }
  }, [onError, scanDirectoryForFiles])

  // 拖拽事件处理
  const handleDragOver = useCallback((e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(true)
  }, [])

  const handleDragLeave = useCallback((e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(false)
  }, [])

  const handleDrop = useCallback((e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragOver(false)
    // 在 Tauri 中，文件拖拽由事件监听器处理，这里不需要处理
  }, [])

  // 处理点击上传区域
  const handleUploadAreaClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      const target = e.target as HTMLElement

      // 检查是否点击了上传按钮区域
      const isUploadButtonClick = target.closest('.upload-button-area')

      if (isUploadButtonClick) {
        // 点击上传按钮 - 打开目录选择对话框
        e.preventDefault()
        e.stopPropagation()
        handleDirectorySelect()
      }
    },
    [handleDirectorySelect],
  )

  // 清除内容
  const clearContent = useCallback(() => {
    setSelectedDirectory('')
    setFoundFiles([])
    onChange([])
    onError('')
  }, [onChange, onError])

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
            // 如果拖拽的是目录，则处理目录
            const path = event.payload.paths[0]
            // 这里假设拖拽的是目录，实际应用中可能需要检查路径是否为目录
            setSelectedDirectory(path)
            await scanDirectoryForFiles(path)
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
  }, [scanDirectoryForFiles])

  // 当value属性变化时更新selectedDirectory状态
  useEffect(() => {
    if (value !== selectedDirectory) {
      setSelectedDirectory(value)

      // 如果value不为空，则扫描目录
      if (value) {
        scanDirectoryForFiles(value)
      } else {
        // 如果value为空，则清空文件列表
        setFoundFiles([])
        onChange([])
      }
    }
  }, [value, scanDirectoryForFiles])

  return (
    <div className={`relative ${className}`}>
      {!selectedDirectory ? (
        <div className='flex items-center justify-center w-full relative'>
          <div
            className={`flex flex-col items-center justify-center w-full h-32 border-2 border-dashed rounded-lg transition-colors duration-200 cursor-pointer ${
              isDragOver
                ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                : 'border-gray-300 bg-gray-50 hover:bg-gray-100 dark:bg-gray-700 dark:border-gray-600 dark:hover:border-gray-500 dark:hover:bg-gray-600'
            } ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
            onClick={disabled ? undefined : handleUploadAreaClick}>
            {/* 上传按钮区域 */}
            <div className='flex flex-col items-center justify-center w-2/5 h-full upload-button-area'>
              <svg
                className='w-8 h-8 mb-4 text-gray-500 dark:text-gray-400'
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
              <p className='mb-2 text-sm text-gray-500 dark:text-gray-400'>
                <span className='font-semibold'>点击选择目录</span>
                {' 或 拖拽目录到此处'}
              </p>
              <p className='text-xs text-gray-500 dark:text-gray-400'>
                支持选择包含
                {acceptedExtensions.length > 0
                  ? acceptedExtensions.join(', ')
                  : '文件'}
                的目录
              </p>
            </div>
          </div>
        </div>
      ) : (
        <div>
          <div className='flex items-center justify-between mb-2'>
            <span className='text-sm text-gray-600 dark:text-gray-400'>
              已选择目录: {selectedDirectory.split(/[/\\]/).pop() || '未知目录'}
            </span>
            <div className='flex gap-2'>
              <button
                onClick={() => handleDirectorySelect()}
                className='text-xs text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300'
                disabled={disabled}>
                重新选择目录
              </button>
              <button
                onClick={clearContent}
                className='text-xs text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-300'
                disabled={disabled}>
                清空内容
              </button>
            </div>
          </div>

          {/* 显示找到的文件数量 */}
          <div className='border border-gray-300 rounded-md shadow-sm dark:bg-gray-700 dark:border-gray-600 p-3'>
            <p className='text-sm font-medium text-gray-700 dark:text-gray-300'>
              {foundFiles.length > 0
                ? `已找到 ${foundFiles.length} 个文件`
                : '未找到文件'}
            </p>
          </div>
        </div>
      )}

      {/* 错误信息 */}
      {error && (
        <div className='mt-2 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md'>
          <p className='text-red-700 dark:text-red-400 text-sm'>{error}</p>
        </div>
      )}
    </div>
  )
}

export default DirectoryUpload
