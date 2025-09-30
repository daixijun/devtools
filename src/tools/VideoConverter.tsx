import { invoke } from '@tauri-apps/api/core'
import { join } from '@tauri-apps/api/path'
import { open } from '@tauri-apps/plugin-dialog'
import { readDir, size } from '@tauri-apps/plugin-fs'
import React, { useEffect, useState } from 'react'
import { Button } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'

// 视频文件信息接口
interface VideoFile {
  name: string
  path: string
  size: number
  sourceFormat?: string // 源文件格式，如 "mov", "mp4", "avi" 等
  duration?: string // 视频时长，格式如 "00:01:30"
  resolution?: string // 视频分辨率，格式如 "1920x1080"
  outputPath?: string
  progress?: number
  status?: 'pending' | 'converting' | 'completed' | 'error'
  error?: string | { message?: string; error?: string; SystemError?: string }
}

// 视频转换响应接口
interface VideoConversionResponse {
  success: boolean
  output_path: string
  message: string
}

// 视频信息接口（与后端对应）
interface VideoInfo {
  name: string
  size: string
  format: string
  duration: string
  resolution: string
  path: string
}

// 视频转换请求接口
interface VideoConversionRequest {
  input_path: string
  output_path: string
  delete_source_file?: boolean
}

// 扩展File接口以包含path属性
declare global {
  interface File {
    path?: string
  }
}

