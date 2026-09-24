import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import React, { useState } from 'react'
import { Button, ErrorMessage } from '../components/common'
import FileUpload from '../components/common/FileUpload'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'

interface SubjectEntry {
  oid: string
  name: string
  value: string
}

interface PublicKeyDetails {
  keyType: string
  keySize: number | null
  curveName: string | null
  spkiPem: string
  fingerprintSha256: string
}

interface CsrExtensionInfo {
  name: string
  value: string
  critical: boolean
}

interface PrivateKeyCheck {
  format: string
  keyType: string
  keySize: number | null
  curveName: string | null
  fingerprintSha256: string
  matchesCsr: boolean
}

interface CsrInfo {
  version: number
  subject: SubjectEntry[]
  publicKey: PublicKeyDetails
  privateKey: PrivateKeyCheck | null
  signatureAlgorithm: string
  requestedExtensions: CsrExtensionInfo[]
  signatureValid: boolean
  derSize: number
  pem: string
}

const PEM_TEXTAREA_CLASSES =
  'w-full resize-none border border-slate-300 dark:border-slate-600 rounded-lg p-3 text-xs font-mono leading-relaxed bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 focus:outline-none transition-colors duration-200'

/**
 * 独立的复制按钮: 每个按钮拥有自己的复制状态, 避免多处复制互相干扰
 */
const CopyButton: React.FC<{ text: string; label?: string }> = ({ text, label = '复制' }) => {
  const { copy, copied } = useCopyToClipboard()
  return (
    <Button variant={copied ? 'success' : 'secondary'} size='sm' onClick={() => copy(text)}>
      {copied ? '已复制 ✓' : label}
    </Button>
  )
}

const SectionCard: React.FC<{
  title: React.ReactNode
  actions?: React.ReactNode
  children: React.ReactNode
}> = ({ title, actions, children }) => (
  <section className='border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-800 p-4'>
    <div className='flex items-center justify-between gap-3 mb-3'>
      <h3 className='text-sm font-semibold text-slate-800 dark:text-slate-100'>{title}</h3>
      {actions}
    </div>
    {children}
  </section>
)

const InfoRow: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <div className='flex gap-3 py-1.5 border-b border-slate-100 dark:border-slate-700 last:border-b-0'>
    <span className='shrink-0 w-32 text-sm text-slate-500 dark:text-slate-400'>{label}</span>
    <span className='flex-1 text-sm text-slate-900 dark:text-slate-100 font-mono break-all'>
      {children}
    </span>
  </div>
)

const formatDerSize = (size: number) => {
  if (size < 1024) return `${size} 字节`
  return `${(size / 1024).toFixed(2)} KB`
}

