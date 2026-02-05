import React, { useCallback, useEffect, useState } from 'react'
import { Button, CodeEditor } from '../components/common'
import FileUpload from '../components/common/FileUpload'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard, useDebounce } from '../hooks'
import { errorUtils } from '../utils'

/**
 * JWT 解码工具
 * 使用重构后的公共组件，提供统一的用户体验
 */
const JwtDecode: React.FC = () => {
  const [input, setInput] = useState('')
  const [header, setHeader] = useState('')
  const [payload, setPayload] = useState('')
  const [signature, setSignature] = useState('')
  const [algorithm, setAlgorithm] = useState('')
  const [error, setError] = useState('')

  // JWT验证相关状态
  const [secretKey, setSecretKey] = useState('')
  const [publicKey, setPublicKey] = useState('')
  const [verificationResult, setVerificationResult] = useState<{
    isValid: boolean
    message: string
    type: 'success' | 'error' | 'warning'
  } | null>(null)
  const [publicKeyError, setPublicKeyError] = useState<string | null>(null)
  const [showVerification, setShowVerification] = useState(false)

  const { copy, copied } = useCopyToClipboard()

  // 使用防抖处理，避免频繁的解码操作
  const debouncedInput = useDebounce(input, 300)

  // 验证RSA公钥格式的有效性
  const validateRSAPublicKey = useCallback((key: string) => {
    // 基本的格式检查
    const beginPattern = /-----BEGIN (RSA )?PUBLIC KEY-----/
    const endPattern = /-----END (RSA )?PUBLIC KEY-----/

    if (!beginPattern.test(key) || !endPattern.test(key)) {
      return false
    }

    // 检查是否有实际的内容
    const content = key
      .replace(/-----BEGIN (RSA )?PUBLIC KEY-----/, '')
      .replace(/-----END (RSA )?PUBLIC KEY-----/, '')
      .trim()

    // 检查内容是否为空
    if (content.length === 0) {
      return false
    }

    // 检查内容是否只包含base64字符和换行符
    const base64Pattern = /^[A-Za-z0-9+/=\s]+$/
    return base64Pattern.test(content)
  }, [])

  // JWT验证函数
  const verifyJWT = async (token: string, alg: string, key: string) => {
    try {
      // 动态导入jsrsasign
      const jose = await import('jsrsasign')

      if (alg.startsWith('HS')) {
        // HMAC算法验证
        const isValid = jose.KJUR.jws.JWS.verify(token, { utf8: key }, [alg])
        return {
          isValid,
          message: isValid ? 'JWT签名验证成功' : 'JWT签名验证失败：签名不匹配',
          type: isValid ? 'success' : 'error',
        } as const
      } else if (alg.startsWith('RS')) {
        // RSA算法验证
        if (!validateRSAPublicKey(key)) {
          return {
            isValid: false,
            message: 'RSA公钥格式无效，请检查公钥内容',
            type: 'error',
          } as const
        }

        try {
          // 对于RSA算法，直接使用公钥字符串进行验证
          const isValid = jose.KJUR.jws.JWS.verify(token, key, [alg])

          return {
            isValid,
            message: isValid
              ? 'JWT签名验证成功'
              : 'JWT签名验证失败：签名不匹配',
            type: isValid ? 'success' : 'error',
          } as const
        } catch (keyError) {
          return {
            isValid: false,
            message: 'RSA公钥解析失败，请检查公钥格式',
            type: 'error',
          } as const
        }
      } else {
        return {
          isValid: false,
          message: `不支持的签名算法：${alg}`,
          type: 'warning',
        } as const
      }
    } catch (error) {
      return {
        isValid: false,
        message: `验证过程出错：${
          error instanceof Error ? error.message : '未知错误'
        }`,
        type: 'error',
      } as const
    }
  }

  // 处理JWT验证
  const handleVerifyJWT = async () => {
    if (!input.trim()) {
      setVerificationResult({
        isValid: false,
        message: '请先输入JWT令牌',
        type: 'warning',
      })
      return
    }

    if (!algorithm) {
      setVerificationResult({
        isValid: false,
        message: '无法获取JWT签名算法',
        type: 'error',
      })
      return
    }

    const key = algorithm.startsWith('HS') ? secretKey : publicKey
    if (!key.trim()) {
      setVerificationResult({
        isValid: false,
        message: `请输入${
          algorithm.startsWith('HS') ? 'Secret Key' : 'RSA公钥'
        }`,
        type: 'warning',
      })
      return
    }

    const result = await verifyJWT(input.trim(), algorithm, key)
    setVerificationResult(result)
  }

  const handleDecode = (token: string) => {
    if (!token.trim()) {
      // 清除所有状态
      setHeader('')
      setPayload('')
      setSignature('')
      setAlgorithm('')
      setError('')
      setVerificationResult(null)
      return
    }

    try {
      setError('')
      setVerificationResult(null)
      const parts = token.split('.')
      if (parts.length !== 3) {
        throw new Error('无效的JWT格式：JWT应该包含3个部分，用点号分隔')
      }

      // 解码头部
      const headerData = JSON.parse(atob(parts[0]))
      setHeader(JSON.stringify(headerData, null, 2))
      setAlgorithm(headerData.alg || 'Unknown')

      // 解码载荷
      const payloadData = JSON.parse(atob(parts[1]))
      setPayload(JSON.stringify(payloadData, null, 2))

      setSignature(parts[2])

      // 根据算法显示验证区域
      setShowVerification(true)
    } catch (err) {
      setHeader('')
      setPayload('')
      setSignature('')
      setAlgorithm('')
      setError(errorUtils.formatError(err, 'JWT解码失败'))
      setVerificationResult(null)
      setShowVerification(false)
    }
  }

  useEffect(() => {
    handleDecode(debouncedInput)
  }, [debouncedInput])

  const handleClear = () => {
    setInput('')
    setSecretKey('')
    setPublicKey('')
    setVerificationResult(null)
    setPublicKeyError(null)
    setShowVerification(false)
  }

  const handleCopyHeader = async () => {
    if (header) {
      await copy(header)
    }
  }

  const handleCopyPayload = async () => {
    if (payload) {
      await copy(payload)
    }
  }

  const handleCopySignature = async () => {
    if (signature) {
      await copy(signature)
    }
  }

  return (
    <ToolLayout
      title='JWT 解码'
      subtitle='解析和查看JWT令牌的头部、载荷和签名'
      description='输入JWT令牌以查看其结构化内容，包括算法、有效期等信息'>
      <div className='flex flex-col h-full'>
        <div className='flex-1 overflow-y-auto space-y-3 px-1'>
          {/* JWT Token输入区域 */}
          <div className='flex-shrink-0'>
            <div className='flex items-center justify-between mb-2'>
              <label className='text-sm font-medium text-slate-700 dark:text-slate-200'>
                JWT Token
              </label>
              <div className='flex items-center space-x-3'>
                <div className='text-sm text-slate-600 dark:text-slate-400'>
                  输入长度: {input.length}
                </div>
                <Button
                  variant='secondary'
                  size='sm'
                  onClick={handleClear}
                  disabled={!input}>
                  清空输入
                </Button>
              </div>
            </div>
            <div className='h-[120px]'>
              <CodeEditor
                language='plaintext'
                value={input}
                onChange={setInput}
                placeholder='请输入JWT令牌...'
                options={{
                  lineNumbers: 'off',
                  glyphMargin: false,
                  minimap: { enabled: false },
                }}
              />
            </div>
          </div>

          {/* JWT签名验证区域 */}
          {showVerification && algorithm && (
            <div className='flex-shrink-0'>
              <div className='p-4 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg'>
                <div className='flex items-center justify-between mb-4'>
                  <h3 className='text-lg font-semibold text-yellow-800 dark:text-yellow-200'>
                    ⚠️ JWT签名验证
                  </h3>
                  <div className='text-sm text-yellow-700 dark:text-yellow-300'>
                    算法: <span className='font-medium'>{algorithm}</span>
                  </div>
                </div>

                {/* 警告提示 */}
                <div className='mb-4 p-3 bg-yellow-100 dark:bg-yellow-900/40 border border-yellow-300 dark:border-yellow-700 rounded'>
                  <div className='flex items-start space-x-2'>
                    <svg
                      className='w-5 h-5 text-yellow-600 dark:text-yellow-400 mt-0.5'
                      fill='none'
                      stroke='currentColor'
                      viewBox='0 0 24 24'>
                      <path
                        strokeLinecap='round'
                        strokeLinejoin='round'
                        strokeWidth={2}
                        d='M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z'
                      />
                    </svg>
                    <div className='text-sm text-yellow-800 dark:text-yellow-200'>
                      <p className='font-medium mb-1'>安全提醒</p>
                      <p>
                        JWT签名验证通常在服务端进行。在客户端验证仅用于调试目的，请勿在生产环境中依赖客户端验证结果。
                      </p>
                    </div>
                  </div>
                </div>

                {/* 密钥输入区域 */}
                <div className='grid grid-cols-1 lg:grid-cols-2 gap-4 mb-4'>
                  {algorithm.startsWith('HS') ? (
                    // HMAC Secret Key输入
                    <div className='lg:col-span-2'>
                      <label className='block text-sm font-medium text-yellow-800 dark:text-yellow-200 mb-2'>
                        Secret Key
                      </label>
                      <input
                        type='text'
                        value={secretKey}
                        onChange={(e) => setSecretKey(e.target.value)}
                        className='w-full p-3 border border-yellow-300 dark:border-yellow-600 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-yellow-500 focus:border-yellow-500 dark:bg-slate-700 dark:text-white font-mono'
                        placeholder='请输入用于签名的Secret Key...'
                      />
                      <p className='text-xs text-yellow-700 dark:text-yellow-300 mt-1'>
                        💡 输入生成JWT时使用的密钥
                      </p>
                    </div>
                  ) : (
                    // RSA Public Key输入
                    <div className='lg:col-span-2'>
                      <label className='block text-sm font-medium text-yellow-800 dark:text-yellow-200 mb-2'>
                        RSA Public Key
                      </label>
                      <FileUpload
                        value={publicKey}
                        onChange={(value) => {
                          setPublicKey(value)
                          // 验证公钥格式
                          if (value && !validateRSAPublicKey(value)) {
                            setPublicKeyError(
                              'RSA公钥格式无效，请检查或重新选择文件',
                            )
                          } else {
                            setPublicKeyError(null)
                          }
                        }}
                        onError={setPublicKeyError}
                        error={publicKeyError}
                        accept='.pem,.key,.pub'
                        fileType='text'
                        placeholder='请输入RSA公钥内容...'
                        className='w-full'
                      />
                      <p className='text-xs text-yellow-700 dark:text-yellow-300 mt-1'>
                        💡 输入与签名私钥对应的RSA公钥
                      </p>
                    </div>
                  )}
                </div>

                {/* 验证按钮 */}
                <div className='flex justify-center'>
                  <Button
                    variant={copied ? 'success' : 'primary'}
                    size='md'
                    onClick={handleVerifyJWT}
                    disabled={
                      algorithm.startsWith('HS')
                        ? !secretKey.trim()
                        : !publicKey.trim() || !!publicKeyError
                    }
                    className='px-6 bg-yellow-600 hover:bg-yellow-700 focus:ring-yellow-500'>
                    验证JWT签名
                  </Button>
                </div>
              </div>
            </div>
          )}

          {/* 验证结果提示 - 独立显示 */}
          {verificationResult && (
            <div className='flex-shrink-0'>
              <div
                className={`p-3 rounded-lg border ${
                  verificationResult.type === 'success'
                    ? 'bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-800'
                    : verificationResult.type === 'warning'
                    ? 'bg-yellow-50 dark:bg-yellow-900/20 border-yellow-200 dark:border-yellow-800'
                    : 'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800'
                }`}>
                <div className='flex items-center space-x-2'>
                  {verificationResult.type === 'success' ? (
                    <svg
                      className='w-5 h-5 text-green-600 dark:text-green-400'
                      fill='none'
                      stroke='currentColor'
                      viewBox='0 0 24 24'>
                      <path
                        strokeLinecap='round'
                        strokeLinejoin='round'
                        strokeWidth={2}
                        d='M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z'
                      />
                    </svg>
                  ) : verificationResult.type === 'warning' ? (
                    <svg
                      className='w-5 h-5 text-yellow-600 dark:text-yellow-400'
                      fill='none'
                      stroke='currentColor'
                      viewBox='0 0 24 24'>
                      <path
                        strokeLinecap='round'
                        strokeLinejoin='round'
                        strokeWidth={2}
                        d='M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z'
                      />
                    </svg>
                  ) : (
                    <svg
                      className='w-5 h-5 text-red-600 dark:text-red-400'
                      fill='none'
                      stroke='currentColor'
                      viewBox='0 0 24 24'>
                      <path
                        strokeLinecap='round'
                        strokeLinejoin='round'
                        strokeWidth={2}
                        d='M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z'
                      />
                    </svg>
                  )}
                  <span
                    className={`font-medium ${
                      verificationResult.type === 'success'
                        ? 'text-green-800 dark:text-green-200'
                        : verificationResult.type === 'warning'
                        ? 'text-yellow-800 dark:text-yellow-200'
                        : 'text-red-800 dark:text-red-200'
                    }`}>
                    {verificationResult.message}
                  </span>
                </div>
              </div>
            </div>
          )}

          {/* 错误提示 */}
          {error && (
            <div className='flex-shrink-0 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg'>
              <p className='text-red-700 dark:text-red-400'>
                <strong>错误:</strong> {error}
              </p>
            </div>
          )}

          {/* 解码结果容器 - 使用 flex-1 并调整内部布局 */}
          <div className='flex-1 flex flex-col space-y-3'>
            {/* Header和Payload */}
            <div className='grid grid-cols-1 lg:grid-cols-2 gap-4'>
              {/* Header */}
              <div className='flex flex-col'>
                <div className='flex items-center justify-between mb-2'>
                  <h3 className='text-lg font-semibold text-slate-800 dark:text-white'>
                    Header
                  </h3>
                  {header && (
                    <Button
                      variant={copied ? 'success' : 'secondary'}
                      size='sm'
                      onClick={handleCopyHeader}>
                      {copied ? '已复制 ✓' : '复制'}
                    </Button>
                  )}
                </div>
                <div className='flex-1 min-h-[200px] h-full'>
                  <CodeEditor
                    language='json'
                    value={header}
                    readOnly={true}
                    placeholder='Header 信息将在这里显示...'
                    options={{
                      lineNumbers: 'on',
                      glyphMargin: false,
                      minimap: { enabled: false },
                      wordWrap: 'on',
                      readOnly: true,
                      domReadOnly: true,
                      scrollBeyondLastLine: false,
                    }}
                  />
                </div>
              </div>

              {/* Payload */}
              <div className='flex flex-col'>
                <div className='flex items-center justify-between mb-2'>
                  <h3 className='text-lg font-semibold text-slate-800 dark:text-white'>
                    Payload
                  </h3>
                  {payload && (
                    <Button
                      variant={copied ? 'success' : 'secondary'}
                      size='sm'
                      onClick={handleCopyPayload}>
                      {copied ? '已复制 ✓' : '复制'}
                    </Button>
                  )}
                </div>
                <div className='flex-1 min-h-[200px] h-full'>
                  <CodeEditor
                    language='json'
                    value={payload}
                    readOnly={true}
                    placeholder='Payload 信息将在这里显示...'
                    options={{
                      lineNumbers: 'on',
                      glyphMargin: false,
                      minimap: { enabled: false },
                      wordWrap: 'on',
                      readOnly: true,
                      domReadOnly: true,
                      scrollBeyondLastLine: false,
                    }}
                  />
                </div>
              </div>
            </div>

            {/* Signature - 单独一行 */}
            <div className='flex-shrink-0'>
              <div className='flex items-center justify-between mb-2'>
                <h3 className='text-lg font-semibold text-slate-800 dark:text-white'>
                  Signature
                </h3>
                {signature && (
                  <Button
                    variant={copied ? 'success' : 'secondary'}
                    size='sm'
                    onClick={handleCopySignature}>
                    {copied ? '已复制 ✓' : '复制'}
                  </Button>
                )}
              </div>
              <div className='h-[80px]'>
                <CodeEditor
                  language='plaintext'
                  value={signature}
                  readOnly={true}
                  placeholder='Signature 信息将在这里显示...'
                  options={{
                    lineNumbers: 'off',
                    glyphMargin: false,
                    minimap: { enabled: false },
                    wordWrap: 'off',
                  }}
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </ToolLayout>
  )
}

export default JwtDecode