const VideoConverter: React.FC = () => {
  // 状态管理
  const [videoFiles, setVideoFiles] = useState<VideoFile[]>([]) // 文件列表
  const [isConverting, setIsConverting] = useState<boolean>(false)
  const [currentConvertingIndex, setCurrentConvertingIndex] =
    useState<number>(-1) // 当前正在转换的文件索引
  const [error, setError] = useState<string>('')
  const [isSuccess, setIsSuccess] = useState<boolean>(false) // 是否全部转换成功
  const [sourceFormat, setSourceFormat] = useState<string>('auto') // 源格式，'auto'表示自动检测
  const [targetFormat, setTargetFormat] = useState<string>('mp4') // 目标格式
  const [useCustomOutputDir, setUseCustomOutputDir] = useState<boolean>(false) // 是否使用自定义输出目录
  const [customOutputDir, setCustomOutputDir] = useState<string>('') // 自定义输出目录
  const [deleteSourceFile, setDeleteSourceFile] = useState<boolean>(false) // 是否删除源文件
  const [ffmpegInstalled, setFfmpegInstalled] = useState<boolean | null>(null) // FFmpeg 安装状态，null 表示正在检查
  const { copyToClipboard } = useCopyToClipboard()

  // 检查 FFmpeg 安装状态
  useEffect(() => {
    const checkFfmpeg = async () => {
      try {
        const isAvailable = await invoke<boolean>('check_ffmpeg_available')
        setFfmpegInstalled(isAvailable)
        // 不再在这里设置错误信息，仅在 UI 中显示未安装提示
      } catch (err) {
        console.error('检查 FFmpeg 状态失败:', err)
        setFfmpegInstalled(false)
        setError('无法检查 FFmpeg 安装状态，请确保 FFmpeg 已正确安装')
      }
    }

    checkFfmpeg()
  }, [])

  // 支持的视频格式
  const supportedFormats = [
    { value: 'mp4', label: 'MP4', description: '通用视频格式' },
    { value: 'mov', label: 'MOV', description: 'QuickTime格式' },
    { value: 'avi', label: 'AVI', description: 'Audio Video Interleave' },
    { value: 'mkv', label: 'MKV', description: 'Matroska格式' },
    { value: 'wmv', label: 'WMV', description: 'Windows Media Video' },
    { value: 'flv', label: 'FLV', description: 'Flash Video' },
    { value: 'webm', label: 'WebM', description: 'Web视频格式' },
    { value: 'm4v', label: 'M4V', description: 'iTunes视频格式' },
    { value: '3gp', label: '3GP', description: '手机视频格式' },
    { value: 'mpeg', label: 'MPEG', description: 'MPEG格式' },
  ]

  // 选择文件（支持多选）
  const handleSelectFiles = () => {
    // 检查 FFmpeg 是否安装
    if (ffmpegInstalled === false) {
      setError('FFmpeg 未安装，无法使用视频转换功能。请先安装 FFmpeg。')
      return
    }

    // 根据选择的源格式确定文件过滤器
    let extensions: string[] = ['mov'] // 默认支持 MOV
    let filterName = 'MOV视频文件'

    if (sourceFormat !== 'auto') {
      // 查找选中格式的扩展名
      const format = supportedFormats.find((f) => f.value === sourceFormat)
      if (format) {
        extensions = [format.value]
        filterName = `${format.label}视频文件`
      }
    } else {
      // 自动模式支持所有格式
      extensions = supportedFormats.map((f) => f.value)
      filterName = '视频文件'
    }

    open({
      multiple: true,
      filters: [
        {
          name: filterName,
          extensions: extensions,
        },
      ],
    })
      .then((selected) => {
        if (selected) {
          const selectedPaths = Array.isArray(selected) ? selected : [selected]
          const files = selectedPaths.map((path) => {
            // 根据文件扩展名确定文件类型
            const fileExt =
              (typeof path === 'string'
                ? path.split('.').pop()?.toLowerCase()
                : 'mov') || 'mov'
            const mimeType = `video/${fileExt}`
            const file = new File([], path, { type: mimeType })
            Object.defineProperty(file, 'path', {
              value: path,
              writable: false,
            })
            return file
          })

          return handleFileSelect(files)
        }
      })
      .catch((error) => {
        setError(
          `选择文件失败: ${
            error instanceof Error ? error.message : String(error)
          }`,
        )
      })
  }

  // 选择自定义输出目录
  const handleSelectOutputDirectory = () => {
    open({
      directory: true,
      multiple: false,
    })
      .then((selected) => {
        if (selected && !Array.isArray(selected)) {
          setCustomOutputDir(selected)
        }
      })
      .catch((error) => {
        setError(
          `选择输出目录失败: ${
            error instanceof Error ? error.message : String(error)
          }`,
        )
      })
  }

  // 选择目录
  const handleSelectDirectory = () => {
    // 检查 FFmpeg 是否安装
    if (ffmpegInstalled === false) {
      setError('FFmpeg 未安装，无法使用视频转换功能。请先安装 FFmpeg。')
      return
    }

    open({
      directory: true,
      multiple: false,
    })
      .then((selected) => {
        if (selected && !Array.isArray(selected)) {
          // 遍历目录获取所有支持的视频文件
          return getFilesFromDirectory(selected).then((files) => {
            if (files.length === 0) {
              const formatList =
                sourceFormat === 'auto'
                  ? '所有支持的视频格式'
                  : `${
                      supportedFormats.find((f) => f.value === sourceFormat)
                        ?.label || sourceFormat
                    }格式`
              setError(`所选目录中没有找到 ${formatList} 的视频文件`)
              return
            }
            return handleFileSelect(files)
          })
        }
      })
      .catch((error) => {
        setError(
          `选择目录失败: ${
            error instanceof Error ? error.message : String(error)
          }`,
        )
      })
  }

  // 从目录中获取所有支持的视频文件
  const getFilesFromDirectory = (directoryPath: string): Promise<any[]> => {
    const files: any[] = []

    // 确定支持的文件扩展名
    let supportedExtensions: string[] = ['mov']
    if (sourceFormat === 'auto') {
      supportedExtensions = supportedFormats.map((f) => f.value)
    } else {
      const format = supportedFormats.find((f) => f.value === sourceFormat)
      if (format) {
        supportedExtensions = [format.value]
      }
    }

    return readDir(directoryPath)
      .then((entries) => {
        const promises = entries.map((entry) => {
          if (entry.isDirectory) {
            // 如果是目录，递归获取文件
            return join(directoryPath, entry.name)
              .then((subDirectoryPath) =>
                getFilesFromDirectory(subDirectoryPath),
              )
              .then((subFiles) => {
                files.push(...subFiles)
              })
          } else if (
            entry.isFile &&
            entry.name &&
            supportedExtensions.some((ext) =>
              entry.name.toLowerCase().endsWith(`.${ext}`),
            )
          ) {
            // 如果是支持的视频文件，添加到文件列表
            return join(directoryPath, entry.name).then((filePath) => {
              const fileExt = filePath.split('.').pop()?.toLowerCase() || 'mov'
              const mimeType = `video/${fileExt}`
              const file = new File([], filePath, { type: mimeType })
              Object.defineProperty(file, 'path', {
                value: filePath,
                writable: false,
              })
              files.push(file)
            })
          }
          return Promise.resolve()
        })
        return Promise.all(promises)
      })
      .then(() => files)
      .catch((error) => {
        console.error('遍历目录失败:', error)
        return files
      })
  }

  // 获取视频信息（统一接口）
  const getVideoInfo = async (filePath: string): Promise<VideoInfo | null> => {
    try {
      const info = await invoke<VideoInfo>('get_video_info', {
        inputPath: filePath,
      })
      return info
    } catch (error) {
      console.error('Failed to get video info:', error)
      return null
    }
  }

  // 处理文件选择
  const handleFileSelect = (files: any[]) => {
    // 确定支持的文件扩展名
    let supportedExtensions: string[] = ['mov']
    if (sourceFormat === 'auto') {
      supportedExtensions = supportedFormats.map((f) => f.value)
    } else {
      const format = supportedFormats.find((f) => f.value === sourceFormat)
      if (format) {
        supportedExtensions = [format.value]
      }
    }

    // 过滤出支持的视频文件
    const validFiles = files.filter((file) =>
      supportedExtensions.some((ext) =>
        file.name.toLowerCase().endsWith(`.${ext}`),
      ),
    )

    if (validFiles.length === 0) {
      const formatList =
        sourceFormat === 'auto'
          ? '所有支持的视频格式'
          : `${
              supportedFormats.find((f) => f.value === sourceFormat)?.label ||
              sourceFormat
            }格式`
      setError(`没有找到 ${formatList} 的视频文件`)
      return
    }

    // 处理每个文件
    const processedFiles: VideoFile[] = []

    const filePromises = validFiles.map(async (file) => {
      // 验证文件类型
      const fileExt = file.name.split('.').pop()?.toLowerCase()
      if (!fileExt || !supportedExtensions.includes(fileExt)) {
        return Promise.resolve()
      }

      // 获取实际的文件大小
      let fileSize = file.size
      if (fileSize === 0 && file.path) {
        try {
          fileSize = await size(file.path)
        } catch (err) {
          console.error('Failed to get file size:', err)
        }
      }

      // 获取视频信息
      let duration = '未知'
      let resolution = '未知'
      if (file.path) {
        try {
          const videoInfo = await getVideoInfo(file.path)
          if (videoInfo) {
            duration = videoInfo.duration
            resolution = videoInfo.resolution
          }
        } catch (err) {
          console.error('Failed to get video info:', err)
        }
      }

      // 生成输出路径（使用目标格式）
      const targetExt = targetFormat
      const fileName = file.name.replace(/\.[^/.]+$/, `.${targetExt}`)

      let outputPath: string
      if (useCustomOutputDir && customOutputDir) {
        // 使用自定义输出目录
        outputPath = `${customOutputDir}/${fileName}`
      } else {
        // 使用原文件同目录
        outputPath = fileName
      }

      processedFiles.push({
        name: file.name,
        path: file.path || '',
        size: fileSize,
        duration,
        resolution,
        outputPath,
        sourceFormat: fileExt,
        status: 'pending',
        progress: 0,
      })
    })

    Promise.all(filePromises).then(() => {
      // 添加到现有文件列表，避免重复
      const existingPaths = new Set(videoFiles.map((f) => f.path))
      const newFiles = processedFiles.filter((f) => !existingPaths.has(f.path))

      if (newFiles.length > 0) {
        setVideoFiles([...videoFiles, ...newFiles])
        setError('')
      }
    })
  }

  // 格式化文件大小
  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes'
    const k = 1024
    const sizes = ['Bytes', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  // 开始批量转换
  const startBatchConversion = () => {
    if (videoFiles.length === 0) {
      setError('请先选择视频文件')
      return
    }

    // 检查 FFmpeg 是否安装
    if (ffmpegInstalled === false) {
      setError('FFmpeg 未安装，无法开始转换。请先安装 FFmpeg。')
      return
    }

    setIsConverting(true)
    setCurrentConvertingIndex(0)
    setError('')

    // 创建文件列表的副本以避免直接修改状态
    const filesToProcess = [...videoFiles]
    const processedFiles = [...videoFiles]

    // 逐个处理文件
    let currentIndex = 0

    const processNextFile = () => {
      if (currentIndex >= filesToProcess.length) {
        // 所有文件转换完成
        setIsConverting(false)
        setCurrentConvertingIndex(-1)

        // 统计成功和失败的文件数量
        const successCount = processedFiles.filter(
          (file) => file.status === 'completed',
        ).length
        const errorCount = processedFiles.filter(
          (file) => file.status === 'error',
        ).length

        // 显示转换完成信息
        if (errorCount === 0) {
          setError(`✅ 批量转换完成：成功转换 ${successCount} 个文件`)
          setIsSuccess(true)
        } else {
          setError(
            `批量转换完成：成功转换 ${successCount} 个文件，${errorCount} 个文件转换失败`,
          )
          setIsSuccess(false)
        }
        return
      }

      setCurrentConvertingIndex(currentIndex)

      // 更新当前文件状态为转换中
      processedFiles[currentIndex] = {
        ...processedFiles[currentIndex],
        status: 'converting',
        progress: 0,
      }
      setVideoFiles([...processedFiles])

      // 模拟转换错误用于测试
      if (filesToProcess[currentIndex].name.includes('test')) {
        const errorMessage = '测试错误：文件格式不支持或文件损坏'
        processedFiles[currentIndex] = {
          ...processedFiles[currentIndex],
          status: 'error',
          progress: 0,
          error: errorMessage,
        }
        setVideoFiles([...processedFiles])
        currentIndex++
        setTimeout(processNextFile, 100)
        return
      }

      const request: VideoConversionRequest = {
        input_path: filesToProcess[currentIndex].path,
        output_path:
          filesToProcess[currentIndex].outputPath ||
          filesToProcess[currentIndex].name.replace(
            /\.[^/.]+$/,
            `.${targetFormat}`,
          ),
        delete_source_file: deleteSourceFile,
      }

      // 调用Tauri后端进行视频转换
      invoke<VideoConversionResponse>('convert_video', {
        request,
      })
        .then(() => {
          // 转换成功
          processedFiles[currentIndex] = {
            ...processedFiles[currentIndex],
            status: 'completed',
            progress: 100,
          }
        })
        .catch((err) => {
          console.error(
            `Video conversion failed for ${filesToProcess[currentIndex].name}:`,
            err,
          )

          // 简化错误处理：直接通过 .catch() 获取错误信息
          let errorMessage = '转换失败'

          if (err instanceof Error) {
            errorMessage = err.message
          } else {
            errorMessage = String(err)
          }

          processedFiles[currentIndex] = {
            ...processedFiles[currentIndex],
            status: 'error',
            progress: 0,
            error: errorMessage,
          }
        })
        .finally(() => {
          setVideoFiles([...processedFiles])
          currentIndex++
          setTimeout(processNextFile, 100)
        })
    }

    // 开始处理第一个文件
    processNextFile()
  }

  // 删除指定文件
  const removeFile = (index: number) => {
    const file = videoFiles[index]
    if (!file) return

    // 显示确认对话框
    const confirmed = window.confirm(`确定要删除文件 "${file.name}" 吗？`)
    if (!confirmed) return

    setVideoFiles((prevFiles) => prevFiles.filter((_, i) => i !== index))
    // 如果删除的是正在转换的文件，重置转换状态
    if (currentConvertingIndex === index) {
      setCurrentConvertingIndex(-1)
      setIsConverting(false)
    }
    // 如果删除的文件在当前转换索引之前，需要调整索引
    if (currentConvertingIndex > index) {
      setCurrentConvertingIndex((prev) => prev - 1)
    }
  }

  // 清空文件列表
  const clearFileList = () => {
    setVideoFiles([])
    setIsConverting(false)
    setCurrentConvertingIndex(-1)
    setError('')
    setIsSuccess(false)
  }

  return (
    <ToolLayout
      title='视频格式转换器'
      description='支持多种主流视频格式的相互转换'>
      <div className='flex flex-col h-full'>
        {/* FFmpeg 状态检查 - 仅在未安装时显示 */}
        {ffmpegInstalled === false && (
          <div className='mb-4 p-4 rounded-lg border bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800'>
            <div className='flex items-center space-x-2'>
              <div className='w-3 h-3 rounded-full bg-red-500'></div>
              <p className='text-sm font-medium text-red-700 dark:text-red-400'>
                ❌ FFmpeg 未安装，无法使用视频转换功能
              </p>
            </div>
            <div className='mt-2 text-xs text-red-600 dark:text-red-400'>
              <p className='font-medium mb-1'>安装方法：</p>
              <ul className='space-y-1 ml-4'>
                <li>
                  • macOS:{' '}
                  <code className='bg-red-100 dark:bg-red-900/30 px-1 rounded'>
                    brew install ffmpeg
                  </code>
                </li>
                <li>• Windows: 下载 FFmpeg 并添加到 PATH</li>
                <li>
                  • Linux:{' '}
                  <code className='bg-red-100 dark:bg-red-900/30 px-1 rounded'>
                    sudo apt install ffmpeg
                  </code>{' '}
                  (Ubuntu/Debian)
                </li>
                <li>
                  • Linux:{' '}
                  <code className='bg-red-100 dark:bg-red-900/30 px-1 rounded'>
                    sudo dnf install ffmpeg
                  </code>{' '}
                  (Fedora)
                </li>
              </ul>
              <p className='mt-2'>安装完成后请刷新页面或重新打开工具。</p>
            </div>
          </div>
        )}

        {/* 配置区域 */}
        <div className='mb-6 p-4 border border-gray-200 dark:border-gray-700 rounded-lg bg-gray-50 dark:bg-gray-800'>
          <h3 className='text-lg font-medium text-gray-900 dark:text-white mb-4'>
            转换设置
          </h3>

          {/* 格式设置 */}
          <div className='grid grid-cols-1 md:grid-cols-2 gap-4 mb-4'>
            {/* 源格式选择 */}
            <div>
              <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2'>
                源格式
              </label>
              <select
                value={sourceFormat}
                onChange={(e) => setSourceFormat(e.target.value)}
                className='w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
                disabled={isConverting || ffmpegInstalled === false}>
                <option value='auto'>自动检测</option>
                {supportedFormats.map((format) => (
                  <option key={`source-${format.value}`} value={format.value}>
                    {format.label} ({format.value})
                  </option>
                ))}
              </select>
              <p className='mt-1 text-xs text-gray-500 dark:text-gray-400'>
                选择源视频文件格式，自动检测可支持所有格式
              </p>
            </div>

            {/* 目标格式选择 */}
            <div>
              <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2'>
                目标格式
              </label>
              <select
                value={targetFormat}
                onChange={(e) => setTargetFormat(e.target.value)}
                className='w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
                disabled={isConverting || ffmpegInstalled === false}>
                {supportedFormats.map((format) => (
                  <option key={`target-${format.value}`} value={format.value}>
                    {format.label} ({format.value})
                  </option>
                ))}
              </select>
              <p className='mt-1 text-xs text-gray-500 dark:text-gray-400'>
                转换后的视频格式，MP4为通用格式
              </p>
            </div>
          </div>

          {/* 输出目录设置 */}
          <div className='space-y-3'>
            <div className='flex items-center space-x-3'>
              <label className='flex items-center cursor-pointer'>
                <input
                  type='checkbox'
                  checked={useCustomOutputDir}
                  onChange={(e) => setUseCustomOutputDir(e.target.checked)}
                  className='w-4 h-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded'
                  disabled={isConverting || ffmpegInstalled === false}
                />
                <span className='ml-2 text-sm font-medium text-gray-700 dark:text-gray-300'>
                  指定输出目录
                </span>
              </label>
            </div>

            {useCustomOutputDir && (
              <div className='flex items-center space-x-3'>
                <div className='flex-1'>
                  <input
                    type='text'
                    value={customOutputDir}
                    onChange={(e) => setCustomOutputDir(e.target.value)}
                    placeholder='选择输出目录...'
                    className='w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
                    readOnly
                  />
                </div>
                <Button
                  onClick={handleSelectOutputDirectory}
                  variant='secondary'
                  size='sm'
                  disabled={isConverting || ffmpegInstalled === false}>
                  选择目录
                </Button>
                {customOutputDir && (
                  <Button
                    onClick={() => setCustomOutputDir('')}
                    variant='secondary'
                    size='sm'
                    disabled={isConverting || ffmpegInstalled === false}>
                    清除
                  </Button>
                )}
              </div>
            )}

            {useCustomOutputDir && customOutputDir && (
              <div className='text-xs text-gray-500 dark:text-gray-400'>
                转换后的文件将保存到: {customOutputDir}
              </div>
            )}
          </div>

          {/* 删除源文件选项 */}
          <div className='mt-4 pt-4 border-t border-gray-200 dark:border-gray-600'>
            <div className='flex items-center space-x-3'>
              <label className='flex items-center cursor-pointer'>
                <input
                  type='checkbox'
                  checked={deleteSourceFile}
                  onChange={(e) => setDeleteSourceFile(e.target.checked)}
                  className='w-4 h-4 text-red-600 focus:ring-red-500 border-gray-300 rounded'
                  disabled={isConverting || ffmpegInstalled === false}
                />
                <span className='ml-2 text-sm font-medium text-red-700 dark:text-red-400'>
                  删除源文件
                </span>
              </label>
            </div>
            <p className='mt-1 text-xs text-red-500 dark:text-red-400'>
              ⚠️ 转换成功后自动删除源文件，此操作不可恢复，请谨慎使用
            </p>
          </div>
        </div>

        {/* 文件列表区域 */}
        <div className='mb-8 flex-1 min-h-[300px]'>
          <div className='flex justify-between items-center mb-3'>
            <h3 className='text-lg font-medium text-gray-900 dark:text-white'>
              待转换文件列表 ({videoFiles.length}个文件)
            </h3>
            <div className='flex space-x-2'>
              <Button
                onClick={handleSelectFiles}
                variant='secondary'
                size='sm'
                disabled={ffmpegInstalled === false}>
                添加文件
              </Button>
              <Button
                onClick={handleSelectDirectory}
                variant='secondary'
                size='sm'
                disabled={ffmpegInstalled === false}>
                添加目录
              </Button>
            </div>
          </div>

          {videoFiles.length > 0 ? (
            <div className='border border-gray-200 dark:border-gray-700 rounded-lg p-4 bg-gray-50 dark:bg-gray-800 h-full overflow-hidden flex flex-col'>
              <div className='overflow-y-auto flex-1 min-h-0'>
                <table className='min-w-full divide-y divide-gray-200 dark:divide-gray-700'>
                  <thead className='bg-gray-100 dark:bg-gray-700 sticky top-0'>
                    <tr>
                      <th className='px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider'>
                        路径信息
                      </th>
                      <th className='px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider'>
                        大小
                      </th>
                      <th className='px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider'>
                        时长
                      </th>
                      <th className='px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider'>
                        分辨率
                      </th>
                      <th className='px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider'>
                        状态
                      </th>
                      <th className='px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider'>
                        错误信息
                      </th>
                      <th className='px-4 py-2 text-center text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider'>
                        操作
                      </th>
                    </tr>
                  </thead>
                  <tbody className='divide-y divide-gray-200 dark:divide-gray-700'>
                    {videoFiles.map((file, index) => (
                      <tr
                        key={index}
                        className={
                          currentConvertingIndex === index
                            ? 'bg-blue-50 dark:bg-blue-900/20'
                            : ''
                        }>
                        <td className='px-4 py-2 text-sm'>
                          <div className='space-y-1'>
                            {/* 源文件路径 */}
                            <div className='flex items-start space-x-1'>
                              <span className='text-xs text-gray-500 dark:text-gray-400 font-medium min-w-[45px]'>
                                源文件:
                              </span>
                              <div className='flex items-center space-x-1 flex-1 min-w-0'>
                                <span
                                  className='text-xs text-gray-700 dark:text-gray-300 truncate inline-block cursor-pointer hover:text-blue-600 dark:hover:text-blue-400'
                                  title={file.path}
                                  onClick={() =>
                                    file.path && copyToClipboard(file.path)
                                  }>
                                  {file.path || '未知'}
                                </span>
                                <button
                                  onClick={() =>
                                    file.path && copyToClipboard(file.path)
                                  }
                                  className='text-gray-400 hover:text-gray-600 dark:text-gray-500 dark:hover:text-gray-300 flex-shrink-0'
                                  title='复制源文件路径'>
                                  <svg
                                    className='w-3 h-3'
                                    fill='none'
                                    stroke='currentColor'
                                    viewBox='0 0 24 24'>
                                    <path
                                      strokeLinecap='round'
                                      strokeLinejoin='round'
                                      strokeWidth={2}
                                      d='M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z'
                                    />
                                  </svg>
                                </button>
                              </div>
                            </div>

                            {/* 转换后路径 */}
                            <div className='flex items-start space-x-1'>
                              <span className='text-xs text-gray-500 dark:text-gray-400 font-medium min-w-[45px]'>
                                转换后:
                              </span>
                              <div className='flex items-center space-x-1 flex-1 min-w-0'>
                                {file.status === 'completed' &&
                                file.outputPath ? (
                                  <>
                                    <span
                                      className='text-xs text-blue-600 dark:text-blue-400 truncate inline-block cursor-pointer hover:text-blue-800 dark:hover:text-blue-300'
                                      title={file.outputPath}
                                      onClick={() =>
                                        file.outputPath &&
                                        copyToClipboard(file.outputPath)
                                      }>
                                      {file.outputPath}
                                    </span>
                                    <button
                                      onClick={() =>
                                        file.outputPath &&
                                        copyToClipboard(file.outputPath)
                                      }
                                      className='text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-200 flex-shrink-0'
                                      title='复制转换后路径'>
                                      <svg
                                        className='w-3 h-3'
                                        fill='none'
                                        stroke='currentColor'
                                        viewBox='0 0 24 24'>
                                        <path
                                          strokeLinecap='round'
                                          strokeLinejoin='round'
                                          strokeWidth={2}
                                          d='M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z'
                                        />
                                      </svg>
                                    </button>
                                  </>
                                ) : (
                                  <span className='text-xs text-gray-400 dark:text-gray-500'>
                                    未转换
                                  </span>
                                )}
                              </div>
                            </div>
                          </div>
                        </td>
                        <td className='px-4 py-2 text-sm text-gray-900 dark:text-gray-100'>
                          {formatFileSize(file.size)}
                        </td>
                        <td className='px-4 py-2 text-sm text-gray-900 dark:text-gray-100'>
                          {file.duration || '未知'}
                        </td>
                        <td className='px-4 py-2 text-sm text-gray-900 dark:text-gray-100'>
                          {file.resolution || '未知'}
                        </td>
                        <td className='px-4 py-2 text-sm'>
                          {file.status === 'pending' && (
                            <span className='text-yellow-600 dark:text-yellow-400'>
                              等待中
                            </span>
                          )}
                          {file.status === 'converting' && (
                            <span className='text-blue-600 dark:text-blue-400'>
                              转换中
                            </span>
                          )}
                          {file.status === 'completed' && (
                            <span className='text-green-600 dark:text-green-400'>
                              已完成
                            </span>
                          )}
                          {file.status === 'error' && (
                            <span className='text-red-600 dark:text-red-400'>
                              错误
                            </span>
                          )}
                        </td>
                        <td className='px-4 py-2 text-sm'>
                          {file.error ? (
                            <span
                              className='text-red-600 dark:text-red-400 text-xs truncate max-w-[200px] inline-block align-middle'
                              title={
                                typeof file.error === 'string'
                                  ? file.error
                                  : String(file.error)
                              }>
                              {(() => {
                                const errorText =
                                  typeof file.error === 'string'
                                    ? file.error
                                    : String(file.error)
                                // 截断处理：超过30个字符显示省略号
                                if (errorText.length > 30) {
                                  return errorText.substring(0, 30) + '...'
                                }
                                return errorText
                              })()}
                            </span>
                          ) : (
                            <span className='text-gray-400 dark:text-gray-500 text-xs'>
                              -
                            </span>
                          )}
                        </td>
                        <td className='px-4 py-2 text-sm text-center'>
                          <button
                            onClick={() => removeFile(index)}
                            disabled={
                              (isConverting && file.status === 'converting') ||
                              ffmpegInstalled === false
                            }
                            className={`p-1 rounded-md transition-colors ${
                              (isConverting && file.status === 'converting') ||
                              ffmpegInstalled === false
                                ? 'text-gray-400 cursor-not-allowed'
                                : 'text-red-600 hover:text-red-700 hover:bg-red-50 dark:hover:bg-red-900/20'
                            }`}
                            title='删除文件'>
                            <svg
                              className='w-4 h-4'
                              fill='none'
                              stroke='currentColor'
                              viewBox='0 0 24 24'>
                              <path
                                strokeLinecap='round'
                                strokeLinejoin='round'
                                strokeWidth={2}
                                d='M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16'
                              />
                            </svg>
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          ) : (
            <div className='border border-gray-200 dark:border-gray-700 rounded-lg p-8 bg-gray-50 dark:bg-gray-800 text-center'>
              <p className='text-gray-500 dark:text-gray-400'>
                暂无待转换文件，请点击右上角按钮添加文件或目录
              </p>
            </div>
          )}
        </div>

        {/* 转换控制区域 */}
        {videoFiles.length > 0 && (
          <div className='mb-6 mt-4'>
            <h3 className='text-lg font-medium text-gray-900 dark:text-white mb-3'>
              转换控制
            </h3>
            <div className='border border-gray-200 dark:border-gray-700 rounded-lg p-4 bg-gray-50 dark:bg-gray-800'>
              {/* 整体进度条 */}
              {isConverting && currentConvertingIndex >= 0 && (
                <div className='mb-4'>
                  <div className='flex justify-between items-center mb-2'>
                    <p className='text-sm text-gray-600 dark:text-gray-400'>
                      总体进度: {currentConvertingIndex + 1} /{' '}
                      {videoFiles.length} 个文件
                    </p>
                    <p className='text-sm text-gray-600 dark:text-gray-400'>
                      {Math.round(
                        ((currentConvertingIndex +
                          (videoFiles[currentConvertingIndex]?.progress || 0) /
                            100) /
                          videoFiles.length) *
                          100,
                      )}
                      %
                    </p>
                  </div>
                  <div className='w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2'>
                    <div
                      className='bg-blue-600 h-2 rounded-full transition-all duration-300'
                      style={{
                        width: `${
                          ((currentConvertingIndex +
                            (videoFiles[currentConvertingIndex]?.progress ||
                              0) /
                              100) /
                            videoFiles.length) *
                          100
                        }%`,
                      }}></div>
                  </div>
                </div>
              )}

              {/* 操作按钮 */}
              <div className='flex space-x-3'>
                <Button
                  onClick={startBatchConversion}
                  variant='primary'
                  disabled={
                    isConverting ||
                    videoFiles.length === 0 ||
                    (useCustomOutputDir && !customOutputDir) ||
                    ffmpegInstalled === false
                  }
                  className='flex-1'>
                  {isConverting ? '转换中...' : '开始转换'}
                </Button>

                <Button
                  onClick={clearFileList}
                  variant='secondary'
                  className='flex-1'
                  disabled={ffmpegInstalled === false}>
                  清空列表
                </Button>
              </div>
            </div>
          </div>
        )}

        {/* 错误/状态信息 */}
        {error && (
          <div
            className={`mt-4 p-4 rounded-lg ${
              isSuccess
                ? 'bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800'
                : 'bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800'
            }`}>
            <p
              className={`text-sm ${
                isSuccess
                  ? 'text-green-700 dark:text-green-400'
                  : 'text-red-700 dark:text-red-400'
              }`}>
              {error}
            </p>
          </div>
        )}
      </div>
    </ToolLayout>
  )
}

export default VideoConverter