const CsrViewer: React.FC = () => {
  const [csrText, setCsrText] = useState('')
  const [keyText, setKeyText] = useState('')
  const [fileData, setFileData] = useState<Uint8Array | null>(null)
  const [fileName, setFileName] = useState('')
  const [fileError, setFileError] = useState<string | null>(null)
  const [result, setResult] = useState<CsrInfo | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState('')
  const [downloadMessage, setDownloadMessage] = useState('')

  const handleTextChange = (value: string) => {
    setCsrText(value)
    if (value.trim()) {
      setFileData(null)
      setFileName('')
    }
  }

  const handleBinaryFileData = (name: string, data: Uint8Array) => {
    setFileName(name)
    setFileData(data)
    setCsrText('')
    setResult(null)
    setError('')
  }

  const handleParse = async () => {
    if (!fileData && !csrText.trim()) {
      setError('请先粘贴 PEM 格式的 CSR 内容或上传 CSR 文件')
      return
    }

    setIsLoading(true)
    setError('')
    setResult(null)

    try {
      let csrContent: string | null = null
      let csrDer: number[] | null = null

      if (fileData) {
        // 上传的文件可能是 PEM 文本, 也可能是二进制 DER
        const text = new TextDecoder('utf-8', { fatal: false }).decode(fileData)
        if (text.includes('-----BEGIN')) {
          csrContent = text
        } else {
          csrDer = Array.from(fileData)
        }
      } else {
        csrContent = csrText
      }

      const info = await invoke<CsrInfo>('parse_csr', {
        csrContent,
        csrDer,
        privateKey: keyText.trim() ? keyText : null,
      })
      setResult(info)
    } catch (err) {
      console.error('解析 CSR 失败:', err)
      setError(typeof err === 'string' ? err : 'CSR 解析失败')
    } finally {
      setIsLoading(false)
    }
  }

  const handleClear = () => {
    setCsrText('')
    setKeyText('')
    setFileData(null)
    setFileName('')
    setFileError(null)
    setResult(null)
    setError('')
    setDownloadMessage('')
  }

  const handleDownloadPem = async () => {
    if (!result) return
    setDownloadMessage('')
    try {
      const filePath = await save({
        filters: [{ name: 'CSR Files', extensions: ['csr'] }],
        defaultPath: 'certificate_request.csr',
      })
      if (!filePath) return
      await writeTextFile(filePath, result.pem)
      setDownloadMessage('文件保存成功')
      setTimeout(() => setDownloadMessage(''), 2000)
    } catch (err) {
      console.error('保存文件失败:', err)
      setDownloadMessage(`保存文件失败: ${err instanceof Error ? err.message : String(err)}`)
    }
  }

  return (
    <ToolLayout
      title='CSR 查看'
      subtitle='解析 PKCS#10 证书签名请求（CSR），查看主题、公钥、签名算法与扩展信息，支持校验配套私钥（PKCS#8 等）是否匹配'>
      <div className='flex flex-col h-full space-y-4 overflow-y-auto pb-4'>
        {/* 输入区 */}
        <section className='border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-800 p-4'>
          <div className='flex flex-wrap items-center justify-between gap-3 mb-3'>
            <h3 className='text-sm font-semibold text-slate-800 dark:text-slate-100'>
              CSR 内容
              {fileName && (
                <span className='ml-2 text-xs font-normal text-slate-500 dark:text-slate-400'>
                  已选择文件: {fileName}
                </span>
              )}
            </h3>
            <div className='flex items-center gap-2'>
              <Button variant='primary' loading={isLoading} onClick={() => void handleParse()}>
                {isLoading ? '解析中…' : '解析 CSR'}
              </Button>
              <Button variant='secondary' size='md' onClick={handleClear}>
                清空
              </Button>
            </div>
          </div>
          <FileUpload
            value={csrText}
            onChange={handleTextChange}
            error={fileError}
            onError={setFileError}
            accept='.csr,.pem,.txt'
            placeholder='粘贴 PEM 格式的 CSR 文本（-----BEGIN CERTIFICATE REQUEST----- ...）'
            fileType='binary'
            onBinaryFileData={handleBinaryFileData}
          />

          <div className='mt-3'>
            <label className='block text-xs font-medium text-slate-500 dark:text-slate-400 mb-1.5'>
              私钥（可选）— 提供 PKCS#8 / PKCS#1 / SEC1 私钥，校验其与 CSR 公钥是否匹配
            </label>
            <textarea
              value={keyText}
              onChange={(e) => setKeyText(e.target.value)}
              disabled={isLoading}
              spellCheck={false}
              placeholder={'-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----'}
              className={`${PEM_TEXTAREA_CLASSES} h-24`}
            />
          </div>
        </section>

        <ErrorMessage message={error || null} />

        {!result && !error && (
          <div className='text-center text-sm text-slate-500 dark:text-slate-400 py-10'>
            上传或粘贴 CSR 文件（PEM / Base64 / DER 格式）后点击「解析 CSR」
          </div>
        )}

        {result && (
          <>
            {/* 基本信息 */}
            <SectionCard
              title='基本信息'
              actions={
                <span
                  className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
                    result.signatureValid
                      ? 'bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300'
                      : 'bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-300'
                  }`}>
                  {result.signatureValid ? '✓ 签名验证通过' : '✗ 签名验证失败'}
                </span>
              }>
              <div>
                <InfoRow label='CSR 版本'>
                  {result.version === 0 ? 'v1（版本号 0）' : result.version}
                </InfoRow>
                <InfoRow label='签名算法'>{result.signatureAlgorithm}</InfoRow>
                <InfoRow label='DER 大小'>{formatDerSize(result.derSize)}</InfoRow>
              </div>
              {!result.signatureValid && (
                <p className='mt-2 text-xs text-red-600 dark:text-red-400'>
                  CSR 的自签名未通过验证：通常表示内容被篡改，或文件与生成它的私钥不匹配
                </p>
              )}
            </SectionCard>

            {/* 主题信息 */}
            <SectionCard title='主题（Subject）'>
              {result.subject.length === 0 ? (
                <p className='text-xs text-slate-500 dark:text-slate-400'>该 CSR 未包含主题属性</p>
              ) : (
                <div>
                  {result.subject.map((entry, index) => (
                    <div
                      key={`${entry.oid}-${index}`}
                      className='flex gap-3 py-1.5 border-b border-slate-100 dark:border-slate-700 last:border-b-0'>
                      <span className='shrink-0 w-40 text-sm text-slate-500 dark:text-slate-400'>
                        {entry.name}
                      </span>
                      <span className='flex-1 min-w-0'>
                        <span className='block text-sm text-slate-900 dark:text-slate-100 font-mono break-all'>
                          {entry.value}
                        </span>
                        <span className='block text-xs text-slate-400 dark:text-slate-500'>
                          OID: {entry.oid}
                        </span>
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </SectionCard>

            {/* 公钥信息 */}
            <SectionCard title='公钥信息'>
              <div>
                <InfoRow label='密钥类型'>
                  {result.publicKey.keyType}
                  {result.publicKey.keySize != null && ` · ${result.publicKey.keySize} 位`}
                  {result.publicKey.curveName && ` · 曲线 ${result.publicKey.curveName}`}
                </InfoRow>
                <InfoRow label='公钥指纹 (SHA-256)'>
                  <span className='flex items-center gap-2'>
                    <span className='break-all'>{result.publicKey.fingerprintSha256}</span>
                    <CopyButton
                      text={result.publicKey.fingerprintSha256}
                      label='复制指纹'
                    />
                  </span>
                </InfoRow>
              </div>
              <div className='mt-3'>
                <div className='flex items-center justify-between gap-2 mb-2'>
                  <span className='text-xs text-slate-500 dark:text-slate-400'>
                    公钥 PEM（SPKI 格式，可用于比对私钥是否与 CSR 匹配）
                  </span>
                  <CopyButton text={result.publicKey.spkiPem} />
                </div>
                <textarea
                  readOnly
                  value={result.publicKey.spkiPem}
                  className={`${PEM_TEXTAREA_CLASSES} h-32`}
                />
              </div>
            </SectionCard>

            {/* 私钥校验 */}
            {result.privateKey && (
              <SectionCard
                title='私钥校验'
                actions={
                  <span
                    className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
                      result.privateKey.matchesCsr
                        ? 'bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300'
                        : 'bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-300'
                    }`}>
                    {result.privateKey.matchesCsr ? '✓ 私钥与 CSR 匹配' : '✗ 私钥与 CSR 不匹配'}
                  </span>
                }>
                <div>
                  <InfoRow label='私钥格式'>{result.privateKey.format}</InfoRow>
                  <InfoRow label='密钥类型'>
                    {result.privateKey.keyType}
                    {result.privateKey.keySize != null && ` · ${result.privateKey.keySize} 位`}
                    {result.privateKey.curveName && ` · 曲线 ${result.privateKey.curveName}`}
                  </InfoRow>
                  <InfoRow label='公钥指纹 (SHA-256)'>
                    <span className='flex items-center gap-2'>
                      <span className='break-all'>{result.privateKey.fingerprintSha256}</span>
                      <CopyButton
                        text={result.privateKey.fingerprintSha256}
                        label='复制指纹'
                      />
                    </span>
                  </InfoRow>
                </div>
                {!result.privateKey.matchesCsr && (
                  <p className='mt-2 text-xs text-red-600 dark:text-red-400'>
                    该私钥与 CSR 中的公钥不一致：请确认是否拿错了密钥文件
                  </p>
                )}
              </SectionCard>
            )}

            {/* 请求的扩展 */}
            <SectionCard title='请求的扩展'>
              {result.requestedExtensions.length === 0 ? (
                <p className='text-xs text-slate-500 dark:text-slate-400'>
                  该 CSR 未包含扩展请求（SAN、Key Usage 等也可能在 CA 签发时补充）
                </p>
              ) : (
                <ul className='space-y-3'>
                  {result.requestedExtensions.map((ext, index) => (
                    <li
                      key={`${ext.name}-${index}`}
                      className='border border-slate-200 dark:border-slate-700 rounded-lg p-3 bg-white dark:bg-slate-900'>
                      <div className='flex items-center gap-2'>
                        <span className='text-sm font-medium text-slate-800 dark:text-slate-100'>
                          {ext.name}
                        </span>
                        {ext.critical && (
                          <span className='px-1.5 py-0.5 rounded text-[10px] font-medium bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300'>
                            critical
                          </span>
                        )}
                      </div>
                      <p className='mt-1.5 text-xs font-mono text-slate-600 dark:text-slate-300 break-all whitespace-pre-wrap'>
                        {ext.value}
                      </p>
                    </li>
                  ))}
                </ul>
              )}
            </SectionCard>

            {/* 原始 PEM */}
            <SectionCard
              title='原始 CSR（PEM）'
              actions={
                <div className='flex items-center gap-2'>
                  <CopyButton text={result.pem} />
                  <Button variant='secondary' size='sm' onClick={() => void handleDownloadPem()}>
                    下载 .csr
                  </Button>
                </div>
              }>
              <textarea readOnly value={result.pem} className={`${PEM_TEXTAREA_CLASSES} h-40`} />
              {downloadMessage && (
                <p
                  className={`mt-2 text-xs ${
                    downloadMessage.includes('成功')
                      ? 'text-green-600 dark:text-green-400'
                      : 'text-red-600 dark:text-red-400'
                  }`}>
                  {downloadMessage}
                </p>
              )}
            </SectionCard>
          </>
        )}
      </div>
    </ToolLayout>
  )
}

export default CsrViewer
