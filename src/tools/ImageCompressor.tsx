import { invoke } from '@tauri-apps/api/core'
import { dirname, basename, join } from '@tauri-apps/api/path'
import { open } from '@tauri-apps/plugin-dialog'
import { readDir, size } from '@tauri-apps/plugin-fs'
import React, { useEffect, useMemo, useRef, useState } from 'react'
import { Button } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'

// ===== 类型定义 (与 Rust 端结构对应, 嵌套结构体保持 snake_case) =====

interface CompressRequest {
  input_path: string
  output_path: string
  mode: 'lossless' | 'quality'
  quality?: number
  max_width?: number
  max_height?: number
  output_format: string
  remove_exif: boolean
  png_optimize_level?: number
}

interface CompressResponse {
  success: boolean
  output_path: string
  message: string
  original_size: number
  compressed_size: number
  compression_ratio: number
  output_width: number
  output_height: number
}

interface ImageDataUrl {
  data_url: string
  width: number
  height: number
}

interface SourceImageInfo {
  name: string
  size: string
  format: string
  dimensions: string
  has_exif: boolean
  path: string
}

interface ImageFile {
  name: string
  path: string
  size: number
  format: string
  dimensions: string
  outputPath?: string
  compressedSize?: number
  ratio?: number
  outDimensions?: string
  status: 'pending' | 'compressing' | 'completed' | 'error'
  error?: string
}

// 支持的输入格式
const SUPPORTED_EXTENSIONS = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'tiff', 'tif', 'webp']

// 输出格式选项
const OUTPUT_FORMATS = [
  { value: 'keep', label: '保持原格式' },
  { value: 'png', label: 'PNG' },
  { value: 'jpeg', label: 'JPEG' },
  { value: 'webp', label: 'WebP' },
]

// 格式化文件大小
const formatFileSize = (bytes: number): string => {
  if (!bytes) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

// 根据输出格式设置计算输出文件名
const computeOutputName = (baseName: string, outputFormat: string): string => {
  if (outputFormat === 'keep') return baseName
  const ext = outputFormat === 'jpeg' ? 'jpg' : outputFormat
  return baseName.replace(/\.[^/.]+$/, `.${ext}`)
}

// 计算等比缩放显示框尺寸
const fitBox = (
  w: number,
  h: number,
  maxW: number,
  maxH: number,
): { w: number; h: number } => {
  if (!w || !h) return { w: maxW, h: maxH }
  const ratio = Math.min(maxW / w, maxH / h, 1)
  return { w: Math.round(w * ratio), h: Math.round(h * ratio) }
}

// ===== 缩略图组件 (惰性加载) =====
const Thumbnail: React.FC<{ path: string }> = ({ path }) => {
  const [url, setUrl] = useState<string | null>(null)
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    let active = true
    setUrl(null)
    setFailed(false)
    invoke<ImageDataUrl>('read_image_data_url', { path, maxDimension: 128 })
      .then((r) => {
        if (active) setUrl(r.data_url)
      })
      .catch(() => {
        if (active) setFailed(true)
      })
    return () => {
      active = false
    }
  }, [path])

  if (failed) {
    return (
      <div className='w-12 h-12 rounded bg-slate-100 dark:bg-slate-700 flex items-center justify-center text-slate-400 text-xs'>
        N/A
      </div>
    )
  }
  if (!url) {
    return (
      <div className='w-12 h-12 rounded bg-slate-100 dark:bg-slate-700 animate-pulse' />
    )
  }
  return (
    <img
      src={url}
      alt='缩略图'
      className='w-12 h-12 rounded object-cover border border-slate-200 dark:border-slate-600'
    />
  )
}

// ===== 滑块对比组件 =====
interface CompareSliderProps {
  originalUrl: string
  compressedUrl: string
  originalDims: { w: number; h: number }
}

