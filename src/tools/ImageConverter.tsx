import { invoke } from '@tauri-apps/api/core'
import { join } from '@tauri-apps/api/path'
import { open } from '@tauri-apps/plugin-dialog'
import { readDir, size } from '@tauri-apps/plugin-fs'
import React, { useState } from 'react'
import { Button } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'

// 图片文件信息接口
interface ImageFile {
  name: string
  path: string
  size: number
  sourceFormat?: string // 源文件格式，如 "jpg", "png", "gif" 等
  dimensions?: string // 图片尺寸，格式如 "1920x1080"
  has_exif?: boolean // 是否包含EXIF信息
  exif_data?: ExifData // 详细EXIF信息
  outputPath?: string
  progress?: number
  status?: 'pending' | 'converting' | 'completed' | 'error'
  error?: string | { message?: string; error?: string; SystemError?: string }
}

// EXIF数据接口
interface ExifData {
  make?: string
  model?: string
  datetime?: string
  exposure_time?: string
  f_number?: string
  iso?: string
  focal_length?: string
  software?: string
}

// 图片转换响应接口
interface ImageConversionResponse {
  success: boolean
  output_path: string
  message: string
  original_size?: number
  converted_size?: number
  compression_ratio?: number
}

// 图片转换请求接口
interface ImageConversionRequest {
  input_path: string
  output_path: string
  target_format: string
  quality?: number
  width?: number
  height?: number
  remove_exif: boolean
  delete_source_file?: boolean
}

// 图片信息接口
interface ImageInfo {
  name: string
  size: string
  format: string
  dimensions: string
  has_exif: boolean
  path: string
}

