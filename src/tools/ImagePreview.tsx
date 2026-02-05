import React, { useEffect, useRef, useState } from 'react'
import { Button } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'

// 图片信息接口
interface ImageInfo {
  width: number
  height: number
  format: string
  size: number // 字节大小
  dataUrl?: string
}

// 缩放和位置状态
interface ViewState {
  scale: number
  positionX: number
  positionY: number
  isDragging: boolean
  lastX: number
  lastY: number
}

// 保存选项
interface SaveOptions {
  format: 'png' | 'jpeg' | 'gif' | 'webp'
  quality: number
  filename: string
}

const ImagePreview: React.FC = () => {
  // 输入状态
  const [inputUrl, setInputUrl] = useState<string>('')
  const [isLoading, setIsLoading] = useState<boolean>(false)
  const [error, setError] = useState<string>('')
  const [isSaving, setIsSaving] = useState<boolean>(false)

  // 图片数据
  const [imageSrc, setImageSrc] = useState<string>('')
  const [imageInfo, setImageInfo] = useState<ImageInfo | null>(null)

  // 视图状态
  const [viewState, setViewState] = useState<ViewState>({
    scale: 1,
    positionX: 0,
    positionY: 0,
    isDragging: false,
    lastX: 0,
    lastY: 0,
  })

  // 全屏状态
  const [isFullscreen, setIsFullscreen] = useState<boolean>(false)

  // 保存选项
  const [saveOptions, setSaveOptions] = useState<SaveOptions>({
    format: 'png',
    quality: 85,
    filename: 'image',
  })

  // refs
  const imageContainerRef = useRef<HTMLDivElement>(null)
  const imgRef = useRef<HTMLImageElement>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)

  const { copyToClipboard } = useCopyToClipboard()

  // 解析 data: URL 获取基本信息
  const parseDataUrl = (
    dataUrl: string,
  ): { format: string; size: number } | null => {
    try {
      const match = dataUrl.match(/^data:image\/(\w+);base64,(.+)$/)
      if (!match) return null

      const base64Data = match[2]
      const format = match[1].toLowerCase()

      // 计算文件大小
      const size = Math.round((base64Data.length * 3) / 4)

      return { format, size }
    } catch {
      return null
    }
  }

  // 加载图片
  const loadImage = async (src: string) => {
    if (!src) return

    setIsLoading(true)
    setError('')

    try {
      // 如果是 data: URL，解析基本信息
      if (src.startsWith('data:')) {
        const parsed = parseDataUrl(src)
        if (!parsed) {
          throw new Error('无效的 data: URL 格式')
        }

        // 创建图片对象获取尺寸
        const img = new Image()
        img.src = src

        await new Promise<void>((resolve, reject) => {
          img.onload = () => resolve()
          img.onerror = () => reject(new Error('图片加载失败'))
        })

        setImageInfo({
          width: img.naturalWidth,
          height: img.naturalHeight,
          format: parsed.format,
          size: parsed.size,
          dataUrl: src,
        })
      } else {
        // 远程图片 URL
        const img = new Image()
        img.crossOrigin = 'anonymous'
        img.src = src

        await new Promise<void>((resolve, reject) => {
          img.onload = () => resolve()
          img.onerror = () =>
            reject(new Error('图片加载失败，请检查 URL 是否正确'))
        })

        setImageInfo({
          width: img.naturalWidth,
          height: img.naturalHeight,
          format: 'unknown',
          size: 0,
        })
      }

      setImageSrc(src)
      // 重置视图状态
      setViewState({
        scale: 1,
        positionX: 0,
        positionY: 0,
        isDragging: false,
        lastX: 0,
        lastY: 0,
      })
    } catch (err) {
      setError(err instanceof Error ? err.message : '图片加载失败')
      setImageSrc('')
      setImageInfo(null)
    } finally {
      setIsLoading(false)
    }
  }

  // 处理 URL 输入
  const handleUrlSubmit = () => {
    if (!inputUrl.trim()) {
      setError('请输入图片 URL 或 data: URL')
      return
    }
    loadImage(inputUrl.trim())
  }

  // 处理粘贴事件
  const handlePaste = async (e: React.ClipboardEvent) => {
    const items = e.clipboardData.items
    for (let i = 0; i < items.length; i++) {
      const item = items[i]
      if (item.type.indexOf('image') !== -1) {
        const file = item.getAsFile()
        if (file) {
          const reader = new FileReader()
          reader.onload = (event) => {
            const dataUrl = event.target?.result as string
            setInputUrl(dataUrl)
            loadImage(dataUrl)
          }
          reader.readAsDataURL(file)
          break
        }
      }
    }
  }

  // 处理文件上传
  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    if (!file.type.startsWith('image/')) {
      setError('请选择有效的图片文件')
      return
    }

    const reader = new FileReader()
    reader.onload = (event) => {
      const dataUrl = event.target?.result as string
      setInputUrl(dataUrl)
      loadImage(dataUrl)
    }
    reader.readAsDataURL(file)
  }

  // 缩放功能
  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault()
    const delta = e.deltaY > 0 ? -0.1 : 0.1
    const newScale = Math.max(0.5, Math.min(5, viewState.scale + delta))

    if (imageContainerRef.current && imgRef.current) {
      const rect = imageContainerRef.current.getBoundingClientRect()

      // 计算缩放中心点相对于图片的位置
      const scaleFactor = newScale / viewState.scale
      const centerX = e.clientX - rect.left
      const centerY = e.clientY - rect.top

      const newPosX = centerX - scaleFactor * (centerX - viewState.positionX)
      const newPosY = centerY - scaleFactor * (centerY - viewState.positionY)

      setViewState({
        ...viewState,
        scale: newScale,
        positionX: newPosX,
        positionY: newPosY,
      })
    } else {
      setViewState({ ...viewState, scale: newScale })
    }
  }

  // 鼠标拖拽
  const handleMouseDown = (e: React.MouseEvent) => {
    setViewState({
      ...viewState,
      isDragging: true,
      lastX: e.clientX - viewState.positionX,
      lastY: e.clientY - viewState.positionY,
    })
  }

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!viewState.isDragging) return

    setViewState({
      ...viewState,
      positionX: e.clientX - viewState.lastX,
      positionY: e.clientY - viewState.lastY,
    })
  }

  const handleMouseUp = () => {
    setViewState({ ...viewState, isDragging: false })
  }

  // 双击缩放
  const handleDoubleClick = () => {
    if (viewState.scale === 1) {
      setViewState({
        ...viewState,
        scale: 2,
        positionX: 0,
        positionY: 0,
      })
    } else {
      setViewState({
        ...viewState,
        scale: 1,
        positionX: 0,
        positionY: 0,
      })
    }
  }

  // 重置视图
  const resetView = () => {
    setViewState({
      scale: 1,
      positionX: 0,
      positionY: 0,
      isDragging: false,
      lastX: 0,
      lastY: 0,
    })
  }

  // 保存图片
  const saveImage = async () => {
    if (!imageSrc || !imgRef.current) {
      setError('没有可保存的图片')
      return
    }

    setIsSaving(true)
    setError('正在准备保存，将弹出保存对话框，请选择保存目录...')

    try {
      // 清空之前的状态
      setError('')

      const canvas = canvasRef.current
      if (!canvas) {
        setError('初始化画布失败')
        setIsSaving(false)
        return
      }

      const ctx = canvas.getContext('2d')
      if (!ctx) {
        setError('获取画布上下文失败')
        setIsSaving(false)
        return
      }

      // 设置画布尺寸
      canvas.width = imgRef.current.naturalWidth
      canvas.height = imgRef.current.naturalHeight

      // 绘制图片
      ctx.drawImage(imgRef.current, 0, 0)

      // 转换为 Blob
      const mimeType = `image/${saveOptions.format}`
      const quality = ['jpeg', 'webp'].includes(saveOptions.format)
        ? saveOptions.quality / 100
        : undefined

      // 使用 Promise 包装 toBlob
      const blob = await new Promise<Blob | null>((resolve) => {
        canvas.toBlob((blob) => resolve(blob), mimeType, quality)
      })

      if (!blob) {
        setError('转换图片失败，请尝试其他格式')
        setIsSaving(false)
        return
      }

      // 导入 Tauri API
      const { save } = await import('@tauri-apps/plugin-dialog')
      const { writeFile } = await import('@tauri-apps/plugin-fs')

      // 打开保存对话框
      const filePath = await save({
        defaultPath: `${saveOptions.filename}.${saveOptions.format}`,
        filters: [
          {
            name: '图片文件',
            extensions: [saveOptions.format],
          },
        ],
      })

      if (filePath) {
        // 将 Blob 转换为 Uint8Array
        const arrayBuffer = await blob.arrayBuffer()
        const uint8Array = new Uint8Array(arrayBuffer)

        // 写入文件
        await writeFile(filePath, uint8Array)

        setError('')
        setIsSaving(false)
      } else {
        // 用户取消保存
        setIsSaving(false)
      }
    } catch (err) {
      console.error('保存图片错误:', err)
      setError(
        err instanceof Error
          ? `保存失败: ${err.message}`
          : '保存图片时发生未知错误',
      )
      setIsSaving(false)
    }
  }

  // 格式化文件大小
  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes'
    const k = 1024
    const sizes = ['Bytes', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  // 获取缩放百分比
  const getScalePercentage = (): string => {
    return Math.round(viewState.scale * 100) + '%'
  }

  // 自动加载输入的 URL
  useEffect(() => {
    if (inputUrl && inputUrl !== imageSrc) {
      const timer = setTimeout(() => {
        loadImage(inputUrl)
      }, 500)
      return () => clearTimeout(timer)
    }
  }, [inputUrl])

  // 键盘快捷键
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isFullscreen) {
        setIsFullscreen(false)
      }
      if (e.ctrlKey || e.metaKey) {
        if (e.key === '=' || e.key === '+') {
          e.preventDefault()
          setViewState({
            ...viewState,
            scale: Math.min(5, viewState.scale + 0.1),
          })
        } else if (e.key === '-') {
          e.preventDefault()
          setViewState({
            ...viewState,
            scale: Math.max(0.5, viewState.scale - 0.1),
          })
        } else if (e.key === '0') {
          e.preventDefault()
          resetView()
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [viewState.scale, isFullscreen])

  return (
    <ToolLayout
      title='图片预览器'
      description='支持 data: URL、图片文件预览，提供缩放、拖拽和格式转换功能'>
      <div className='flex flex-col h-full' onPaste={handlePaste}>
        {/* 输入区域 */}
        <div className='flex-shrink-0 mb-6 space-y-4'>
          {/* URL 输入框 */}
          <div className='flex space-x-2'>
            <input
              type='text'
              value={inputUrl}
              onChange={(e) => setInputUrl(e.target.value)}
              placeholder='粘贴图片 URL 或 data: URL...'
              className='flex-1 px-4 py-2 border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'
              onKeyDown={(e) => e.key === 'Enter' && handleUrlSubmit()}
            />
            <Button onClick={handleUrlSubmit} disabled={!inputUrl.trim()}>
              加载
            </Button>
            <label className='px-4 py-2 bg-slate-200 dark:bg-slate-600 text-slate-700 dark:text-slate-300 rounded-lg hover:bg-slate-300 dark:hover:bg-slate-500 cursor-pointer transition-colors'>
              上传文件
              <input
                type='file'
                accept='image/*'
                onChange={handleFileSelect}
                className='hidden'
              />
            </label>
          </div>

          {/* 快捷提示 */}
          <div className='text-sm text-slate-500 dark:text-slate-400'>
            💡 提示：直接粘贴图片（Ctrl+V）可快速预览，或使用 Ctrl+滚轮 缩放
          </div>

          {/* 错误信息 */}
          {error && (
            <div className='p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-red-700 dark:text-red-400 text-sm'>
              {error}
            </div>
          )}
        </div>

        <div className='flex-1 grid grid-cols-1 lg:grid-cols-4 gap-6 min-h-0'>
          {/* 图片预览区域 */}
          <div className='lg:col-span-3 min-h-0'>
            <div
              ref={imageContainerRef}
              className='h-full border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-800 overflow-hidden relative'
              onWheel={handleWheel}
              onMouseMove={handleMouseMove}
              onMouseUp={handleMouseUp}
              onMouseLeave={handleMouseUp}>
              {isLoading ? (
                <div className='flex items-center justify-center h-full'>
                  <div className='text-slate-500 dark:text-slate-400'>
                    加载中...
                  </div>
                </div>
              ) : imageSrc ? (
                <div className='h-full flex items-center justify-center overflow-hidden'>
                  <img
                    ref={imgRef}
                    src={imageSrc}
                    alt='预览图片'
                    className={`max-w-none transition-transform duration-100 ${
                      viewState.isDragging ? 'cursor-grabbing' : 'cursor-grab'
                    }`}
                    style={{
                      transform: `translate(${viewState.positionX}px, ${viewState.positionY}px) scale(${viewState.scale})`,
                    }}
                    onMouseDown={handleMouseDown}
                    onDoubleClick={handleDoubleClick}
                    draggable={false}
                  />
                </div>
              ) : (
                <div className='flex items-center justify-center h-full'>
                  <div className='text-center text-slate-400 dark:text-slate-500'>
                    <svg
                      className='w-16 h-16 mx-auto mb-4'
                      fill='none'
                      stroke='currentColor'
                      viewBox='0 0 24 24'>
                      <path
                        strokeLinecap='round'
                        strokeLinejoin='round'
                        strokeWidth={2}
                        d='M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z'
                      />
                    </svg>
                    <p>粘贴图片或输入 URL 开始预览</p>
                  </div>
                </div>
              )}

              {/* 控制按钮 */}
              {imageSrc && (
                <div className='absolute top-4 right-4 flex space-x-2'>
                  <button
                    onClick={resetView}
                    className='px-3 py-1 bg-white dark:bg-slate-700 shadow rounded-lg text-sm text-slate-900 dark:text-white hover:bg-slate-50 dark:hover:bg-slate-600 transition-colors'>
                    重置
                  </button>
                  <button
                    onClick={() => setIsFullscreen(true)}
                    className='px-3 py-1 bg-white dark:bg-slate-700 shadow rounded-lg text-sm text-slate-900 dark:text-white hover:bg-slate-50 dark:hover:bg-slate-600 transition-colors'>
                    全屏
                  </button>
                  <button
                    onClick={() => imageSrc && copyToClipboard(imageSrc)}
                    className='px-3 py-1 bg-white dark:bg-slate-700 shadow rounded-lg text-sm text-slate-900 dark:text-white hover:bg-slate-50 dark:hover:bg-slate-600 transition-colors'>
                    复制 URL
                  </button>
                </div>
              )}

              {/* 缩放指示器 */}
              {imageSrc && (
                <div className='absolute bottom-4 left-4 px-3 py-1 bg-black/50 text-white rounded-lg text-sm'>
                  {getScalePercentage()}
                </div>
              )}
            </div>

            {/* 隐藏的 canvas 用于保存 */}
            <canvas ref={canvasRef} className='hidden' />
          </div>

          {/* 信息面板 */}
          <div className='lg:col-span-1 min-h-0'>
            <div className='h-full border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-800 p-4 space-y-6'>
              {/* 图片信息 */}
              <div>
                <h3 className='text-lg font-medium text-slate-900 dark:text-white mb-3'>
                  图片信息
                </h3>
                {imageInfo ? (
                  <div className='space-y-2 text-sm'>
                    <div className='flex justify-between'>
                      <span className='text-slate-600 dark:text-slate-400'>
                        尺寸:
                      </span>
                      <span className='text-slate-900 dark:text-white font-medium'>
                        {imageInfo.width} × {imageInfo.height}
                      </span>
                    </div>
                    <div className='flex justify-between'>
                      <span className='text-slate-600 dark:text-slate-400'>
                        格式:
                      </span>
                      <span className='text-slate-900 dark:text-white font-medium uppercase'>
                        {imageInfo.format}
                      </span>
                    </div>
                    <div className='flex justify-between'>
                      <span className='text-slate-600 dark:text-slate-400'>
                        大小:
                      </span>
                      <span className='text-slate-900 dark:text-white font-medium'>
                        {formatFileSize(imageInfo.size)}
                      </span>
                    </div>
                  </div>
                ) : (
                  <p className='text-sm text-slate-500 dark:text-slate-400'>
                    暂无图片信息
                  </p>
                )}
              </div>

              {/* 视图控制 */}
              <div>
                <h3 className='text-lg font-medium text-slate-900 dark:text-white mb-3'>
                  视图控制
                </h3>
                <div className='space-y-2'>
                  <div className='flex items-center justify-between text-sm'>
                    <span className='text-slate-600 dark:text-slate-400'>
                      缩放:
                    </span>
                    <span className='text-slate-900 dark:text-white font-medium'>
                      {getScalePercentage()}
                    </span>
                  </div>
                  <div className='space-y-2'>
                    <button
                      onClick={() =>
                        setViewState({
                          ...viewState,
                          scale: Math.min(5, viewState.scale + 0.2),
                        })
                      }
                      className='w-full px-3 py-2 bg-white dark:bg-slate-700 shadow rounded text-sm text-slate-900 dark:text-white hover:bg-slate-50 dark:hover:bg-slate-600 transition-colors'>
                      放大
                    </button>
                    <button
                      onClick={() =>
                        setViewState({
                          ...viewState,
                          scale: Math.max(0.5, viewState.scale - 0.2),
                        })
                      }
                      className='w-full px-3 py-2 bg-white dark:bg-slate-700 shadow rounded text-sm text-slate-900 dark:text-white hover:bg-slate-50 dark:hover:bg-slate-600 transition-colors'>
                      缩小
                    </button>
                    <button
                      onClick={resetView}
                      className='w-full px-3 py-2 bg-white dark:bg-slate-700 shadow rounded text-sm text-slate-900 dark:text-white hover:bg-slate-50 dark:hover:bg-slate-600 transition-colors'>
                      100%
                    </button>
                  </div>
                </div>
              </div>

              {/* 保存设置 */}
              <div>
                <h3 className='text-lg font-medium text-slate-900 dark:text-white mb-3'>
                  保存设置
                </h3>
                <div className='space-y-3'>
                  <div>
                    <label className='block text-sm text-slate-600 dark:text-slate-400 mb-1'>
                      格式
                    </label>
                    <select
                      value={saveOptions.format}
                      onChange={(e) =>
                        setSaveOptions({
                          ...saveOptions,
                          format: e.target.value as any,
                        })
                      }
                      className='w-full px-3 py-2 bg-white dark:bg-slate-700 border border-slate-300 dark:border-slate-600 rounded text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 dark:text-white'>
                      <option value='png'>PNG (无损)</option>
                      <option value='jpeg'>JPEG (有损)</option>
                      <option value='gif'>GIF (动图)</option>
                      <option value='webp'>WebP (现代格式)</option>
                    </select>
                  </div>

                  {['jpeg', 'webp'].includes(saveOptions.format) && (
                    <div>
                      <label className='block text-sm text-slate-600 dark:text-slate-400 mb-1'>
                        质量: {saveOptions.quality}%
                      </label>
                      <input
                        type='range'
                        min='10'
                        max='100'
                        value={saveOptions.quality}
                        onChange={(e) =>
                          setSaveOptions({
                            ...saveOptions,
                            quality: parseInt(e.target.value),
                          })
                        }
                        className='w-full'
                      />
                    </div>
                  )}

                  <div>
                    <label className='block text-sm text-slate-600 dark:text-slate-400 mb-1'>
                      文件名
                    </label>
                    <input
                      type='text'
                      value={saveOptions.filename}
                      onChange={(e) =>
                        setSaveOptions({
                          ...saveOptions,
                          filename: e.target.value,
                        })
                      }
                      className='w-full px-3 py-2 bg-white dark:bg-slate-700 border border-slate-300 dark:border-slate-600 rounded text-sm focus:outline-none focus:ring-2 focus:ring-primary-500 dark:text-white'
                    />
                  </div>

                  <Button
                    onClick={saveImage}
                    disabled={!imageSrc || isSaving}
                    className='w-full'>
                    {isSaving ? '正在保存...' : '保存图片到指定目录'}
                  </Button>
                  {!isSaving && (
                    <p className='text-xs text-slate-500 dark:text-slate-400 text-center'>
                      点击后将弹出保存对话框，您可以自由选择保存目录和文件名
                    </p>
                  )}
                  {isSaving && (
                    <div className='flex items-center justify-center mt-2'>
                      <div className='animate-spin rounded-full h-4 w-4 border-b-2 border-primary-500'></div>
                      <span className='ml-2 text-xs text-primary-500 dark:text-primary-400'>
                        即将弹出保存对话框...
                      </span>
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* 全屏模式 */}
        {isFullscreen && (
          <div className='fixed inset-0 z-50 bg-black bg-opacity-90 flex items-center justify-center'>
            <button
              onClick={() => setIsFullscreen(false)}
              className='absolute top-4 right-4 px-4 py-2 bg-white/10 hover:bg-white/20 rounded-lg text-white transition-colors'>
              退出全屏 (ESC)
            </button>
            {imageSrc && (
              <img
                src={imageSrc}
                alt='全屏预览'
                className='max-w-full max-h-full object-contain'
                style={{
                  transform: `translate(${viewState.positionX}px, ${viewState.positionY}px) scale(${viewState.scale})`,
                }}
                onMouseDown={handleMouseDown}
                onDoubleClick={() => setIsFullscreen(false)}
                draggable={false}
              />
            )}
            <div className='absolute bottom-4 left-4 px-3 py-1 bg-black/50 text-white rounded-lg text-sm'>
              {getScalePercentage()}
            </div>
          </div>
        )}
      </div>
    </ToolLayout>
  )
}

export default ImagePreview