const CompareSlider: React.FC<CompareSliderProps> = ({
  originalUrl,
  compressedUrl,
  originalDims,
}) => {
  const [pos, setPos] = useState(50)
  const containerRef = useRef<HTMLDivElement>(null)
  const draggingRef = useRef(false)

  const box = useMemo(
    () => fitBox(originalDims.w, originalDims.h, 880, 560),
    [originalDims.w, originalDims.h],
  )

  const updateFromClientX = (clientX: number) => {
    const el = containerRef.current
    if (!el) return
    const rect = el.getBoundingClientRect()
    const p = ((clientX - rect.left) / rect.width) * 100
    setPos(Math.max(0, Math.min(100, p)))
  }

  return (
    <div
      ref={containerRef}
      className='relative mx-auto select-none touch-none cursor-ew-resize'
      style={{ width: box.w, height: box.h }}
      onPointerDown={(e) => {
        draggingRef.current = true
        e.currentTarget.setPointerCapture(e.pointerId)
        updateFromClientX(e.clientX)
      }}
      onPointerMove={(e) => {
        if (draggingRef.current) updateFromClientX(e.clientX)
      }}
      onPointerUp={(e) => {
        draggingRef.current = false
        try {
          e.currentTarget.releasePointerCapture(e.pointerId)
        } catch {
          /* ignore */
        }
      }}>
      {/* 底层: 压缩后 */}
      <img
        src={compressedUrl}
        alt='压缩后'
        draggable={false}
        className='absolute inset-0 w-full h-full object-contain'
      />
      {/* 上层: 原图, 通过 clip-path 显示左侧 pos% */}
      <img
        src={originalUrl}
        alt='原图'
        draggable={false}
        className='absolute inset-0 w-full h-full object-contain'
        style={{ clipPath: `inset(0 ${100 - pos}% 0 0)` }}
      />
      {/* 分隔线 + 手柄 */}
      <div
        className='absolute top-0 bottom-0 w-0.5 bg-white shadow-[0_0_0_1px_rgba(0,0,0,0.3)]'
        style={{ left: `${pos}%` }}
      />
      <div
        className='absolute top-1/2 -translate-y-1/2 -translate-x-1/2 w-8 h-8 rounded-full bg-white shadow-lg flex items-center justify-center text-slate-700 text-xs font-bold'
        style={{ left: `${pos}%` }}>
        ⇄
      </div>
      {/* 标签 */}
      <div className='absolute top-2 left-2 px-2 py-0.5 rounded bg-black/60 text-white text-xs'>
        原图
      </div>
      <div className='absolute top-2 right-2 px-2 py-0.5 rounded bg-primary-600/80 text-white text-xs'>
        压缩后
      </div>
    </div>
  )
}