const ImageConverter: React.FC = () => {
  // 状态管理
  const [imageFiles, setImageFiles] = useState<ImageFile[]>([]) // 文件列表
  const [isConverting, setIsConverting] = useState<boolean>(false)
  const [currentConvertingIndex, setCurrentConvertingIndex] =
    useState<number>(-1) // 当前正在转换的文件索引
  const [error, setError] = useState<string>('')
  const [isSuccess, setIsSuccess] = useState<boolean>(false) // 是否全部转换成功
  const [hoveredExifData, setHoveredExifData] = useState<{
    file: ImageFile
    position: { x: number; y: number }
  } | null>(null) // 悬停的EXIF数据

  // 转换设置
  const [sourceFormat, setSourceFormat] = useState<string>('auto') // 源格式，'auto'表示自动检测
  const [targetFormat, setTargetFormat] = useState<string>('jpg') // 目标格式
  const [quality, setQuality] = useState<number>(85) // 图片质量 (1-100)
  const [resizeMode, setResizeMode] = useState<
    'none' | 'width' | 'height' | 'both'
  >('none') // 尺寸调整模式
  const [width, setWidth] = useState<number>(1920) // 目标宽度
  const [height, setHeight] = useState<number>(1080) // 目标高度
  const [removeExif, setRemoveExif] = useState<boolean>(false) // 移除EXIF信息
  const [deleteSourceFile, setDeleteSourceFile] = useState<boolean>(false) // 删除源文件
  const [useCustomOutputDir, setUseCustomOutputDir] = useState<boolean>(false) // 是否使用自定义输出目录
  const [customOutputDir, setCustomOutputDir] = useState<string>('') // 自定义输出目录
  const { copyToClipboard } = useCopyToClipboard()

  // 支持的图片格式
  const supportedFormats = [
    { value: 'jpg', label: 'JPG', description: '通用压缩格式' },
    { value: 'jpeg', label: 'JPEG', description: '高质量压缩格式' },
    { value: 'png', label: 'PNG', description: '无损压缩格式' },
    { value: 'gif', label: 'GIF', description: '支持动画' },
    { value: 'bmp', label: 'BMP', description: '位图格式' },
    { value: 'tiff', label: 'TIFF', description: '高质量图像格式' },
    { value: 'webp', label: 'WebP', description: '现代网络图像格式' },
    { value: 'ico', label: 'ICO', description: '图标格式' },
    { value: 'heic', label: 'HEIC', description: '苹果高效图像格式' },
  ]

  // 获取图片信息
  const getImageInfo = async (filePath: string): Promise<ImageInfo> => {
    try {
      const info = await invoke<ImageInfo>('get_image_info_command', {
        inputPath: filePath,
      })
      return info
    } catch (error) {
      console.error('Failed to get image info:', error)
      return {
        name: filePath.split('/').pop() || 'unknown',
        size: '未知',
        format: '未知',
        dimensions: '未知',
        has_exif: false,
        path: filePath,
      }
    }
  }

  // 获取图片详细EXIF信息
  const getImageExifData = async (
    filePath: string,
  ): Promise<ExifData | null> => {
    try {
      const exifData = await invoke<ExifData | null>('get_image_exif_data', {
        inputPath: filePath,
      })
      return exifData
    } catch (error) {
      console.error('Failed to get image EXIF data:', error)
      return null
    }
  }

  // 处理EXIF单元格鼠标悬停
  const handleExifHover = async (file: ImageFile, event: React.MouseEvent) => {
    if (!file.has_exif) return

    // 如果已经有EXIF数据，直接显示
    if (file.exif_data) {
      setHoveredExifData({
        file,
        position: { x: event.clientX, y: event.clientY },
      })
      return
    }

    // 否则获取EXIF数据
    try {
      const exifData = await getImageExifData(file.path)
      if (exifData) {
        // 更新文件列表中的EXIF数据
        setImageFiles((prev) =>
          prev.map((f) =>
            f.path === file.path ? { ...f, exif_data: exifData } : f,
          ),
        )
        setHoveredExifData({
          file: { ...file, exif_data: exifData },
          position: { x: event.clientX, y: event.clientY },
        })
      }
    } catch (error) {
      console.error('Failed to load EXIF data:', error)
    }
  }

  // 处理EXIF单元格鼠标离开
  const handleExifLeave = () => {
    setHoveredExifData(null)
  }

  // EXIF信息悬停组件
  const ExifTooltip = ({
    data,
    position,
  }: {
    data: ImageFile
    position: { x: number; y: number }
  }) => {
    if (!data.exif_data) return null

    const exif = data.exif_data
    const hasAnyData = Object.values(exif).some(
      (value) => value && value.trim() !== '',
    )

    if (!hasAnyData) return null

    return (
      <div
        className='absolute z-50 bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-600 rounded-lg shadow-lg p-4 min-w-[280px] max-w-[400px]'
        style={{
          left: position.x + 10,
          top: position.y + 10,
          transform: 'translateY(-100%)',
        }}>
        <div className='text-sm font-medium text-slate-900 dark:text-white mb-3 pb-2 border-b border-slate-200 dark:border-slate-600'>
          📷 EXIF 信息 - {data.name}
        </div>

        <div className='space-y-2 text-xs text-slate-700 dark:text-slate-300'>
          {exif.make && (
            <div className='flex justify-between'>
              <span className='font-medium'>相机制造商:</span>
              <span>{exif.make}</span>
            </div>
          )}
          {exif.model && (
            <div className='flex justify-between'>
              <span className='font-medium'>相机型号:</span>
              <span>{exif.model}</span>
            </div>
          )}
          {exif.datetime && (
            <div className='flex justify-between'>
              <span className='font-medium'>拍摄时间:</span>
              <span>{exif.datetime}</span>
            </div>
          )}
          {exif.exposure_time && (
            <div className='flex justify-between'>
              <span className='font-medium'>曝光时间:</span>
              <span>{exif.exposure_time}</span>
            </div>
          )}
          {exif.f_number && (
            <div className='flex justify-between'>
              <span className='font-medium'>光圈值:</span>
              <span>{exif.f_number}</span>
            </div>
          )}
          {exif.iso && (
            <div className='flex justify-between'>
              <span className='font-medium'>ISO 感光度:</span>
              <span>{exif.iso}</span>
            </div>
          )}
          {exif.focal_length && (
            <div className='flex justify-between'>
              <span className='font-medium'>焦距:</span>
              <span>{exif.focal_length}</span>
            </div>
          )}
          {exif.software && (
            <div className='flex justify-between'>
              <span className='font-medium'>处理软件:</span>
              <span>{exif.software}</span>
            </div>
          )}
        </div>
      </div>
    )
  }

  // 选择文件（支持多选）
  const handleSelectFiles = () => {
    // 根据选择的源格式确定文件过滤器
    let extensions: string[] = ['jpg', 'jpeg', 'png'] // 默认支持常见格式
    let filterName = '图片文件'

    if (sourceFormat !== 'auto') {
      // 查找选中格式的扩展名
      const format = supportedFormats.find((f) => f.value === sourceFormat)
      if (format) {
        extensions = [format.value]
        filterName = `${format.label}图片文件`
      }
    } else {
      // 自动模式支持所有格式
      extensions = supportedFormats.map((f) => f.value)
      filterName = '图片文件'
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
                : 'jpg') || 'jpg'
            const mimeType = `image/${fileExt}`
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
    open({
      directory: true,
      multiple: false,
    })
      .then((selected) => {
        if (selected && !Array.isArray(selected)) {
          // 遍历目录获取所有支持的图片文件
          return getFilesFromDirectory(selected).then((files) => {
            if (files.length === 0) {
              const formatList =
                sourceFormat === 'auto'
                  ? '所有支持的图片格式'
                  : `${
                      supportedFormats.find((f) => f.value === sourceFormat)
                        ?.label || sourceFormat
                    }格式`
              setError(`所选目录中没有找到 ${formatList} 的图片文件`)
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

  // 从目录中获取所有支持的图片文件
  const getFilesFromDirectory = (directoryPath: string): Promise<any[]> => {
    const files: any[] = []

    // 确定支持的文件扩展名
    let supportedExtensions: string[] = ['jpg', 'jpeg', 'png']
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
            // 如果是支持的图片文件，添加到文件列表
            return join(directoryPath, entry.name).then((filePath) => {
              const fileExt = filePath.split('.').pop()?.toLowerCase() || 'jpg'
              const mimeType = `image/${fileExt}`
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

  // 处理文件选择
  const handleFileSelect = async (files: any[]) => {
    // 确定支持的文件扩展名
    let supportedExtensions: string[] = ['jpg', 'jpeg', 'png']
    if (sourceFormat === 'auto') {
      supportedExtensions = supportedFormats.map((f) => f.value)
    } else {
      const format = supportedFormats.find((f) => f.value === sourceFormat)
      if (format) {
        supportedExtensions = [format.value]
      }
    }

    // 过滤出支持的图片文件
    const validFiles = files.filter((file) =>
      supportedExtensions.some((ext) =>
        file.name.toLowerCase().endsWith(`.${ext}`),
      ),
    )

    if (validFiles.length === 0) {
      const formatList =
        sourceFormat === 'auto'
          ? '所有支持的图片格式'
          : `${
              supportedFormats.find((f) => f.value === sourceFormat)?.label ||
              sourceFormat
            }格式`
      setError(`没有找到 ${formatList} 的图片文件`)
      return
    }

    // 处理每个文件
    const processedFiles: ImageFile[] = []

    for (const file of validFiles) {
      // 验证文件类型
      const fileExt = file.name.split('.').pop()?.toLowerCase()
      if (!fileExt || !supportedExtensions.includes(fileExt)) {
        continue
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

      // 获取图片信息
      let dimensions = '未知'
      let has_exif = false
      if (file.path) {
        try {
          const imageInfo = await getImageInfo(file.path)
          dimensions = imageInfo.dimensions
          has_exif = imageInfo.has_exif
        } catch (err) {
          console.error('Failed to get image info:', err)
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
        dimensions,
        has_exif,
        outputPath,
        sourceFormat: fileExt,
        status: 'pending',
        progress: 0,
      })
    }

    // 添加到现有文件列表，避免重复
    const existingPaths = new Set(imageFiles.map((f) => f.path))
    const newFiles = processedFiles.filter((f) => !existingPaths.has(f.path))

    if (newFiles.length > 0) {
      setImageFiles([...imageFiles, ...newFiles])
      setError('')
    }
  }

  // 开始批量转换
  const startBatchConversion = async () => {
    if (imageFiles.length === 0) {
      setError('请先选择图片文件')
      return
    }

    setIsConverting(true)
    setCurrentConvertingIndex(0)
    setError('')

    // 创建文件列表的副本以避免直接修改状态
    const filesToProcess = [...imageFiles]
    const processedFiles = [...imageFiles]

    // 逐个处理文件
    let currentIndex = 0

    const processNextFile = async () => {
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
      setImageFiles([...processedFiles])

      // 模拟转换错误用于测试
      if (filesToProcess[currentIndex].name.includes('test')) {
        const errorMessage = '测试错误：文件格式不支持或文件损坏'
        processedFiles[currentIndex] = {
          ...processedFiles[currentIndex],
          status: 'error',
          progress: 0,
          error: errorMessage,
        }
        setImageFiles([...processedFiles])
        currentIndex++
        setTimeout(processNextFile, 100)
        return
      }

      // 构建转换请求
      const request: ImageConversionRequest = {
        input_path: filesToProcess[currentIndex].path,
        output_path:
          filesToProcess[currentIndex].outputPath ||
          filesToProcess[currentIndex].name.replace(
            /\.[^/.]+$/,
            `.${targetFormat}`,
          ),
        target_format: targetFormat,
        quality: ['jpg', 'jpeg', 'webp'].includes(targetFormat)
          ? quality
          : undefined,
        width:
          resizeMode === 'width' || resizeMode === 'both' ? width : undefined,
        height:
          resizeMode === 'height' || resizeMode === 'both' ? height : undefined,
        remove_exif: removeExif,
        delete_source_file: deleteSourceFile,
      }

      try {
        // 调用Tauri后端进行图片转换
        const response = await invoke<ImageConversionResponse>(
          'convert_image',
          {
            request,
          },
        )

        // 转换成功
        processedFiles[currentIndex] = {
          ...processedFiles[currentIndex],
          status: 'completed',
          progress: 100,
          outputPath: response.output_path,
        }
      } catch (err) {
        console.error(
          `Image conversion failed for ${filesToProcess[currentIndex].name}:`,
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
      } finally {
        setImageFiles([...processedFiles])
        currentIndex++
        setTimeout(processNextFile, 100)
      }
    }

    // 开始处理第一个文件
    processNextFile()
  }

  // 删除指定文件
  const removeFile = (index: number) => {
    const file = imageFiles[index]
    if (!file) return

    // 显示确认对话框
    const confirmed = window.confirm(`确定要删除文件 "${file.name}" 吗？`)
    if (!confirmed) return

    setImageFiles((prevFiles) => prevFiles.filter((_, i) => i !== index))
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
    setImageFiles([])
    setIsConverting(false)
    setCurrentConvertingIndex(-1)
    setError('')
    setIsSuccess(false)
  }

  // 格式化文件大小
  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes'
    const k = 1024
    const sizes = ['Bytes', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  return (
    <ToolLayout
      title='图片格式转换器'
      description='支持多种主流图片格式的相互转换，可设置压缩比例和去除EXIF信息'>
      <div className='flex flex-col h-full'>
        {/* 配置区域 */}
        <div className='mb-6 p-4 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-800'>
          <h3 className='text-lg font-medium text-slate-900 dark:text-white mb-4'>
            转换设置
          </h3>

          <div className='grid grid-cols-1 md:grid-cols-2 gap-4 mb-4'>
            {/* 源格式选择 */}
            <div>
              <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                源格式
              </label>
              <select
                value={sourceFormat}
                onChange={(e) => setSourceFormat(e.target.value)}
                className='w-full px-3 py-2 border border-slate-300 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                disabled={isConverting}>
                <option value='auto'>自动检测</option>
                {supportedFormats.map((format) => (
                  <option key={`source-${format.value}`} value={format.value}>
                    {format.label} ({format.value})
                  </option>
                ))}
              </select>
              <p className='mt-1 text-xs text-slate-500 dark:text-slate-400'>
                选择源图片文件格式，自动检测可支持所有格式
              </p>
            </div>

            {/* 目标格式选择 */}
            <div>
              <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                目标格式
              </label>
              <select
                value={targetFormat}
                onChange={(e) => setTargetFormat(e.target.value)}
                className='w-full px-3 py-2 border border-slate-300 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                disabled={isConverting}>
                {supportedFormats.map((format) => (
                  <option key={`target-${format.value}`} value={format.value}>
                    {format.label} ({format.value})
                  </option>
                ))}
              </select>
              <p className='mt-1 text-xs text-slate-500 dark:text-slate-400'>
                转换后的图片格式
              </p>
            </div>
          </div>

          {/* 图片质量设置 */}
          {['jpg', 'jpeg', 'webp'].includes(targetFormat) && (
            <div className='mb-4'>
              <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                图片质量: {quality}%
              </label>
              <input
                type='range'
                min='1'
                max='100'
                value={quality}
                onChange={(e) => setQuality(parseInt(e.target.value))}
                className='w-full h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer dark:bg-slate-700'
                disabled={isConverting}
              />
              <p className='mt-1 text-xs text-slate-500 dark:text-slate-400'>
                质量越高文件越大，建议设置在 70-90 之间
              </p>
            </div>
          )}

          {/* 尺寸调整设置 */}
          <div className='mb-4'>
            <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
              尺寸调整
            </label>
            <div className='space-y-2'>
              <div className='flex items-center space-x-4'>
                <label className='flex items-center cursor-pointer'>
                  <input
                    type='radio'
                    name='resizeMode'
                    value='none'
                    checked={resizeMode === 'none'}
                    onChange={() => setResizeMode('none')}
                    className='w-4 h-4 text-primary-600 focus:ring-primary-500 border-slate-300'
                    disabled={isConverting}
                  />
                  <span className='ml-2 text-sm text-slate-700 dark:text-slate-300'>
                    保持原尺寸
                  </span>
                </label>

                <label className='flex items-center cursor-pointer'>
                  <input
                    type='radio'
                    name='resizeMode'
                    value='width'
                    checked={resizeMode === 'width'}
                    onChange={() => setResizeMode('width')}
                    className='w-4 h-4 text-primary-600 focus:ring-primary-500 border-slate-300'
                    disabled={isConverting}
                  />
                  <span className='ml-2 text-sm text-slate-700 dark:text-slate-300'>
                    按宽度调整
                  </span>
                </label>

                <label className='flex items-center cursor-pointer'>
                  <input
                    type='radio'
                    name='resizeMode'
                    value='height'
                    checked={resizeMode === 'height'}
                    onChange={() => setResizeMode('height')}
                    className='w-4 h-4 text-primary-600 focus:ring-primary-500 border-slate-300'
                    disabled={isConverting}
                  />
                  <span className='ml-2 text-sm text-slate-700 dark:text-slate-300'>
                    按高度调整
                  </span>
                </label>

                <label className='flex items-center cursor-pointer'>
                  <input
                    type='radio'
                    name='resizeMode'
                    value='both'
                    checked={resizeMode === 'both'}
                    onChange={() => setResizeMode('both')}
                    className='w-4 h-4 text-primary-600 focus:ring-primary-500 border-slate-300'
                    disabled={isConverting}
                  />
                  <span className='ml-2 text-sm text-slate-700 dark:text-slate-300'>
                    指定宽高
                  </span>
                </label>
              </div>

              {(resizeMode === 'width' || resizeMode === 'both') && (
                <div className='flex items-center space-x-2'>
                  <span className='text-sm text-slate-600 dark:text-slate-400'>
                    宽度:
                  </span>
                  <input
                    type='number'
                    min='1'
                    max='10000'
                    value={width}
                    onChange={(e) => setWidth(parseInt(e.target.value) || 1920)}
                    className='w-20 px-2 py-1 border border-slate-300 rounded text-sm dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                    disabled={isConverting}
                  />
                  <span className='text-sm text-slate-500 dark:text-slate-400'>
                    px
                  </span>
                </div>
              )}

              {(resizeMode === 'height' || resizeMode === 'both') && (
                <div className='flex items-center space-x-2'>
                  <span className='text-sm text-slate-600 dark:text-slate-400'>
                    高度:
                  </span>
                  <input
                    type='number'
                    min='1'
                    max='10000'
                    value={height}
                    onChange={(e) =>
                      setHeight(parseInt(e.target.value) || 1080)
                    }
                    className='w-20 px-2 py-1 border border-slate-300 rounded text-sm dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                    disabled={isConverting}
                  />
                  <span className='text-sm text-slate-500 dark:text-slate-400'>
                    px
                  </span>
                </div>
              )}
            </div>
          </div>

          {/* 其他选项 */}
          <div className='mb-4'>
            <label className='flex items-center cursor-pointer'>
              <input
                type='checkbox'
                checked={removeExif}
                onChange={(e) => setRemoveExif(e.target.checked)}
                className='w-4 h-4 text-primary-600 focus:ring-primary-500 border-slate-300 rounded'
                disabled={isConverting}
              />
              <span className='ml-2 text-sm font-medium text-slate-700 dark:text-slate-300'>
                移除EXIF信息（保护隐私，默认不移除）
              </span>
            </label>
          </div>

          {/* 删除源文件设置 */}
          <div className='flex items-center space-x-3'>
            <label className='flex items-center cursor-pointer'>
              <input
                type='checkbox'
                checked={deleteSourceFile}
                onChange={(e) => setDeleteSourceFile(e.target.checked)}
                className='w-4 h-4 text-red-600 focus:ring-red-500 border-slate-300 rounded'
                disabled={isConverting}
              />
              <span className='ml-2 text-sm font-medium text-red-700 dark:text-red-400'>
                转换成功后删除源文件（谨慎操作）
              </span>
            </label>
          </div>

          {/* 输出目录设置 */}
          <div className='space-y-3'>
            <div className='flex items-center space-x-3'>
              <label className='flex items-center cursor-pointer'>
                <input
                  type='checkbox'
                  checked={useCustomOutputDir}
                  onChange={(e) => setUseCustomOutputDir(e.target.checked)}
                  className='w-4 h-4 text-primary-600 focus:ring-primary-500 border-slate-300 rounded'
                  disabled={isConverting}
                />
                <span className='ml-2 text-sm font-medium text-slate-700 dark:text-slate-300'>
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
                    className='w-full px-3 py-2 border border-slate-300 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                    readOnly
                  />
                </div>
                <Button
                  onClick={handleSelectOutputDirectory}
                  variant='secondary'
                  size='sm'
                  disabled={isConverting}>
                  选择目录
                </Button>
                {customOutputDir && (
                  <Button
                    onClick={() => setCustomOutputDir('')}
                    variant='secondary'
                    size='sm'
                    disabled={isConverting}>
                    清除
                  </Button>
                )}
              </div>
            )}

            {useCustomOutputDir && customOutputDir && (
              <div className='text-xs text-slate-500 dark:text-slate-400'>
                转换后的文件将保存到: {customOutputDir}
              </div>
            )}
          </div>
        </div>

        {/* 文件列表区域 */}
        <div className='mb-8 flex flex-col' style={{ minHeight: '500px' }}>
          <div className='flex justify-between items-center mb-3 flex-shrink-0'>
            <h3 className='text-lg font-medium text-slate-900 dark:text-white'>
              待转换文件列表 ({imageFiles.length}个文件)
            </h3>
            <div className='flex space-x-2'>
              <Button onClick={handleSelectFiles} variant='secondary' size='sm'>
                添加文件
              </Button>
              <Button
                onClick={handleSelectDirectory}
                variant='secondary'
                size='sm'>
                添加目录
              </Button>
            </div>
          </div>

          {imageFiles.length > 0 ? (
            <div
              className='border border-slate-200 dark:border-slate-700 rounded-lg p-4 bg-slate-50 dark:bg-slate-800 flex-1 overflow-hidden'
              style={{ minHeight: '300px', maxHeight: 'calc(100vh - 400px)' }}>
              <div className='overflow-y-auto h-full'>
                <table className='min-w-full divide-y divide-slate-200 dark:divide-slate-700'>
                  <thead className='bg-slate-100 dark:bg-slate-700 sticky top-0'>
                    <tr>
                      <th className='px-4 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-300 uppercase tracking-wider'>
                        路径信息
                      </th>
                      <th className='px-4 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-300 uppercase tracking-wider'>
                        大小
                      </th>
                      <th className='px-4 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-300 uppercase tracking-wider'>
                        尺寸
                      </th>
                      <th className='px-4 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-300 uppercase tracking-wider'>
                        EXIF
                      </th>
                      <th className='px-4 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-300 uppercase tracking-wider'>
                        状态
                      </th>
                      <th className='px-4 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-300 uppercase tracking-wider'>
                        错误信息
                      </th>
                      <th className='px-4 py-2 text-center text-xs font-medium text-slate-500 dark:text-slate-300 uppercase tracking-wider'>
                        操作
                      </th>
                    </tr>
                  </thead>
                  <tbody className='divide-y divide-slate-200 dark:divide-slate-700'>
                    {imageFiles.map((file, index) => (
                      <tr
                        key={index}
                        className={
                          currentConvertingIndex === index
                            ? 'bg-primary-50 dark:bg-primary-900/20'
                            : ''
                        }>
                        <td className='px-4 py-2 text-sm'>
                          <div className='space-y-1'>
                            {/* 源文件路径 */}
                            <div className='flex items-start space-x-1'>
                              <span className='text-xs text-slate-500 dark:text-slate-400 font-medium min-w-[45px]'>
                                源文件:
                              </span>
                              <div className='flex items-center space-x-1 flex-1 min-w-0'>
                                <span
                                  className='text-xs text-slate-700 dark:text-slate-300 truncate inline-block cursor-pointer hover:text-primary-600 dark:hover:text-primary-400'
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
                                  className='text-slate-400 hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300 flex-shrink-0'
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
                              <span className='text-xs text-slate-500 dark:text-slate-400 font-medium min-w-[45px]'>
                                转换后:
                              </span>
                              <div className='flex items-center space-x-1 flex-1 min-w-0'>
                                {file.status === 'completed' &&
                                file.outputPath ? (
                                  <>
                                    <span
                                      className='text-xs text-primary-600 dark:text-primary-400 truncate inline-block cursor-pointer hover:text-primary-800 dark:hover:text-primary-300'
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
                                      className='text-primary-600 hover:text-primary-800 dark:text-primary-400 dark:hover:text-primary-200 flex-shrink-0'
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
                                  <span className='text-xs text-slate-400 dark:text-slate-500'>
                                    未转换
                                  </span>
                                )}
                              </div>
                            </div>
                          </div>
                        </td>
                        <td className='px-4 py-2 text-sm text-slate-900 dark:text-slate-100'>
                          {formatFileSize(file.size)}
                        </td>
                        <td className='px-4 py-2 text-sm text-slate-900 dark:text-slate-100'>
                          {file.dimensions || '未知'}
                        </td>
                        <td
                          className='px-4 py-2 text-sm text-slate-900 dark:text-slate-100 cursor-help'
                          onMouseEnter={(e) => handleExifHover(file, e)}
                          onMouseLeave={handleExifLeave}>
                          {file.has_exif ? (
                            <span className='text-orange-600 dark:text-orange-400 hover:text-orange-700 dark:hover:text-orange-300 underline'>
                              包含
                            </span>
                          ) : (
                            <span className='text-slate-500 dark:text-slate-400'>
                              无
                            </span>
                          )}
                        </td>
                        <td className='px-4 py-2 text-sm'>
                          {file.status === 'pending' && (
                            <span className='text-yellow-600 dark:text-yellow-400'>
                              等待中
                            </span>
                          )}
                          {file.status === 'converting' && (
                            <span className='text-primary-600 dark:text-primary-400'>
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
                            <span className='text-slate-400 dark:text-slate-500 text-xs'>
                              -
                            </span>
                          )}
                        </td>
                        <td className='px-4 py-2 text-sm text-center'>
                          <button
                            onClick={() => removeFile(index)}
                            disabled={
                              isConverting && file.status === 'converting'
                            }
                            className={`p-1 rounded-lg transition-colors ${
                              isConverting && file.status === 'converting'
                                ? 'text-slate-400 cursor-not-allowed'
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
            <div className='border border-slate-200 dark:border-slate-700 rounded-lg p-8 bg-slate-50 dark:bg-slate-800 text-center'>
              <p className='text-slate-500 dark:text-slate-400'>
                暂无待转换文件，请点击右上角按钮添加文件或目录
              </p>
            </div>
          )}
        </div>

        {/* 转换控制区域 */}
        {imageFiles.length > 0 && (
          <div className='mb-6 flex-shrink-0'>
            <h3 className='text-lg font-medium text-slate-900 dark:text-white mb-3'>
              转换控制
            </h3>
            <div className='border border-slate-200 dark:border-slate-700 rounded-lg p-4 bg-slate-50 dark:bg-slate-800'>
              {/* 整体进度条 */}
              {isConverting && currentConvertingIndex >= 0 && (
                <div className='mb-4'>
                  <div className='flex justify-between items-center mb-2'>
                    <p className='text-sm text-slate-600 dark:text-slate-400'>
                      总体进度: {currentConvertingIndex + 1} /{' '}
                      {imageFiles.length} 个文件
                    </p>
                    <p className='text-sm text-slate-600 dark:text-slate-400'>
                      {Math.round(
                        ((currentConvertingIndex +
                          (imageFiles[currentConvertingIndex]?.progress || 0) /
                            100) /
                          imageFiles.length) *
                          100,
                      )}
                      %
                    </p>
                  </div>
                  <div className='w-full bg-slate-200 dark:bg-slate-700 rounded-full h-2'>
                    <div
                      className='bg-primary-600 h-2 rounded-full transition-all duration-200'
                      style={{
                        width: `${
                          ((currentConvertingIndex +
                            (imageFiles[currentConvertingIndex]?.progress ||
                              0) /
                              100) /
                            imageFiles.length) *
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
                  variant={copied ? 'ghost' : 'primary'}
                  disabled={
                    isConverting ||
                    imageFiles.length === 0 ||
                    (useCustomOutputDir && !customOutputDir)
                  }
                  className='flex-1'>
                  {isConverting ? '转换中...' : '开始转换'}
                </Button>

                <Button
                  onClick={clearFileList}
                  variant='secondary'
                  className='flex-1'>
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

        {/* EXIF信息悬停提示 */}
        {hoveredExifData && (
          <ExifTooltip
            data={hoveredExifData.file}
            position={hoveredExifData.position}
          />
        )}
      </div>
    </ToolLayout>
  )
}

export default ImageConverter
