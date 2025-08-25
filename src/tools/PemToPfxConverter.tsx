import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { writeFile } from '@tauri-apps/plugin-fs'
import React, { useState } from 'react'
import { Button } from '../components/common'
import FileUpload from '../components/common/FileUpload'
import { ToolLayout } from '../components/layouts'

interface PemToPfxResult {
  pfx_data: number[]
  success: boolean
  error?: string
}

const PemToPfxConverter: React.FC = () => {
  const [pemContent, setPemContent] = useState('')
  const [privateKeyContent, setPrivateKeyContent] = useState('')
  const [pfxData, setPfxData] = useState<number[] | null>(null)
  const [password, setPassword] = useState('')
  const [showPfxPassword, setShowPfxPassword] = useState(false)
  const [privateKeyPassword, setPrivateKeyPassword] = useState('')
  const [showPrivateKeyPassword, setShowPrivateKeyPassword] = useState(false)
  const [showOpensslInfo, setShowOpensslInfo] = useState(false)
  const [isProcessing, setIsProcessing] = useState(false)
  // 分离错误：表单级与字段级
  const [formError, setFormError] = useState<string | null>('')
  const [pemError, setPemError] = useState<string | null>(null)
  const [keyError, setKeyError] = useState<string | null>(null)
  const [certInfo, setCertInfo] = useState<{
    hasCert: boolean
    hasPrivateKey: boolean
    certOnly: string
    extractedPrivateKey: string | null
  }>({
    hasCert: false,
    hasPrivateKey: false,
    certOnly: '',
    extractedPrivateKey: null,
  })

  // 更新证书信息
  const updateCertificateInfo = (content: string) => {
    if (!content || typeof content !== 'string') {
      setCertInfo({
        hasCert: false,
        hasPrivateKey: false,
        certOnly: '',
        extractedPrivateKey: null,
      })
      return
    }

    const hasCert = isValidPEM(content)
    const hasPrivateKey = containsPrivateKey(content)

    if (hasCert) {
      const parsed = parseCertificateContent(content)
      setCertInfo({
        hasCert: true,
        hasPrivateKey: hasPrivateKey,
        certOnly: parsed.cert || '',
        extractedPrivateKey: parsed.privateKey,
      })
    } else {
      setCertInfo({
        hasCert: false,
        hasPrivateKey: false,
        certOnly: '',
        extractedPrivateKey: null,
      })
    }
  }

  // 验证PEM格式
  const isValidPEM = (content: string): boolean => {
    if (!content || typeof content !== 'string') return false
    const hasBegin = content.includes('-----BEGIN CERTIFICATE-----')
    const hasEnd = content.includes('-----END CERTIFICATE-----')
    return hasBegin && hasEnd
  }

  // 验证私钥格式
  const isValidPrivateKey = (content: string): boolean => {
    if (!content || typeof content !== 'string') return false
    const hasRsaBegin = content.includes('-----BEGIN RSA PRIVATE KEY-----')
    const hasRsaEnd = content.includes('-----END RSA PRIVATE KEY-----')
    const hasEcBegin = content.includes('-----BEGIN EC PRIVATE KEY-----')
    const hasEcEnd = content.includes('-----END EC PRIVATE KEY-----')
    const hasPkcs8Begin = content.includes('-----BEGIN PRIVATE KEY-----')
    const hasPkcs8End = content.includes('-----END PRIVATE KEY-----')

    return (
      (hasRsaBegin && hasRsaEnd) ||
      (hasEcBegin && hasEcEnd) ||
      (hasPkcs8Begin && hasPkcs8End)
    )
  }

  // 判断私钥是否被加密（粗略检测常见情况）
  const isEncryptedPrivateKey = (content: string): boolean => {
    if (!content || typeof content !== 'string') return false
    if (content.includes('-----BEGIN ENCRYPTED PRIVATE KEY-----')) return true
    const hasLegacyKeyHeader =
      content.includes('-----BEGIN RSA PRIVATE KEY-----') ||
      content.includes('-----BEGIN EC PRIVATE KEY-----')
    if (hasLegacyKeyHeader && /Proc-Type:\s*4,ENCRYPTED/i.test(content)) {
      return true
    }
    // 宽松匹配：存在旧格式私钥并包含 ENCRYPTED 关键字
    if (hasLegacyKeyHeader && /ENCRYPTED/i.test(content)) {
      return true
    }
    return false
  }

  // 检查证书内容中是否包含私钥
  const containsPrivateKey = (content: string): boolean => {
    if (!content || typeof content !== 'string') return false
    const hasRsaKey =
      content.includes('-----BEGIN RSA PRIVATE KEY-----') &&
      content.includes('-----END RSA PRIVATE KEY-----')
    const hasEcKey =
      content.includes('-----BEGIN EC PRIVATE KEY-----') &&
      content.includes('-----END EC PRIVATE KEY-----')
    const hasPkcs8Key =
      content.includes('-----BEGIN PRIVATE KEY-----') &&
      content.includes('-----END PRIVATE KEY-----')
    return hasRsaKey || hasEcKey || hasPkcs8Key
  }

  // 解析证书内容，提取证书和私钥部分
  const parseCertificateContent = (
    content: string,
  ): { cert: string; privateKey: string | null } => {
    if (!content || typeof content !== 'string') {
      return { cert: '', privateKey: null }
    }

    const lines = content.split('\n')
    let certLines: string[] = []
    let privateKeyLines: string[] = []
    let inCert = false
    let inPrivateKey = false

    for (const line of lines) {
      const trimmedLine = line.trim()
      if (!trimmedLine) continue

      if (trimmedLine.includes('-----BEGIN CERTIFICATE-----')) {
        inCert = true
        inPrivateKey = false
        certLines.push(trimmedLine)
      } else if (trimmedLine.includes('-----END CERTIFICATE-----')) {
        inCert = false
        certLines.push(trimmedLine)
      } else if (
        trimmedLine.includes('-----BEGIN RSA PRIVATE KEY-----') ||
        trimmedLine.includes('-----BEGIN EC PRIVATE KEY-----') ||
        trimmedLine.includes('-----BEGIN PRIVATE KEY-----')
      ) {
        inPrivateKey = true
        inCert = false
        privateKeyLines.push(trimmedLine)
      } else if (
        trimmedLine.includes('-----END RSA PRIVATE KEY-----') ||
        trimmedLine.includes('-----END EC PRIVATE KEY-----') ||
        trimmedLine.includes('-----END PRIVATE KEY-----')
      ) {
        inPrivateKey = false
        privateKeyLines.push(trimmedLine)
      } else if (inCert) {
        certLines.push(trimmedLine)
      } else if (inPrivateKey) {
        privateKeyLines.push(trimmedLine)
      }
    }

    const cert = certLines.join('\n').trim()
    const privateKey =
      privateKeyLines.length > 0 ? privateKeyLines.join('\n').trim() : null

    return {
      cert: cert || '',
      privateKey: privateKey,
    }
  }

  // PEM转PFX
  const convertPEMtoPFX = async () => {
    if (!pemContent.trim()) {
      setPemError('请提供PEM证书内容')
      setFormError(null)
      return
    }

    if (!password.trim()) {
      setFormError('请设置PFX密码')
      return
    }

    // 如果证书不包含私钥，则私钥内容为必填项
    if (!certInfo.hasPrivateKey && !privateKeyContent.trim()) {
      setKeyError('请提供私钥内容')
      setFormError(null)
      return
    }

    setIsProcessing(true)
    setPemError(null)
    setKeyError(null)
    setFormError(null)

    try {
      // 获取最终使用的私钥内容
      const finalPrivateKey =
        privateKeyContent.trim() || certInfo.extractedPrivateKey || ''

      // 如果私钥被加密，则必须填写私钥密码
      if (
        finalPrivateKey &&
        isEncryptedPrivateKey(finalPrivateKey) &&
        !privateKeyPassword.trim()
      ) {
        setKeyError('检测到加密私钥，请填写私钥密码')
        return
      }

      // 验证证书内容
      if (!certInfo.certOnly.trim()) {
        throw new Error('无法提取有效的证书内容')
      }

      // 清理和验证输入内容
      const cleanCert = certInfo.certOnly.trim()
      const cleanPrivateKey = finalPrivateKey.trim() || null
      const cleanPassword = password.trim()
      const cleanPrivateKeyPassword = privateKeyPassword.trim() || null

      // 调用Rust后端进行转换
      const result: PemToPfxResult = await invoke('convert_pem_to_pfx', {
        certPem: cleanCert,
        privateKeyPem: cleanPrivateKey,
        password: cleanPassword,
        privateKeyPassword: cleanPrivateKeyPassword,
      })

      if (result.success && result.pfx_data && result.pfx_data.length > 0) {
        setPfxData(result.pfx_data)
        setFormError(null)
      } else {
        const errorMsg = result.error || '转换失败'
        if (errorMsg.includes('证书')) {
          setPemError(errorMsg)
        } else if (errorMsg.includes('私钥')) {
          setKeyError(errorMsg)
        } else {
          setFormError(errorMsg)
        }
        setPfxData(null)
      }
    } catch (err: any) {
      console.error('PEM转PFX转换错误:', err)

      let errorMessage = '转换失败'

      if (err && typeof err === 'string') {
        errorMessage = err
      } else if (err && err.message) {
        errorMessage = err.message
      } else if (err && err.toString) {
        errorMessage = err.toString()
      }

      // 提供更友好的错误信息
      if (errorMessage.includes('证书内容不能为空')) {
        errorMessage = '证书内容不能为空，请提供有效的证书内容'
      } else if (errorMessage.includes('证书格式不正确')) {
        errorMessage =
          '证书格式不正确，请确保证书以-----BEGIN CERTIFICATE-----开头'
      } else if (errorMessage.includes('私钥内容不能为空')) {
        errorMessage = '私钥内容不能为空，请提供有效的私钥内容'
      } else if (errorMessage.includes('私钥格式不正确')) {
        errorMessage = '私钥格式不正确，请确保私钥包含正确的BEGIN和END标记'
      } else if (errorMessage.includes('私钥密码错误')) {
        errorMessage = '私钥密码错误或私钥格式不正确'
      } else if (errorMessage.includes('PFX密码不能为空')) {
        errorMessage = 'PFX密码不能为空，请设置PFX文件密码'
      } else if (errorMessage.includes('PKCS12构建失败')) {
        errorMessage = 'PFX文件构建失败：证书和私钥可能不匹配'
      } else if (errorMessage.includes('证书解析失败')) {
        errorMessage = '证书解析失败：请检查证书格式是否正确'
      } else if (errorMessage.includes('私钥解析失败')) {
        errorMessage = '私钥解析失败：请检查私钥格式和密码是否正确'
      } else if (
        errorMessage === '转换失败' ||
        errorMessage.includes('未知错误')
      ) {
        errorMessage = '转换失败：请检查证书和私钥内容是否匹配'
      }

      if (errorMessage.includes('证书')) {
        setPemError(errorMessage)
      } else if (errorMessage.includes('私钥')) {
        setKeyError(errorMessage)
      } else {
        setFormError(errorMessage)
      }
      setPfxData(null)
    } finally {
      setIsProcessing(false)
    }
  }

  // 下载PFX文件
  const downloadPFX = async () => {
    if (!pfxData) return

    try {
      // 打开保存对话框
      const filePath = await save({
        defaultPath: 'certificate.pfx',
        filters: [
          {
            name: 'PFX文件',
            extensions: ['pfx', 'p12'],
          },
        ],
      })

      if (filePath) {
        // 将数据写入文件
        await writeFile(filePath, new Uint8Array(pfxData))
        setFormError(null)

        // 显示成功消息
        setTimeout(() => {
          setFormError('PFX文件已成功保存！')
          setTimeout(() => setFormError(null), 3000)
        }, 100)
      }
    } catch (err) {
      console.error('保存文件失败:', err)
      setFormError('保存文件失败：' + (err as Error).message)
    }
  }

  // 处理PEM文件上传
  const handlePemFileUpload = (content: string) => {
    setPemContent(content)
    updateCertificateInfo(content)
    if (content) setPemError(null)
  }

  // 处理私钥文件上传
  const handlePrivateKeyFileUpload = (content: string) => {
    setPrivateKeyContent(content)
    if (content) setKeyError(null)
  }

  return (
    <ToolLayout 
      title="PEM 转 PFX 转换器"
      subtitle="将PEM格式的证书和私钥转换为PFX格式文件"
      actions={
        <Button 
          variant='secondary'
          size='sm'
          onClick={() => setShowOpensslInfo(!showOpensslInfo)}>
          {showOpensslInfo ? '隐藏' : '查看'}OpenSSL命令
        </Button>
      }
    >
      <div className='flex flex-col h-full space-y-6 overflow-y-auto'>
        {/* OpenSSL命令提示 */}
        {showOpensslInfo && (
          <div className='mb-6 p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg'>
          <div className='flex items-center justify-between mb-3'>
            <h3 className='text-sm font-semibold text-blue-800 dark:text-blue-200'>
              📋 OpenSSL命令参考
            </h3>
            <button
              onClick={() => setShowOpensslInfo(false)}
              className='text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300'>
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
                基本转换：
              </p>
              <code className='block p-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded font-mono text-blue-700 dark:text-blue-300'>
                openssl pkcs12 -export -out certificate.pfx -inkey private.key
                -in certificate.crt
              </code>
            </div>
            <div>
              <p className='font-medium text-blue-800 dark:text-blue-200 mb-1'>
                带私钥密码：
              </p>
              <code className='block p-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded font-mono text-blue-700 dark:text-blue-300'>
                openssl pkcs12 -export -out certificate.pfx -inkey private.key
                -in certificate.crt -passin pass:私钥密码 -passout pass:PFX密码
              </code>
            </div>
            <div>
              <p className='font-medium text-blue-800 dark:text-blue-200 mb-1'>
                从PEM文件：
              </p>
              <code className='block p-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded font-mono text-blue-700 dark:text-blue-300'>
                openssl pkcs12 -export -out certificate.pfx -inkey key.pem -in
                cert.pem -certfile chain.pem
              </code>
            </div>
            <div>
              <p className='font-medium text-blue-800 dark:text-blue-200 mb-1'>
                仅证书和私钥：
              </p>
              <code className='block p-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded font-mono text-blue-700 dark:text-blue-300'>
                openssl pkcs12 -export -out certificate.pfx -inkey private.pem
                -in cert.pem
              </code>
            </div>
          </div>
          <p className='mt-3 text-xs text-blue-700 dark:text-blue-300 italic'>
            💡
            此工具为可视化版本，上述命令仅供参考。转换结果与命令行工具完全一致。
          </p>
        </div>
        )}

        {/* PEM证书输入 */}
        <div className='border border-gray-200 dark:border-gray-700 rounded-lg p-4'>
          <h3 className='text-lg font-semibold mb-3 text-gray-900 dark:text-white'>
            PEM证书
          </h3>
          <FileUpload
            value={pemContent}
            onChange={handlePemFileUpload}
            onError={setPemError}
            error={pemError}
            placeholder='粘贴PEM格式的证书内容，或以-----BEGIN CERTIFICATE-----开头...'
            accept='.pem,.crt,.cer'
            fileType='text'
          />
          {certInfo.hasCert && (
            <div className='mt-2 space-y-2'>
              <div className='p-2 bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-300 rounded text-sm'>
                证书格式有效
              </div>
              {certInfo.hasPrivateKey && (
                <div className='p-2 bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 rounded text-sm'>
                  证书已包含私钥，无需额外输入
                </div>
              )}
            </div>
          )}
        </div>

        {/* 私钥输入 - 仅在证书已输入但不包含私钥时显示 */}
        {certInfo.hasCert && !certInfo.hasPrivateKey && (
          <div className='border border-gray-200 dark:border-gray-700 rounded-lg p-4'>
            <h3 className='text-lg font-semibold mb-3 text-gray-900 dark:text-white'>
              私钥 *
            </h3>
            <FileUpload
              value={privateKeyContent}
              onChange={handlePrivateKeyFileUpload}
              onError={setKeyError}
              error={keyError}
              placeholder='请粘贴PEM格式的私钥内容，或以-----BEGIN PRIVATE KEY-----开头...'
              accept='.key,.pem'
              fileType='text'
            />
            {privateKeyContent && isValidPrivateKey(privateKeyContent) && (
              <div className='mt-2 p-2 bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-300 rounded text-sm'>
                私钥格式有效
              </div>
            )}
          </div>
        )}

        {/* 密码设置区域 */}
        <div className='border border-gray-200 dark:border-gray-700 rounded-lg p-4'>
          <h3 className='text-lg font-semibold mb-3 text-gray-900 dark:text-white'>
            密码设置
          </h3>
          <div className='grid grid-cols-1 md:grid-cols-2 gap-4'>
            {/* 私钥密码输入 - 仅在私钥加密时显示 */}
            {(() => {
              const effectiveKey = (
                privateKeyContent ||
                certInfo.extractedPrivateKey ||
                ''
              ).trim()
              const needsPassword = isEncryptedPrivateKey(effectiveKey)
              return needsPassword ? (
                <div>
              <div className='flex flex-col sm:flex-row sm:items-center sm:space-x-3 space-y-2 sm:space-y-0'>
                  <label className='w-28 shrink-0 text-sm font-medium text-gray-700 dark:text-gray-300 mb-0'>
                    {(() => {
                      const effectiveKey = (
                        privateKeyContent ||
                        certInfo.extractedPrivateKey ||
                        ''
                      ).trim()
                      // 如果证书不包含私钥，则私钥密码必填
                      if (!certInfo.hasPrivateKey) {
                        return isEncryptedPrivateKey(effectiveKey)
                          ? '私钥密码 *'
                          : '私钥密码 *'
                      }
                      // 如果证书包含私钥，则根据是否加密决定
                      return isEncryptedPrivateKey(effectiveKey)
                        ? '私钥密码 *'
                        : '私钥密码（可选）'
                    })()}
                  </label>
                  <div className='relative w-full'>
                    <input
                      type={showPrivateKeyPassword ? 'text' : 'password'}
                      value={privateKeyPassword}
                      onChange={(e) => setPrivateKeyPassword(e.target.value)}
                      placeholder='如果私钥需要密码...'
                      className='w-full px-3 py-2 pr-10 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500'
                    />
                    <button
                      type='button'
                      onClick={() =>
                        setShowPrivateKeyPassword(!showPrivateKeyPassword)
                      }
                      className='absolute inset-y-0 right-0 px-3 flex items-center text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'
                      title={showPrivateKeyPassword ? '隐藏密码' : '显示密码'}>
                      {showPrivateKeyPassword ? (
                        <svg
                          className='w-5 h-5'
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
                          className='w-5 h-5'
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
              </div>
              ) : null
            })()}

            {/* PFX密码设置 */}
            <div className={(() => {
              const effectiveKey = (
                privateKeyContent ||
                certInfo.extractedPrivateKey ||
                ''
              ).trim()
              const needsPassword = isEncryptedPrivateKey(effectiveKey)
              return needsPassword ? '' : 'md:col-span-2'
            })()}>
              <div className='flex flex-col sm:flex-row sm:items-center sm:space-x-3 space-y-2 sm:space-y-0'>
                <label className='w-28 shrink-0 text-sm font-medium text-gray-700 dark:text-gray-300 mb-0'>
                  PFX密码 *
                </label>
                <div className='relative w-full'>
                  <input
                    type={showPfxPassword ? 'text' : 'password'}
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    placeholder='设置PFX文件密码...'
                    className='w-full px-3 py-2 pr-10 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500'
                  />
                  <button
                    type='button'
                    onClick={() => setShowPfxPassword(!showPfxPassword)}
                    className='absolute inset-y-0 right-0 px-3 flex items-center text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'
                    title={showPfxPassword ? '隐藏密码' : '显示密码'}>
                    {showPfxPassword ? (
                      <svg
                        className='w-5 h-5'
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
                        className='w-5 h-5'
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
              <p className='mt-1 text-xs text-gray-600 dark:text-gray-400'>
                用于保护生成的PFX文件
              </p>
            </div>
          </div>
        </div>

        {/* 操作按钮区域 */}
        <div>
          <Button
            variant='primary'
            size='lg'
            onClick={convertPEMtoPFX}
            disabled={isProcessing || !pemContent.trim() || !password.trim() || (!certInfo.hasPrivateKey && !privateKeyContent.trim())}
            className='w-full'>
            {isProcessing ? '转换中...' : '转换为PFX'}
          </Button>
        </div>

        {formError && (
            <div
              className='mt-3 p-3 rounded-md border text-sm '
              style={{
                backgroundColor: formError.includes('成功')
                  ? 'var(--tw-color-green-50, #f0fdf4)'
                  : undefined,
              }}>
              <div
                className={
                  formError.includes('成功')
                    ? 'text-green-700 dark:text-green-300'
                    : 'text-red-700 dark:text-red-300'
                }>
                {formError}
              </div>
            </div>
          )}

        {/* 结果下载区域 */}
        {pfxData && (
          <div className='border border-gray-200 dark:border-gray-700 rounded-lg p-4'>
            <h3 className='text-lg font-semibold mb-3 text-gray-900 dark:text-white'>
              转换结果
            </h3>
            <div className='space-y-3'>
              <div className='p-3 bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-300 rounded text-sm'>
                转换成功！文件大小：{(pfxData.length / 1024).toFixed(2)} KB
              </div>
              <Button
                variant='primary'
                size='sm'
                onClick={downloadPFX}
                className='w-full'>
                下载PFX文件
              </Button>
            </div>
          </div>
        )}
      </div>
    </ToolLayout>
  )
}

export default PemToPfxConverter
