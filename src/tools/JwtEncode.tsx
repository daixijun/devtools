import React, { useCallback, useState } from 'react'
import { Button, CodeEditor } from '../components/common'
import FileUpload from '../components/common/FileUpload'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'
import { errorUtils } from '../utils'

// 定义Header和Claims的接口
interface JwtHeader {
  alg: string
  typ: string
}

interface JwtClaims {
  sub: string
  iss?: string
  iat: number
  exp?: number
}

const JwtEncode: React.FC = () => {
  const [secretKey, setSecretKey] = useState('mySecretKey')
  const [privateKey, setPrivateKey] = useState('')
  const [header, setHeader] = useState<JwtHeader>({
    alg: 'HS256',
    typ: 'JWT',
  })
  const [payload, setPayload] = useState<JwtClaims>({
    sub: '1234567890',
    iss: '',
    iat: Math.floor(Date.now() / 1000),
  })

  const [expiresIn, setExpiresIn] = useState('3600')
  const [output, setOutput] = useState('')
  const [headerError, setHeaderError] = useState('')
  const [payloadError, setPayloadError] = useState('')
  const [privateKeyError, setPrivateKeyError] = useState<string | null>(null)
  const { copy, copied } = useCopyToClipboard()

  // 验证RSA私钥格式的有效性
  const validateRSAPrivateKey = useCallback((key: string) => {
    // 基本的格式检查
    const beginPattern = /-----BEGIN (RSA )?PRIVATE KEY-----/
    const endPattern = /-----END (RSA )?PRIVATE KEY-----/

    if (!beginPattern.test(key) || !endPattern.test(key)) {
      return false
    }

    // 检查是否有实际的内容
    const content = key
      .replace(/-----BEGIN (RSA )?PRIVATE KEY-----/, '')
      .replace(/-----END (RSA )?PRIVATE KEY-----/, '')
      .trim()

    // 检查内容是否为空
    if (content.length === 0) {
      return false
    }

    // 检查内容是否只包含base64字符和换行符
    const base64Pattern = /^[A-Za-z0-9+/=\s]+$/
    return base64Pattern.test(content)
  }, [])

  const handleGenerate = async () => {
    try {
      // 设置过期时间
      const expiresInNum = parseInt(expiresIn)
      if (isNaN(expiresInNum) || expiresInNum <= 0) {
        setOutput('生成出错: 有效期必须是正整数')
        return
      }

      // 动态导入jsrsasign（因为它可能比较大）
      const jose = await import('jsrsasign')

      // 更新payload，设置iat为当前时间，exp为过期时间
      const updatedPayload = {
        ...payload,
        exp: Math.floor(Date.now() / 1000) + expiresInNum,
      }
      setPayload(updatedPayload)

      // 生成JWT
      let token: string
      if (header.alg.startsWith('HS')) {
        // 使用HMAC算法
        token = jose.KJUR.jws.JWS.sign(header.alg, header, updatedPayload, {
          utf8: secretKey,
        })
      } else {
        // 如果是RSA算法，验证私钥格式
        if (!validateRSAPrivateKey(privateKey)) {
          setOutput('生成出错: RSA私钥格式无效，请检查或重新选择文件')
          return
        }
        // 使用RSA算法
        token = jose.KJUR.jws.JWS.sign(
          header.alg,
          header,
          updatedPayload,
          privateKey,
        )
      }

      setOutput(token)
    } catch (error) {
      setOutput(errorUtils.formatError(error, '生成JWT失败'))
    }
  }

  const handleLoadExample = () => {
    setHeader({
      alg: 'HS256',
      typ: 'JWT',
    })
    setPayload({
      sub: '1234567890',
      iss: 'example.com',
      iat: Math.floor(Date.now() / 1000),
      exp: Math.floor(Date.now() / 1000) + 3600,
    })
    setSecretKey('your-256-bit-secret')
    setExpiresIn('3600')
  }

  const handleClearAll = () => {
    setHeader({ alg: 'HS256', typ: 'JWT' })
    setPayload({ sub: '', iss: '', iat: Math.floor(Date.now() / 1000) })
    setSecretKey('')
    setPrivateKey('')
    setExpiresIn('3600')
    setOutput('')
    setHeaderError('')
    setPayloadError('')
    setPrivateKeyError(null)
  }

  const handleCopyOutput = async () => {
    if (output && !output.startsWith('生成出错')) {
      await copy(output)
    }
  }

  const actions = (
    <div className='flex items-center space-x-3'>
      <Button variant='secondary' size='sm' onClick={handleLoadExample}>
        示例
      </Button>
      <Button variant='secondary' size='sm' onClick={handleClearAll}>
        清空
      </Button>
    </div>
  )

  return (
    <ToolLayout
      title='JWT 生成'
      subtitle='创建和签名JWT令牌'
      description='配置Header、Payload和签名密钥，生成符合标准的JWT令牌'
      actions={actions}>
      <div className='flex flex-col h-full space-y-4 overflow-y-auto'>
        {/* Header 和 Payload 水平布局 */}
        <div className='grid grid-cols-1 lg:grid-cols-2 gap-4'>
          {/* Header 输入部分 */}
          <div className='flex flex-col'>
            <div className='mb-3 h-[75px]'>
              <h3 className='text-lg font-semibold text-gray-800 dark:text-gray-200 mb-1'>
                Header
              </h3>
              <p className='text-sm text-gray-600 dark:text-gray-400'>
                包含令牌的元数据，如签名算法(alg)和类型(typ)
              </p>
            </div>
            <div className='flex-1 h-[200px]'>
              <CodeEditor
                language='json'
                value={JSON.stringify(header, null, 2)}
                onChange={(newValue) => {
                  if (newValue === undefined) return
                  try {
                    const parsed = JSON.parse(newValue)
                    setHeader(parsed)
                    setHeaderError('')
                  } catch (e) {
                    setHeaderError('Header 格式错误: 请输入有效的 JSON')
                  }
                }}
                options={{
                  minimap: { enabled: false },
                  lineNumbers: 'off',
                  glyphMargin: false,
                }}
              />
              {headerError && (
                <div className='text-red-500 text-sm mt-2'>{headerError}</div>
              )}
            </div>
          </div>

          {/* Payload 输入部分 */}
          <div className='flex flex-col'>
            <div className='mb-3 h-[75px]'>
              <h3 className='text-lg font-semibold text-gray-800 dark:text-gray-200 mb-1'>
                Payload
              </h3>
              <p className='text-sm text-gray-600 dark:text-gray-400'>
                包含声明(claims)，如主题(sub)、签发者(iss)、签发时间(iat)和过期时间(exp)
              </p>
            </div>
            <div className='flex-1 h-[200px]'>
              <CodeEditor
                language='json'
                value={JSON.stringify(payload, null, 2)}
                onChange={(newValue) => {
                  if (newValue === undefined) return
                  try {
                    const parsed = JSON.parse(newValue)
                    setPayload(parsed)
                    setPayloadError('')
                  } catch (e) {
                    setPayloadError('Payload 格式错误: 请输入有效的 JSON')
                  }
                }}
                options={{
                  minimap: { enabled: false },
                  lineNumbers: 'off',
                  glyphMargin: false,
                }}
              />
              {payloadError && (
                <div className='text-red-500 text-sm mt-2'>{payloadError}</div>
              )}
            </div>
          </div>
        </div>

        {/* 配置区域 */}
        <div className='space-y-4'>
          {/* 有效期输入 */}
          <div className='flex items-center space-x-4'>
            <label className='text-lg font-medium text-gray-700 dark:text-gray-200 w-24 flex-shrink-0'>
              有效期 (秒)
            </label>
            <input
              type='number'
              className='flex-1 p-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-800 dark:text-white'
              placeholder='输入有效期'
              value={expiresIn}
              onChange={(e) => setExpiresIn(e.target.value)}
            />
          </div>

          {/* 根据alg值显示不同的密钥输入框 */}
          <div>
            {header.alg.startsWith('HS') ? (
              <div className='flex items-center space-x-4'>
                <label className='text-lg font-medium text-gray-700 dark:text-gray-200 w-24 flex-shrink-0'>
                  Secret Key
                </label>
                <input
                  type='text'
                  className='flex-1 p-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-800 dark:text-white'
                  placeholder='输入 Secret Key'
                  value={secretKey}
                  onChange={(e) => setSecretKey(e.target.value)}
                />
              </div>
            ) : (
              <div>
                <label className='block text-lg font-medium text-gray-700 dark:text-gray-200 mb-2'>
                  RSA Private Key
                </label>
                <FileUpload
                  value={privateKey}
                  onChange={(value) => {
                    setPrivateKey(value)
                    // 验证私钥格式
                    if (value && !validateRSAPrivateKey(value)) {
                      setPrivateKeyError(
                        'RSA私钥格式无效，请检查或重新选择文件',
                      )
                    } else {
                      setPrivateKeyError(null)
                    }
                  }}
                  onError={setPrivateKeyError}
                  error={privateKeyError}
                  accept='.pem,.key'
                  fileType='text'
                  placeholder='请输入RSA私钥内容...'
                  className='w-full'
                />
              </div>
            )}
          </div>
        </div>

        {/* 生成JWT按钮 */}
        <div className='w-full flex-shrink-0'>
          <Button
            variant='primary'
            size='lg'
            onClick={handleGenerate}
            disabled={
              !!headerError ||
              !!payloadError ||
              (header.alg.startsWith('RS') &&
                (!!privateKeyError || !privateKey.trim()))
            }
            className='w-full'>
            生成JWT
          </Button>
        </div>

        {/* 结果显示区域 */}
        {output && (
          <div className='flex-shrink-0'>
            <div className='flex items-center justify-between mb-3'>
              <h3 className='text-lg font-semibold text-gray-800 dark:text-gray-200'>
                生成的JWT
              </h3>
              {!output.startsWith('生成出错') && (
                <Button
                  variant='primary'
                  size='sm'
                  onClick={handleCopyOutput}
                  className={copied ? 'bg-green-600 hover:bg-green-700' : ''}>
                  {copied ? '已复制' : '复制JWT'}
                </Button>
              )}
            </div>
            <textarea
              className={`w-full h-24 p-3 border rounded-md shadow-sm font-mono text-sm resize-none focus:outline-none ${
                output.startsWith('生成出错')
                  ? 'border-red-300 bg-red-50 text-red-700 dark:bg-red-900/20 dark:border-red-700 dark:text-red-400'
                  : 'border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-700 text-gray-900 dark:text-gray-100'
              }`}
              value={output}
              readOnly
            />
          </div>
        )}
      </div>
    </ToolLayout>
  )
}

export default JwtEncode
