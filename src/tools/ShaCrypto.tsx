import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { readTextFile } from '@tauri-apps/plugin-fs'
import CryptoJS from 'crypto-js'
import React, { useEffect, useState } from 'react'
import { Button, InputField } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'

type TabType = 'text' | 'file' | 'batch'
type HashAlgorithm = 'sha1' | 'sha256' | 'sha512' | 'sha3' | 'sha224' | 'sha384'

/**
 * SHA 哈希加密工具
 * 支持对文本和文件内容进行多种 SHA 算法加密
 */
const ShaCrypto: React.FC = () => {
  const [activeTab, setActiveTab] = useState<TabType>('text')
  const [algorithm, setAlgorithm] = useState<HashAlgorithm>('sha256')

  // 文本加密相关状态
  const [textInput, setTextInput] = useState('')
  const [textShaHash, setTextShaHash] = useState('')
  const [textError, setTextError] = useState('')

  // 文件加密相关状态
  const [filePath, setFilePath] = useState('')
  const [fileShaHash, setFileShaHash] = useState('')
  const [fileError, setFileError] = useState('')
  const [isProcessingFile, setIsProcessingFile] = useState(false)

  // 批量文件处理状态
  const [batchFiles, setBatchFiles] = useState<
    Array<{
      path: string
      name: string
      sha: string
      status: 'pending' | 'processing' | 'completed' | 'error'
    }>
  >([])
  const [isProcessingBatch, setIsProcessingBatch] = useState(false)

  const { copy, copied } = useCopyToClipboard()

  // 获取算法名称
  const getAlgorithmName = (algo: HashAlgorithm): string => {
    const algoMap: Record<HashAlgorithm, string> = {
      sha1: 'SHA-1',
      sha256: 'SHA-256',
      sha512: 'SHA-512',
      sha3: 'SHA-3',
      sha224: 'SHA-224',
      sha384: 'SHA-384',
    }
    return algoMap[algo]
  }

  // 根据算法计算哈希值
  const calculateHash = (text: string, algo: HashAlgorithm): string => {
    let hash: string
    switch (algo) {
      case 'sha1':
        hash = CryptoJS.SHA1(text).toString()
        break
      case 'sha256':
        hash = CryptoJS.SHA256(text).toString()
        break
      case 'sha512':
        hash = CryptoJS.SHA512(text).toString()
        break
      case 'sha3':
        hash = CryptoJS.SHA3(text).toString()
        break
      case 'sha224':
        hash = CryptoJS.SHA224(text).toString()
        break
      case 'sha384':
        hash = CryptoJS.SHA384(text).toString()
        break
      default:
        hash = CryptoJS.SHA256(text).toString()
    }
    return hash
  }

  // 自动计算文本的 SHA 值
  useEffect(() => {
    if (!textInput) {
      setTextShaHash('')
      setTextError('')
      return
    }

    try {
      const hash = calculateHash(textInput, algorithm)
      setTextShaHash(hash)
      setTextError('')
    } catch (error) {
      setTextShaHash('')
      setTextError(
        '加密过程中发生错误: ' +
          (error instanceof Error ? error.message : '未知错误'),
      )
    }
  }, [textInput, algorithm])

  // 选择文件
  const selectFile = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
      })

      if (selected && !Array.isArray(selected)) {
        // 确保selected是字符串类型
        const pathStr = String(selected)
        setFilePath(pathStr)
        setFileShaHash('')
        setFileError('')
      }
    } catch (error) {
      setFileError(
        '选择文件时发生错误: ' +
          (error instanceof Error ? error.message : '未知错误'),
      )
    }
  }

  // 计算文件的 SHA 值
  const hashFile = async () => {
    if (!filePath) {
      setFileError('请先选择一个文件')
      return
    }

    try {
      setIsProcessingFile(true)
      setFileError('')

      // 读取文件内容
      const fileContent = await readTextFile(filePath)

      // 计算 SHA 值
      const hash = calculateHash(fileContent, algorithm)
      setFileShaHash(hash)
    } catch (error) {
      setFileShaHash('')
      setFileError(
        '处理文件时发生错误: ' +
          (error instanceof Error ? error.message : '未知错误'),
      )
    } finally {
      setIsProcessingFile(false)
    }
  }

  // 当文件路径或算法变化时，自动计算 SHA
  useEffect(() => {
    if (filePath) {
      hashFile()
    }
  }, [filePath, algorithm])

  // 复制文本 SHA 结果
  const copyTextSha = async () => {
    if (textShaHash) {
      await copy(textShaHash)
    }
  }

  // 复制文件 SHA 结果
  const copyFileSha = async () => {
    if (fileShaHash) {
      await copy(fileShaHash)
    }
  }

  // 清空文本输入
  const clearTextInput = () => {
    setTextInput('')
    setTextShaHash('')
    setTextError('')
  }

  // 清空文件选择
  const clearFileInput = () => {
    setFilePath('')
    setFileShaHash('')
    setFileError('')
  }

  // 批量选择文件
  const selectBatchFiles = async () => {
    try {
      const selected = await openDialog({
        multiple: true,
      })

      if (selected) {
        // 确保selected是数组类型
        const files = Array.isArray(selected) ? selected : [selected]
        const newBatchFiles = files.map((file) => ({
          path: String(file),
          name: String(file).split('/').pop() || String(file),
          sha: '',
          status: 'pending' as const,
        }))
        setBatchFiles(newBatchFiles)

        // 自动开始处理文件
        if (newBatchFiles.length > 0) {
          setIsProcessingBatch(true)

          // 顺序处理每个文件
          for (const file of newBatchFiles) {
            await hashSingleFile(file)
          }

          setIsProcessingBatch(false)
        }
      }
    } catch (error) {
      console.error('选择文件失败:', error)
    }
  }

  // 计算单个文件的SHA
  const hashSingleFile = async (file: {
    path: string
    name: string
    sha: string
    status: 'pending' | 'processing' | 'completed' | 'error'
  }) => {
    try {
      // 更新文件状态为处理中
      setBatchFiles((prev) =>
        prev.map((f) =>
          f.path === file.path ? { ...f, status: 'processing' } : f,
        ),
      )

      const contents = await readTextFile(file.path)

      // 使用CryptoJS计算SHA，支持所有类型的文件
      const hash = calculateHash(contents, algorithm)

      // 更新文件状态为已完成
      setBatchFiles((prev) =>
        prev.map((f) =>
          f.path === file.path ? { ...f, sha: hash, status: 'completed' } : f,
        ),
      )

      return hash
    } catch (error) {
      console.error(`计算文件 ${file.name} SHA 失败:`, error)

      // 更新文件状态为错误
      setBatchFiles((prev) =>
        prev.map((f) => (f.path === file.path ? { ...f, status: 'error' } : f)),
      )

      return null
    }
  }

  // 清空批量文件列表
  const clearBatchFiles = () => {
    setBatchFiles([])
  }

  // 复制所有SHA结果
  const copyAllShaResults = async () => {
    const completedFiles = batchFiles.filter((f) => f.status === 'completed')
    if (completedFiles.length === 0) return

    const results = completedFiles.map((f) => `${f.name}: ${f.sha}`).join('\n')
    await copy(results)
  }

  // 导出SHA结果到文件
  const exportShaResults = async () => {
    const completedFiles = batchFiles.filter((f) => f.status === 'completed')
    if (completedFiles.length === 0) return

    const results = completedFiles.map((f) => `${f.name}: ${f.sha}`).join('\n')

    // 使用Tauri的API保存文件
    try {
      const { save } = await import('@tauri-apps/plugin-dialog')
      const path = await save({
        filters: [
          {
            name: '文本文件',
            extensions: ['txt'],
          },
        ],
      })

      if (path) {
        const { writeTextFile } = await import('@tauri-apps/plugin-fs')
        await writeTextFile(path, results)
      }
    } catch (error) {
      console.error('导出文件失败:', error)
    }
  }

  // 加载示例
  const loadExample = () => {
    setTextInput('Hello, World! 你好，世界！')
  }

  return (
    <ToolLayout
      title='SHA 哈希加密工具'
      subtitle='对文本和文件内容进行多种 SHA 算法加密'>
      <div className='flex flex-col h-full'>
        {/* 算法选择 */}
        <div className='mb-4 p-3 bg-slate-50 dark:bg-slate-700 rounded-lg'>
          <div className='flex items-center space-x-4'>
            <label className='text-sm font-medium text-slate-700 dark:text-slate-300'>
              选择哈希算法:
            </label>
            <select
              value={algorithm}
              onChange={(e) => setAlgorithm(e.target.value as HashAlgorithm)}
              className='px-3 py-1 border border-slate-300 dark:border-slate-600 rounded-lg text-sm bg-white dark:bg-slate-800 text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-2 focus:ring-primary-500'>
              <option value='sha1'>SHA-1</option>
              <option value='sha224'>SHA-224</option>
              <option value='sha256'>SHA-256</option>
              <option value='sha384'>SHA-384</option>
              <option value='sha512'>SHA-512</option>
              <option value='sha3'>SHA-3</option>
            </select>
          </div>
        </div>

        {/* 选项卡 */}
        <div className='flex border-b border-slate-200 dark:border-slate-700 mb-4'>
          <button
            className={`px-4 py-2 font-medium text-sm ${
              activeTab === 'text'
                ? 'border-b-2 border-primary-500 text-primary-600 dark:text-primary-400'
                : 'text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300'
            }`}
            onClick={() => setActiveTab('text')}>
            文本加密
          </button>
          <button
            className={`px-4 py-2 font-medium text-sm ${
              activeTab === 'file'
                ? 'border-b-2 border-primary-500 text-primary-600 dark:text-primary-400'
                : 'text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300'
            }`}
            onClick={() => setActiveTab('file')}>
            文件加密
          </button>
          <button
            className={`px-4 py-2 font-medium text-sm ${
              activeTab === 'batch'
                ? 'border-b-2 border-primary-500 text-primary-600 dark:text-primary-400'
                : 'text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300'
            }`}
            onClick={() => setActiveTab('batch')}>
            批量加密
          </button>
        </div>

        {/* 文本加密选项卡 */}
        {activeTab === 'text' && (
          <div className='flex-1 flex flex-col p-4'>
            <div className='mb-4'>
              <div className='mb-2 flex items-center justify-between'>
                <div className='text-sm text-slate-600 dark:text-slate-400'>
                  输入长度: {textInput.length}
                </div>
                <div className='flex items-center space-x-2'>
                  <Button variant='secondary' size='sm' onClick={loadExample}>
                    示例
                  </Button>
                  <Button
                    variant='secondary'
                    size='sm'
                    onClick={clearTextInput}
                    disabled={!textInput}>
                    清空
                  </Button>
                </div>
              </div>
              <textarea
                className='w-full h-40 resize-none border border-slate-300 dark:border-slate-600 rounded-lg p-3 text-sm font-mono bg-white dark:bg-slate-800 text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-2 focus:ring-primary-500'
                placeholder='请输入要加密的文本...'
                value={textInput}
                onChange={(e) => setTextInput(e.target.value)}
              />
            </div>

            {textError && (
              <div className='mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg'>
                <p className='text-red-700 dark:text-red-400 text-sm'>
                  {textError}
                </p>
              </div>
            )}

            <div className='flex-1 flex flex-col'>
              <div className='mb-2 flex items-center justify-between'>
                <h3 className='text-sm font-medium text-slate-700 dark:text-slate-300'>
                  {getAlgorithmName(algorithm)} 哈希值
                </h3>
                <Button
                  variant={copied ? 'ghost' : 'primary'}
                  size='sm'
                  onClick={copyTextSha}
                  disabled={!textShaHash || !!textError}
                  className={copied ? 'bg-green-100 text-green-700 hover:bg-green-200 dark:bg-green-900 dark:text-green-200 border-green-300 dark:border-green-700' : ''}>
                  {copied ? '已复制 ✓' : '复制结果'}
                </Button>
              </div>
              <div className='flex-1 p-3 bg-slate-100 dark:bg-slate-700 border border-slate-300 dark:border-slate-600 rounded-lg'>
                <p className='font-mono text-sm break-all text-slate-900 dark:text-slate-100'>
                  {textShaHash ||
                    `${getAlgorithmName(algorithm)} 哈希值将在这里显示...`}
                </p>
              </div>
            </div>
          </div>
        )}

        {/* 文件加密选项卡 */}
        {activeTab === 'file' && (
          <div className='flex-1 flex flex-col p-4'>
            <div className='mb-4'>
              <div className='mb-2 flex items-center justify-between'>
                <h3 className='text-sm font-medium text-slate-700 dark:text-slate-300'>
                  选择文件
                </h3>
                <Button
                  variant='secondary'
                  size='sm'
                  onClick={clearFileInput}
                  disabled={!filePath}>
                  清空
                </Button>
              </div>
              <div className='flex items-center space-x-2'>
                <InputField
                  value={filePath}
                  onChange={() => {}} // 使用空函数替代setFilePath，因为输入是只读的
                  placeholder='点击右边按钮选择文件...'
                  disabled // 使用disabled替代readOnly
                  className='flex-1'
                />
                <Button variant={copied ? 'ghost' : 'primary'} onClick={selectFile}>
                  选择文件
                </Button>
              </div>
            </div>

            {fileError && (
              <div className='mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg'>
                <p className='text-red-700 dark:text-red-400 text-sm'>
                  {fileError}
                </p>
              </div>
            )}

            <div className='flex-1 flex flex-col'>
              <div className='mb-2 flex items-center justify-between'>
                <h3 className='text-sm font-medium text-slate-700 dark:text-slate-300'>
                  {getAlgorithmName(algorithm)} 哈希值
                </h3>
                <Button
                  variant={copied ? 'ghost' : 'primary'}
                  size='sm'
                  onClick={copyFileSha}
                  disabled={!fileShaHash || !!fileError || isProcessingFile}
                  className={copied ? 'bg-green-100 text-green-700 hover:bg-green-200 dark:bg-green-900 dark:text-green-200 border-green-300 dark:border-green-700' : ''}>
                  {copied ? '已复制 ✓' : '复制结果'}
                </Button>
              </div>
              <div className='flex-1 p-3 bg-slate-100 dark:bg-slate-700 border border-slate-300 dark:border-slate-600 rounded-lg flex items-center justify-center'>
                {isProcessingFile ? (
                  <p className='text-slate-500 dark:text-slate-400 text-sm'>
                    正在处理文件...
                  </p>
                ) : (
                  <p className='font-mono text-sm break-all text-slate-900 dark:text-slate-100'>
                    {fileShaHash ||
                      `${getAlgorithmName(algorithm)} 哈希值将在这里显示...`}
                  </p>
                )}
              </div>
            </div>
          </div>
        )}

        {/* 批量文件加密选项卡 */}
        {activeTab === 'batch' && (
          <div className='flex-1 flex flex-col p-4'>
            <div className='mb-4'>
              <div className='mb-2 flex items-center justify-between'>
                <h3 className='text-sm font-medium text-slate-700 dark:text-slate-300'>
                  选择多个文件 ({batchFiles.length})
                </h3>
                <div className='flex items-center space-x-2'>
                  <Button
                    variant='secondary'
                    size='sm'
                    onClick={clearBatchFiles}
                    disabled={batchFiles.length === 0}>
                    清空
                  </Button>
                  <Button variant={copied ? 'ghost' : 'primary'} onClick={selectBatchFiles}>
                    选择文件
                  </Button>
                </div>
              </div>
            </div>

            {batchFiles.length > 0 && (
              <div className='mb-4'>
                <div className='mb-2 flex items-center justify-between'>
                  <h3 className='text-sm font-medium text-slate-700 dark:text-slate-300'>
                    文件列表 {isProcessingBatch && '(处理中...)'}
                  </h3>
                  <div className='flex items-center space-x-2'>
                    <Button
                      variant='secondary'
                      size='sm'
                      onClick={copyAllShaResults}
                      disabled={
                        !batchFiles.some((f) => f.status === 'completed')
                      }
                      className={
                        copied ? 'bg-green-600 hover:bg-green-700' : ''
                      }>
                      {copied ? '已复制 ✓' : '复制结果'}
                    </Button>
                    <Button
                      variant='secondary'
                      size='sm'
                      onClick={exportShaResults}
                      disabled={
                        !batchFiles.some((f) => f.status === 'completed')
                      }>
                      导出结果
                    </Button>
                  </div>
                </div>
                <div className='border border-slate-300 dark:border-slate-600 rounded-lg max-h-[50vh] overflow-auto'>
                  <table
                    className='w-full text-sm'
                    style={{ minWidth: '800px' }}>
                    <thead className='bg-slate-50 dark:bg-slate-800 sticky top-0 z-10'>
                      <tr>
                        <th
                          className='px-3 py-2 text-left font-medium text-slate-700 dark:text-slate-300'
                          style={{ width: '200px' }}>
                          文件名
                        </th>
                        <th
                          className='px-3 py-2 text-left font-medium text-slate-700 dark:text-slate-300'
                          style={{ width: '100px' }}>
                          状态
                        </th>
                        <th
                          className='px-3 py-2 text-left font-medium text-slate-700 dark:text-slate-300'
                          style={{ minWidth: '500px' }}>
                          {getAlgorithmName(algorithm)} 哈希值
                        </th>
                      </tr>
                    </thead>
                    <tbody className='divide-y divide-slate-200 dark:divide-slate-700'>
                      {batchFiles.map((file, index) => (
                        <tr
                          key={index}
                          className='hover:bg-slate-50 dark:hover:bg-slate-800'>
                          <td
                            className='px-3 py-2 text-slate-900 dark:text-slate-100'
                            style={{ width: '200px', wordBreak: 'break-all' }}>
                            {file.name}
                          </td>
                          <td className='px-3 py-2' style={{ width: '100px' }}>
                            {file.status === 'pending' && (
                              <span className='inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-slate-100 text-slate-800 dark:bg-slate-700 dark:text-slate-200'>
                                待处理
                              </span>
                            )}
                            {file.status === 'processing' && (
                              <span className='inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-primary-100 text-primary-800 dark:bg-primary-900 dark:text-primary-200'>
                                处理中
                              </span>
                            )}
                            {file.status === 'completed' && (
                              <span className='inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'>
                                已完成
                              </span>
                            )}
                            {file.status === 'error' && (
                              <span className='inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'>
                                错误
                              </span>
                            )}
                          </td>
                          <td
                            className='px-3 py-2 font-mono text-xs text-slate-900 dark:text-slate-100'
                            style={{
                              minWidth: '500px',
                              wordBreak: 'break-all',
                            }}>
                            {file.sha || '-'}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}

            {batchFiles.length === 0 && (
              <div className='flex-1 flex items-center justify-center'>
                <div className='text-center'>
                  <div className='text-slate-400 mb-2'>
                    <svg
                      xmlns='http://www.w3.org/2000/svg'
                      className='h-12 w-12 mx-auto'
                      fill='none'
                      viewBox='0 0 24 24'
                      stroke='currentColor'>
                      <path
                        strokeLinecap='round'
                        strokeLinejoin='round'
                        strokeWidth={2}
                        d='M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z'
                      />
                    </svg>
                  </div>
                  <p className='text-slate-500 dark:text-slate-400 text-sm'>
                    点击"选择文件"按钮选择多个文件进行批量
                    {getAlgorithmName(algorithm)}加密
                  </p>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </ToolLayout>
  )
}

export default ShaCrypto