// ===== 主组件 =====
const ImageCompressor: React.FC = () => {
  const [imageFiles, setImageFiles] = useState<ImageFile[]>([])
  const [isConverting, setIsConverting] = useState(false)
  const [currentIndex, setCurrentIndex] = useState<number>(-1)
  const [error, setError] = useState('')
  const [isSuccess, setIsSuccess] = useState(false)

  // 压缩设置
  const [mode, setMode] = useState<'lossless' | 'quality'>('quality')
  const [quality, setQuality] = useState(80)
  const [pngLevel, setPngLevel] = useState(3)
  const [maxWidth, setMaxWidth] = useState<string>('')
  const [maxHeight, setMaxHeight] = useState<string>('')
  const [outputFormat, setOutputFormat] = useState('keep')
  const [removeExif, setRemoveExif] = useState(true)
  const [useCustomOutputDir, setUseCustomOutputDir] = useState(false)
  const [customOutputDir, setCustomOutputDir] = useState('')

  // 对比预览
  const [compare, setCompare] = useState<{
    file: ImageFile
  } | null>(null)
  const [compareLoading, setCompareLoading] = useState(false)
  const [compareData, setCompareData] = useState<{
    original: ImageDataUrl
    compressed: ImageDataUrl
  } | null>(null)

  const { copyToClipboard } = useCopyToClipboard()

  // ===== 文件选择 =====
  const buildImageFile = async (filePath: string): Promise<ImageFile> => {
    const name = filePath.split(/[\\/]/).pop() || filePath
    const ext = name.split('.').pop()?.toLowerCase() || ''
    let fileSize = 0
    try {
      fileSize = await size(filePath)
    } catch {
      /* ignore */
    }
    let dimensions = '未知'
    try {
      const info = await invoke<SourceImageInfo>('get_image_info_command', {
        inputPath: filePath,
      })
      dimensions = info.dimensions
    } catch {
      /* ignore */
    }
    return {
      name,
      path: filePath,
      size: fileSize,
      format: ext,
      dimensions,
      status: 'pending',
    }
  }

  const addPaths = async (paths: string[]) => {
    const valid = paths.filter((p) => {
      const ext = p.split('.').pop()?.toLowerCase() || ''
      return SUPPORTED_EXTENSIONS.includes(ext)
    })
    if (valid.length === 0) {
      setError('未找到支持的图片文件 (png/jpg/jpeg/webp/gif/bmp/tiff)')
      return
    }
    const built: ImageFile[] = []
    for (const p of valid) {
      try {
        built.push(await buildImageFile(p))
      } catch {
        /* skip */
      }
    }
    setImageFiles((prev) => {
      const existing = new Set(prev.map((f) => f.path))
      const fresh = built.filter((f) => !existing.has(f.path))
      if (fresh.length > 0) setError('')
      return [...prev, ...fresh]
    })
  }

  const handleSelectFiles = () => {
    open({
      multiple: true,
      filters: [{ name: '图片文件', extensions: SUPPORTED_EXTENSIONS }],
    })
      .then((selected) => {
        if (!selected) return
        const arr = Array.isArray(selected) ? selected : [selected]
        return addPaths(arr as string[])
      })
      .catch((e) => setError(`选择文件失败: ${String(e)}`))
  }

  const getFilesFromDirectory = async (dirPath: string): Promise<string[]> => {
    const result: string[] = []
    const entries = await readDir(dirPath)
    for (const entry of entries) {
      const full = await join(dirPath, entry.name)
      if (entry.isDirectory) {
        result.push(...(await getFilesFromDirectory(full)))
      } else if (entry.isFile) {
        const ext = entry.name.split('.').pop()?.toLowerCase() || ''
        if (SUPPORTED_EXTENSIONS.includes(ext)) result.push(full)
      }
    }
    return result
  }

  const handleSelectDirectory = () => {
    open({ directory: true, multiple: false })
      .then(async (selected) => {
        if (!selected || Array.isArray(selected)) return
        const files = await getFilesFromDirectory(selected as string)
        if (files.length === 0) {
          setError('所选目录中没有支持的图片文件')
          return
        }
        return addPaths(files)
      })
      .catch((e) => setError(`选择目录失败: ${String(e)}`))
  }

  const handleSelectOutputDirectory = () => {
    open({ directory: true, multiple: false })
      .then((selected) => {
        if (selected && !Array.isArray(selected)) setCustomOutputDir(selected)
      })
      .catch((e) => setError(`选择输出目录失败: ${String(e)}`))
  }

  // 计算单文件输出路径
  const computeOutputPath = async (filePath: string): Promise<string> => {
    const dir = await dirname(filePath)
    const base = await basename(filePath)
    const outName = computeOutputName(base, outputFormat)
    if (useCustomOutputDir && customOutputDir) {
      return join(customOutputDir, outName)
    }
    return join(dir, 'compressed', outName)
  }

  // ===== 批量压缩 =====
  const startBatchCompression = async () => {
    if (imageFiles.length === 0) {
      setError('请先添加图片文件')
      return
    }
    if (useCustomOutputDir && !customOutputDir) {
      setError('已启用自定义输出目录, 但未选择目录')
      return
    }

    setIsConverting(true)
    setError('')
    setIsSuccess(false)
    const working = [...imageFiles]
    const totalCount = working.length
    let idx = 0
    let successCount = 0
    let errorCount = 0

    const processNext = async () => {
      if (idx >= totalCount) {
        setIsConverting(false)
        setCurrentIndex(-1)
        if (errorCount === 0) {
          setError(`✅ 批量压缩完成: 成功 ${successCount} 个文件`)
          setIsSuccess(true)
        } else {
          setError(
            `批量压缩完成: 成功 ${successCount} 个, 失败 ${errorCount} 个`,
          )
          setIsSuccess(false)
        }
        return
      }

      const currentIdx = idx
      setCurrentIndex(currentIdx)
      const file = working[currentIdx]

      setImageFiles((prev) =>
        prev.map((f, i) =>
          i === currentIdx ? { ...f, status: 'compressing' } : f,
        ),
      )

      try {
        const outputPath = await computeOutputPath(file.path)
        const mw = maxWidth.trim() ? Number(maxWidth) : undefined
        const mh = maxHeight.trim() ? Number(maxHeight) : undefined
        const request: CompressRequest = {
          input_path: file.path,
          output_path: outputPath,
          mode,
          quality: mode === 'quality' ? quality : undefined,
          max_width: mw && mw > 0 ? mw : undefined,
          max_height: mh && mh > 0 ? mh : undefined,
          output_format: outputFormat,
          remove_exif: removeExif,
          png_optimize_level: pngLevel,
        }
        const resp = await invoke<CompressResponse>('compress_image', { request })
        setImageFiles((prev) =>
          prev.map((f, i) =>
            i === currentIdx
              ? {
                  ...f,
                  status: 'completed',
                  outputPath: resp.output_path,
                  compressedSize: resp.compressed_size,
                  ratio: resp.compression_ratio,
                  outDimensions: `${resp.output_width}x${resp.output_height}`,
                }
              : f,
          ),
        )
        successCount += 1
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e)
        setImageFiles((prev) =>
          prev.map((f, i) =>
            i === currentIdx
              ? { ...f, status: 'error', error: msg }
              : f,
          ),
        )
        errorCount += 1
      } finally {
        idx += 1
        // 让 UI 有机会刷新
        setTimeout(processNext, 30)
      }
    }
    processNext()
  }

  const removeFile = (index: number) => {
    setImageFiles((prev) => prev.filter((_, i) => i !== index))
    if (currentIndex === index) {
      setCurrentIndex(-1)
      setIsConverting(false)
    } else if (currentIndex > index) {
      setCurrentIndex((c) => c - 1)
    }
  }

  const clearFileList = () => {
    setImageFiles([])
    setIsConverting(false)
    setCurrentIndex(-1)
    setError('')
    setIsSuccess(false)
  }

  // ===== 对比预览 =====
  const openCompare = async (file: ImageFile) => {
    if (!file.outputPath) return
    setCompare({ file })
    setCompareData(null)
    setCompareLoading(true)
    try {
      const [original, compressed] = await Promise.all([
        invoke<ImageDataUrl>('read_image_data_url', {
          path: file.path,
          maxDimension: 1400,
        }),
        invoke<ImageDataUrl>('read_image_data_url', {
          path: file.outputPath,
          maxDimension: 1400,
        }),
      ])
      setCompareData({ original, compressed })
    } catch (e) {
      setError(`加载对比预览失败: ${String(e)}`)
      setCompare(null)
    } finally {
      setCompareLoading(false)
    }
  }

  const closeCompare = () => {
    setCompare(null)
    setCompareData(null)
  }

  // ===== 统计 =====
  const stats = useMemo(() => {
    const done = imageFiles.filter((f) => f.status === 'completed')
    const totalOriginal = done.reduce((s, f) => s + f.size, 0)
    const totalCompressed = done.reduce((s, f) => s + (f.compressedSize || 0), 0)
    const saved = totalOriginal - totalCompressed
    const savedPct =
      totalOriginal > 0 ? (1 - totalCompressed / totalOriginal) * 100 : 0
    return { totalOriginal, totalCompressed, saved, savedPct, count: done.length }
  }, [imageFiles])

  // 质量滑块是否适用 (PNG/JPEG/WebP 输出, 或保持原格式的对应类型)
  const qualityApplies =
    mode === 'quality' &&
    (outputFormat === 'png' ||
      outputFormat === 'jpeg' ||
      outputFormat === 'webp' ||
      (outputFormat === 'keep' &&
        imageFiles.some((f) =>
          ['png', 'jpg', 'jpeg', 'webp'].includes(f.format),
        )))

  const pngLevelApplies =
    outputFormat === 'png' ||
    (outputFormat === 'keep' && imageFiles.some((f) => f.format === 'png'))

  return (
    <ToolLayout
      title='图片压缩工具'
      description='PNG/JPG 无损优化与质量压缩,支持等比缩放、批量处理、压缩前后滑块对比预览'>
      <div className='flex flex-col h-full'>
        {/* ===== 压缩设置 ===== */}
        <div className='mb-4 p-4 border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-800'>
          <h3 className='text-lg font-medium text-slate-900 dark:text-white mb-4'>
            压缩设置
          </h3>

          <div className='grid grid-cols-1 md:grid-cols-2 gap-4 mb-4'>
            {/* 压缩模式 */}
            <div>
              <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                压缩模式
              </label>
              <div className='flex space-x-4'>
                <label className='flex items-center cursor-pointer'>
                  <input
                    type='radio'
                    name='mode'
                    checked={mode === 'lossless'}
                    onChange={() => setMode('lossless')}
                    disabled={isConverting}
                    className='w-4 h-4 text-primary-600 focus:ring-primary-500 border-slate-300'
                  />
                  <span className='ml-2 text-sm text-slate-700 dark:text-slate-300'>
                    无损优化
                  </span>
                </label>
                <label className='flex items-center cursor-pointer'>
                  <input
                    type='radio'
                    name='mode'
                    checked={mode === 'quality'}
                    onChange={() => setMode('quality')}
                    disabled={isConverting}
                    className='w-4 h-4 text-primary-600 focus:ring-primary-500 border-slate-300'
                  />
                  <span className='ml-2 text-sm text-slate-700 dark:text-slate-300'>
                    质量压缩
                  </span>
                </label>
              </div>
              <p className='mt-1 text-xs text-slate-500 dark:text-slate-400'>
                {mode === 'lossless'
                  ? 'PNG 由 oxipng 真无损优化(对已优化的 PNG 效果有限);JPEG 以最高质量重编码(近似无损)。如需大幅减小 PNG 体积请用"质量压缩"'
                  : 'PNG 走色彩量化(视觉无损,类 TinyPNG,大幅减小);JPEG/WebP 通过降低质量换取更小体积'}
              </p>
            </div>

            {/* 输出格式 */}
            <div>
              <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                输出格式
              </label>
              <select
                value={outputFormat}
                onChange={(e) => setOutputFormat(e.target.value)}
                disabled={isConverting}
                className='w-full px-3 py-2 border border-slate-300 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'>
                {OUTPUT_FORMATS.map((f) => (
                  <option key={f.value} value={f.value}>
                    {f.label}
                  </option>
                ))}
              </select>
              <p className='mt-1 text-xs text-slate-500 dark:text-slate-400'>
                WebP 仅支持无损编码
              </p>
            </div>
          </div>

          {/* 质量滑块 */}
          {mode === 'quality' && (
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
                disabled={isConverting}
                className='w-full h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer dark:bg-slate-700'
              />
              <p className='mt-1 text-xs text-slate-500 dark:text-slate-400'>
                {qualityApplies
                  ? '质量越低文件越小(PNG 为色彩数,JPEG/WebP 为编码质量),建议 70-90'
                  : '当前输出格式不支持质量调整(仅 PNG/JPEG/WebP 生效)'}
              </p>
            </div>
          )}

          {/* PNG 优化级别 */}
          {pngLevelApplies && (
            <div className='mb-4'>
              <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                PNG 无损优化级别
              </label>
              <select
                value={pngLevel}
                onChange={(e) => setPngLevel(parseInt(e.target.value))}
                disabled={isConverting}
                className='px-3 py-2 border border-slate-300 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'>
                <option value={1}>1 - 最快</option>
                <option value={2}>2 - 快速</option>
                <option value={3}>3 - 标准 (推荐)</option>
                <option value={4}>4 - 较强</option>
                <option value={5}>5 - 强力</option>
                <option value={6}>6 - 最大压缩</option>
              </select>
              <p className='mt-1 text-xs text-slate-500 dark:text-slate-400'>
                级别越高压缩率越好,但耗时增加
              </p>
            </div>
          )}

          {/* 等比缩放 */}
          <div className='mb-4'>
            <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
              等比缩放 (可选, 留空则保持原尺寸)
            </label>
            <div className='flex items-center space-x-4'>
              <div className='flex items-center space-x-2'>
                <span className='text-sm text-slate-600 dark:text-slate-400'>
                  最大宽度
                </span>
                <input
                  type='number'
                  min='1'
                  placeholder='如 1920'
                  value={maxWidth}
                  onChange={(e) => setMaxWidth(e.target.value)}
                  disabled={isConverting}
                  className='w-28 px-2 py-1 border border-slate-300 rounded text-sm dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                />
                <span className='text-sm text-slate-500 dark:text-slate-400'>px</span>
              </div>
              <div className='flex items-center space-x-2'>
                <span className='text-sm text-slate-600 dark:text-slate-400'>
                  最大高度
                </span>
                <input
                  type='number'
                  min='1'
                  placeholder='如 1080'
                  value={maxHeight}
                  onChange={(e) => setMaxHeight(e.target.value)}
                  disabled={isConverting}
                  className='w-28 px-2 py-1 border border-slate-300 rounded text-sm dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                />
                <span className='text-sm text-slate-500 dark:text-slate-400'>px</span>
              </div>
            </div>
            <p className='mt-1 text-xs text-slate-500 dark:text-slate-400'>
              同时填写时按约束框等比缩放;只填一项则仅约束该方向
            </p>
          </div>

          {/* 选项: 移除 EXIF */}
          <div className='mb-4'>
            <label className='flex items-center cursor-pointer'>
              <input
                type='checkbox'
                checked={removeExif}
                onChange={(e) => setRemoveExif(e.target.checked)}
                disabled={isConverting}
                className='w-4 h-4 text-primary-600 focus:ring-primary-500 border-slate-300 rounded'
              />
              <span className='ml-2 text-sm font-medium text-slate-700 dark:text-slate-300'>
                移除 EXIF 等元数据 (保护隐私且进一步减小体积)
              </span>
            </label>
          </div>

          {/* 输出目录 */}
          <div>
            <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
              输出位置
            </label>
            <div className='flex items-center space-x-4 mb-2'>
              <label className='flex items-center cursor-pointer'>
                <input
                  type='radio'
                  name='outputDir'
                  checked={!useCustomOutputDir}
                  onChange={() => setUseCustomOutputDir(false)}
                  disabled={isConverting}
                  className='w-4 h-4 text-primary-600 focus:ring-primary-500 border-slate-300'
                />
                <span className='ml-2 text-sm text-slate-700 dark:text-slate-300'>
                  自动创建 compressed 子目录
                </span>
              </label>
              <label className='flex items-center cursor-pointer'>
                <input
                  type='radio'
                  name='outputDir'
                  checked={useCustomOutputDir}
                  onChange={() => setUseCustomOutputDir(true)}
                  disabled={isConverting}
                  className='w-4 h-4 text-primary-600 focus:ring-primary-500 border-slate-300'
                />
                <span className='ml-2 text-sm text-slate-700 dark:text-slate-300'>
                  自定义输出目录
                </span>
              </label>
            </div>
            {useCustomOutputDir && (
              <div className='flex items-center space-x-3'>
                <input
                  type='text'
                  value={customOutputDir}
                  placeholder='选择输出目录...'
                  readOnly
                  className='flex-1 px-3 py-2 border border-slate-300 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-primary-500 dark:bg-slate-700 dark:border-slate-600 dark:text-white'
                />
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
            {!useCustomOutputDir && (
              <p className='text-xs text-slate-500 dark:text-slate-400'>
                压缩结果将写入每个源文件同级的 <code>compressed/</code> 子目录
              </p>
            )}
          </div>
        </div>

        {/* ===== 文件列表 ===== */}
        <div className='mb-4 flex flex-col flex-1 min-h-[320px]'>
          <div className='flex justify-between items-center mb-3 flex-shrink-0'>
            <h3 className='text-lg font-medium text-slate-900 dark:text-white'>
              待压缩文件 ({imageFiles.length})
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
            <div className='border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-800 flex-1 min-h-0 overflow-hidden flex flex-col'>
              <div className='overflow-auto flex-1'>
                <table className='min-w-full divide-y divide-slate-200 dark:divide-slate-700'>
                  <thead className='bg-slate-100 dark:bg-slate-700 sticky top-0 z-10'>
                    <tr>
                      <th className='px-3 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-300 uppercase'>
                        预览
                      </th>
                      <th className='px-3 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-300 uppercase'>
                        文件名
                      </th>
                      <th className='px-3 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-300 uppercase'>
                        原始大小
                      </th>
                      <th className='px-3 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-300 uppercase'>
                        压缩后
                      </th>
                      <th className='px-3 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-300 uppercase'>
                        尺寸
                      </th>
                      <th className='px-3 py-2 text-left text-xs font-medium text-slate-500 dark:text-slate-300 uppercase'>
                        状态
                      </th>
                      <th className='px-3 py-2 text-center text-xs font-medium text-slate-500 dark:text-slate-300 uppercase'>
                        操作
                      </th>
                    </tr>
                  </thead>
                  <tbody className='divide-y divide-slate-200 dark:divide-slate-700'>
                    {imageFiles.map((file, index) => (
                      <tr
                        key={file.path}
                        className={
                          currentIndex === index
                            ? 'bg-primary-50 dark:bg-primary-900/20'
                            : ''
                        }>
                        <td className='px-3 py-2'>
                          <Thumbnail path={file.path} />
                        </td>
                        <td className='px-3 py-2 text-sm'>
                          <div
                            className='text-slate-900 dark:text-slate-100 truncate max-w-[220px] cursor-pointer hover:text-primary-600 dark:hover:text-primary-400'
                            title={file.path}
                            onClick={() => copyToClipboard(file.path)}>
                            {file.name}
                          </div>
                          <div className='text-xs text-slate-500 dark:text-slate-400 uppercase'>
                            {file.format}
                          </div>
                          {file.status === 'completed' && file.outputPath && (
                            <div
                              className='text-xs text-primary-600 dark:text-primary-400 truncate max-w-[220px] cursor-pointer hover:underline mt-0.5'
                              title={file.outputPath}
                              onClick={() => copyToClipboard(file.outputPath!)}>
                              → {file.outputPath.split(/[\\/]/).pop()}
                            </div>
                          )}
                        </td>
                        <td className='px-3 py-2 text-sm text-slate-900 dark:text-slate-100 whitespace-nowrap'>
                          {formatFileSize(file.size)}
                        </td>
                        <td className='px-3 py-2 text-sm whitespace-nowrap'>
                          {file.status === 'completed' && file.compressedSize !== undefined ? (
                            <div>
                              <div className='text-slate-900 dark:text-slate-100'>
                                {formatFileSize(file.compressedSize)}
                              </div>
                              <div
                                className={`text-xs font-medium ${
                                  file.ratio !== undefined && file.ratio < 1
                                    ? 'text-green-600 dark:text-green-400'
                                    : 'text-orange-600 dark:text-orange-400'
                                }`}>
                                {file.ratio !== undefined
                                  ? `${Math.round((1 - file.ratio) * 100)}%`
                                  : ''}
                              </div>
                            </div>
                          ) : (
                            <span className='text-slate-400 dark:text-slate-500'>-</span>
                          )}
                        </td>
                        <td className='px-3 py-2 text-sm text-slate-700 dark:text-slate-300 whitespace-nowrap'>
                          {file.outDimensions
                            ? `${file.dimensions} → ${file.outDimensions}`
                            : file.dimensions}
                        </td>
                        <td className='px-3 py-2 text-sm whitespace-nowrap'>
                          {file.status === 'pending' && (
                            <span className='text-yellow-600 dark:text-yellow-400'>
                              等待中
                            </span>
                          )}
                          {file.status === 'compressing' && (
                            <span className='text-primary-600 dark:text-primary-400'>
                              压缩中...
                            </span>
                          )}
                          {file.status === 'completed' && (
                            <span className='text-green-600 dark:text-green-400'>
                              已完成
                            </span>
                          )}
                          {file.status === 'error' && (
                            <span
                              className='text-red-600 dark:text-red-400'
                              title={file.error}>
                              失败
                            </span>
                          )}
                        </td>
                        <td className='px-3 py-2 text-sm'>
                          <div className='flex items-center justify-center space-x-1'>
                            {file.status === 'completed' && (
                              <button
                                onClick={() => openCompare(file)}
                                className='px-2 py-1 text-xs rounded bg-primary-100 text-primary-700 hover:bg-primary-200 dark:bg-primary-900/40 dark:text-primary-300 dark:hover:bg-primary-900/60'>
                                对比
                              </button>
                            )}
                            <button
                              onClick={() => removeFile(index)}
                              disabled={isConverting && file.status === 'compressing'}
                              className={`p-1 rounded-lg transition-colors ${
                                isConverting && file.status === 'compressing'
                                  ? 'text-slate-400 cursor-not-allowed'
                                  : 'text-red-600 hover:text-red-700 hover:bg-red-50 dark:hover:bg-red-900/20'
                              }`}
                              title='移除'>
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
                暂无文件,点击右上角"添加文件"或"添加目录"
              </p>
            </div>
          )}
        </div>

        {/* ===== 操作栏 + 汇总 ===== */}
        {imageFiles.length > 0 && (
          <div className='mb-4 flex-shrink-0'>
            <div className='border border-slate-200 dark:border-slate-700 rounded-lg p-4 bg-slate-50 dark:bg-slate-800'>
              {isConverting && currentIndex >= 0 && (
                <div className='mb-4'>
                  <div className='flex justify-between items-center mb-2'>
                    <p className='text-sm text-slate-600 dark:text-slate-400'>
                      总体进度: {currentIndex + 1} / {imageFiles.length}
                    </p>
                    <p className='text-sm text-slate-600 dark:text-slate-400'>
                      {Math.round(((currentIndex + 1) / imageFiles.length) * 100)}%
                    </p>
                  </div>
                  <div className='w-full bg-slate-200 dark:bg-slate-700 rounded-full h-2 overflow-hidden'>
                    <div
                      className='bg-primary-600 h-2 rounded-full transition-all duration-200'
                      style={{
                        width: `${((currentIndex + 1) / imageFiles.length) * 100}%`,
                      }}
                    />
                  </div>
                </div>
              )}

              {stats.count > 0 && !isConverting && (
                <div className='mb-4 grid grid-cols-2 md:grid-cols-4 gap-3 text-center'>
                  <div className='p-2 rounded-lg bg-white dark:bg-slate-700'>
                    <div className='text-xs text-slate-500 dark:text-slate-400'>
                      已压缩
                    </div>
                    <div className='text-lg font-semibold text-slate-900 dark:text-white'>
                      {stats.count}
                    </div>
                  </div>
                  <div className='p-2 rounded-lg bg-white dark:bg-slate-700'>
                    <div className='text-xs text-slate-500 dark:text-slate-400'>
                      原始合计
                    </div>
                    <div className='text-lg font-semibold text-slate-900 dark:text-white'>
                      {formatFileSize(stats.totalOriginal)}
                    </div>
                  </div>
                  <div className='p-2 rounded-lg bg-white dark:bg-slate-700'>
                    <div className='text-xs text-slate-500 dark:text-slate-400'>
                      压缩后
                    </div>
                    <div className='text-lg font-semibold text-slate-900 dark:text-white'>
                      {formatFileSize(stats.totalCompressed)}
                    </div>
                  </div>
                  <div className='p-2 rounded-lg bg-green-50 dark:bg-green-900/20'>
                    <div className='text-xs text-green-600 dark:text-green-400'>
                      节省空间
                    </div>
                    <div className='text-lg font-semibold text-green-700 dark:text-green-400'>
                      {formatFileSize(stats.saved)}{' '}
                      <span className='text-sm'>
                        ({stats.savedPct.toFixed(1)}%)
                      </span>
                    </div>
                  </div>
                </div>
              )}

              <div className='flex space-x-3'>
                <Button
                  onClick={startBatchCompression}
                  variant='primary'
                  disabled={isConverting || imageFiles.length === 0}
                  className='flex-1'>
                  {isConverting ? '压缩中...' : '开始压缩'}
                </Button>
                <Button
                  onClick={clearFileList}
                  variant='secondary'
                  disabled={isConverting}
                  className='flex-1'>
                  清空列表
                </Button>
              </div>
            </div>
          </div>
        )}

        {/* 状态/错误信息 */}
        {error && (
          <div
            className={`mt-4 p-4 rounded-lg flex-shrink-0 ${
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

      {/* ===== 对比预览弹窗 ===== */}
      {compare && (
        <div
          className='fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4'
          onClick={closeCompare}>
          <div
            className='bg-white dark:bg-slate-800 rounded-lg shadow-2xl max-w-[940px] w-full max-h-[92vh] overflow-auto'
            onClick={(e) => e.stopPropagation()}>
            <div className='flex items-center justify-between p-4 border-b border-slate-200 dark:border-slate-700 sticky top-0 bg-white dark:bg-slate-800 z-10'>
              <div>
                <h3 className='text-lg font-medium text-slate-900 dark:text-white'>
                  压缩前后对比
                </h3>
                <p className='text-xs text-slate-500 dark:text-slate-400 truncate max-w-[600px]'>
                  {compare.file.name}
                </p>
              </div>
              <button
                onClick={closeCompare}
                className='p-2 rounded-lg text-slate-500 hover:text-slate-700 hover:bg-slate-100 dark:hover:bg-slate-700'>
                <svg
                  className='w-5 h-5'
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

            <div className='p-4'>
              {compareLoading || !compareData ? (
                <div className='flex items-center justify-center h-[400px]'>
                  <div className='text-slate-500 dark:text-slate-400'>
                    加载中...
                  </div>
                </div>
              ) : (
                <>
                  <div className='flex flex-wrap justify-center gap-4 mb-4 text-sm'>
                    <div className='px-3 py-1 rounded bg-slate-100 dark:bg-slate-700'>
                      <span className='text-slate-500 dark:text-slate-400'>原图:</span>{' '}
                      <span className='font-medium text-slate-900 dark:text-white'>
                        {formatFileSize(compare.file.size)}
                      </span>
                      <span className='text-slate-500 dark:text-slate-400 ml-1'>
                        ({compare.file.dimensions})
                      </span>
                    </div>
                    <div className='px-3 py-1 rounded bg-primary-50 dark:bg-primary-900/30'>
                      <span className='text-primary-600 dark:text-primary-400'>
                        压缩后:
                      </span>{' '}
                      <span className='font-medium text-slate-900 dark:text-white'>
                        {formatFileSize(compare.file.compressedSize || 0)}
                      </span>
                      <span className='text-green-600 dark:text-green-400 ml-1'>
                        (-{Math.round((1 - (compare.file.ratio || 1)) * 100)}%)
                      </span>
                    </div>
                  </div>
                  <p className='text-center text-xs text-slate-500 dark:text-slate-400 mb-3'>
                    拖动中间的 ⇄ 手柄左右滑动对比画质
                  </p>
                  <CompareSlider
                    originalUrl={compareData.original.data_url}
                    compressedUrl={compareData.compressed.data_url}
                    originalDims={{
                      w: compareData.original.width,
                      h: compareData.original.height,
                    }}
                  />
                </>
              )}
            </div>
          </div>
        </div>
      )}
    </ToolLayout>
  )
}

export default ImageCompressor
