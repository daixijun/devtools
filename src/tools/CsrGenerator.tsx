import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import React, { useState } from 'react'
import { Button, ErrorMessage, InputField } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'

interface GeneratedCsr {
  csrPem: string
  privateKeyPem: string
  publicKeyPem: string
  keyType: string
  keySize: number | null
  curveName: string | null
  subject: string
  sans: string[]
}

type KeyType = 'rsa' | 'ec'

const PEM_TEXTAREA_CLASSES =
  'w-full resize-none border border-slate-300 dark:border-slate-600 rounded-lg p-3 text-xs font-mono leading-relaxed bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 focus:outline-none transition-colors duration-200'

const SELECT_CLASSES =
  // h-[42px]: macOS WKWebView 的原生 select 会忽略垂直 padding 自行计算高度,
  // 显式定高与 InputField 输入框(px-3 py-2 + 16px 字号)等高, 保证 flex items-end 布局下标签对齐
  'h-[42px] px-3 pr-8 border border-slate-300 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent dark:bg-slate-700 dark:border-slate-600 dark:text-white disabled:opacity-50 transition-all duration-200 cursor-pointer'

/** 文件名安全化：保留字母数字与 . _ -，其余替换为下划线 */
const safeFileName = (name: string, fallback: string) => {
  const cleaned = name.trim().replace(/[^a-zA-Z0-9._-]/g, '_')
  return cleaned || fallback
}

/**
 * 输出卡片: 只读 PEM 文本域 + 复制 / 下载操作
 */
const OutputCard: React.FC<{
  title: string
  description: string
  text: string
  fileExtension: string
  baseFileName: string
  textareaClassName: string
}> = ({ title, description, text, fileExtension, baseFileName, textareaClassName }) => {
  const { copy, copied } = useCopyToClipboard()
  const [message, setMessage] = useState('')

  const handleDownload = async () => {
    setMessage('')
    try {
      const filePath = await save({
        filters: [
          {
            name: fileExtension === 'csr' ? 'CSR Files' : 'PEM Files',
            extensions: [fileExtension],
          },
        ],
        defaultPath: `${baseFileName}.${fileExtension}`,
      })
      if (!filePath) return
      await writeTextFile(filePath, text)
      setMessage('文件保存成功')
      setTimeout(() => setMessage(''), 2000)
    } catch (err) {
      console.error('保存文件失败:', err)
      setMessage(`保存文件失败: ${err instanceof Error ? err.message : String(err)}`)
    }
  }

  return (
    <section className='border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-800 p-4'>
      <div className='flex items-start justify-between gap-3 flex-wrap'>
        <div>
          <h3 className='text-sm font-semibold text-slate-800 dark:text-slate-100'>{title}</h3>
          <p className='mt-0.5 text-xs text-slate-500 dark:text-slate-400'>{description}</p>
        </div>
        <div className='flex items-center gap-2'>
          <Button variant={copied ? 'success' : 'secondary'} size='sm' onClick={() => copy(text)}>
            {copied ? '已复制 ✓' : '复制'}
          </Button>
          <Button variant='secondary' size='sm' onClick={() => void handleDownload()}>
            下载 .{fileExtension}
          </Button>
        </div>
      </div>

      <textarea readOnly value={text} className={`${PEM_TEXTAREA_CLASSES} ${textareaClassName} mt-3`} />

      {message && (
        <p
          className={`mt-2 text-xs ${message.includes('成功') ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'}`}>
          {message}
        </p>
      )}
    </section>
  )
}

const CsrGenerator: React.FC = () => {
  const [commonName, setCommonName] = useState('')
  const [country, setCountry] = useState('')
  const [state, setState] = useState('')
  const [locality, setLocality] = useState('')
  const [organization, setOrganization] = useState('')
  const [orgUnit, setOrgUnit] = useState('')
  const [email, setEmail] = useState('')
  const [keyType, setKeyType] = useState<KeyType>('rsa')
  const [rsaKeySize, setRsaKeySize] = useState(2048)
  const [ecCurve, setEcCurve] = useState('P-256')
  const [sanText, setSanText] = useState('')
  const [result, setResult] = useState<GeneratedCsr | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState('')

  const handleGenerate = async () => {
    if (!commonName.trim()) {
      setError('请填写通用名称（CN），通常为要申请证书的域名')
      return
    }

    setIsLoading(true)
    setError('')
    setResult(null)

    try {
      const sans = sanText
        .split('\n')
        .map((s) => s.trim())
        .filter(Boolean)

      const generated = await invoke<GeneratedCsr>('generate_csr', {
        request: {
          commonName,
          country: country.trim() || null,
          state: state.trim() || null,
          locality: locality.trim() || null,
          organization: organization.trim() || null,
          orgUnit: orgUnit.trim() || null,
          email: email.trim() || null,
          keyType,
          keySize: keyType === 'rsa' ? rsaKeySize : null,
          curve: keyType === 'ec' ? ecCurve : null,
          sans: sans.length ? sans : null,
        },
      })
      setResult(generated)
    } catch (err) {
      console.error('生成 CSR 失败:', err)
      setError(String(err))
    } finally {
      setIsLoading(false)
    }
  }

  const nameBase = safeFileName(commonName, 'certificate_request')

  return (
    <ToolLayout
      title='CSR 生成'
      subtitle='生成 PKCS#10 证书签名请求（CSR）与配套私钥，支持 RSA/ECC 密钥和 SAN 扩展'>
      <div className='flex flex-col h-full space-y-4 overflow-y-auto pb-4'>
        {/* 参数设置 */}
        <div className='border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-800 p-4 space-y-4'>
          <div className='flex flex-wrap items-end gap-4'>
            <InputField
              label='通用名称（CN，必填）'
              value={commonName}
              onChange={setCommonName}
              placeholder='如 example.com'
              className='w-64'
              disabled={isLoading}
              onKeyDown={(e) => {
                if (e.key === 'Enter') void handleGenerate()
              }}
            />

            <div>
              <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                密钥类型
              </label>
              <select
                value={keyType}
                disabled={isLoading}
                onChange={(e) => setKeyType(e.target.value as KeyType)}
                className={SELECT_CLASSES}>
                <option value='rsa'>RSA</option>
                <option value='ec'>ECC</option>
              </select>
            </div>

            {keyType === 'rsa' ? (
              <div>
                <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                  密钥长度
                </label>
                <select
                  value={rsaKeySize}
                  disabled={isLoading}
                  onChange={(e) => setRsaKeySize(Number(e.target.value))}
                  className={SELECT_CLASSES}>
                  <option value={2048}>2048 位（推荐）</option>
                  <option value={3072}>3072 位</option>
                  <option value={4096}>4096 位</option>
                </select>
              </div>
            ) : (
              <div>
                <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                  ECC 曲线
                </label>
                <select
                  value={ecCurve}
                  disabled={isLoading}
                  onChange={(e) => setEcCurve(e.target.value)}
                  className={SELECT_CLASSES}>
                  <option value='P-256'>P-256（prime256v1）</option>
                  <option value='P-384'>P-384（secp384r1）</option>
                  <option value='P-521'>P-521（secp521r1）</option>
                </select>
              </div>
            )}

            <Button variant='primary' loading={isLoading} onClick={() => void handleGenerate()}>
              {isLoading ? '生成中…' : '生成 CSR'}
            </Button>
          </div>

          <div className='flex flex-wrap items-end gap-3'>
            <InputField
              label='国家（C）'
              value={country}
              onChange={setCountry}
              placeholder='两位字母，如 CN'
              className='w-32'
              disabled={isLoading}
            />
            <InputField
              label='省份（ST）'
              value={state}
              onChange={setState}
              placeholder='如 Shanghai'
              className='w-40'
              disabled={isLoading}
            />
            <InputField
              label='城市（L）'
              value={locality}
              onChange={setLocality}
              placeholder='如 Shanghai'
              className='w-40'
              disabled={isLoading}
            />
            <InputField
              label='组织（O）'
              value={organization}
              onChange={setOrganization}
              placeholder='如 DevTools'
              className='w-44'
              disabled={isLoading}
            />
            <InputField
              label='部门（OU）'
              value={orgUnit}
              onChange={setOrgUnit}
              placeholder='如 Engineering'
              className='w-44'
              disabled={isLoading}
            />
            <InputField
              label='邮箱'
              value={email}
              onChange={setEmail}
              placeholder='如 admin@example.com'
              className='w-52'
              disabled={isLoading}
            />
          </div>

          <div>
            <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
              SAN（可选，每行一个域名或 IP，IP 自动识别）
            </label>
            <textarea
              value={sanText}
              onChange={(e) => setSanText(e.target.value)}
              disabled={isLoading}
              spellCheck={false}
              placeholder={'example.com\nwww.example.com\n10.0.0.1'}
              className={`${PEM_TEXTAREA_CLASSES} h-20`}
            />
          </div>
        </div>

        <ErrorMessage message={error || null} />

        {!result && !error && (
          <div className='text-center text-sm text-slate-500 dark:text-slate-400 py-10'>
            填写主题信息后点击「生成 CSR」，生成的私钥请妥善保管
          </div>
        )}

        {result && (
          <>
            {/* 结果信息 */}
            <section className='border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-800 p-4'>
              <h3 className='text-sm font-semibold text-slate-800 dark:text-slate-100 mb-3'>
                生成结果
              </h3>
              <div className='flex gap-3 py-1.5'>
                <span className='shrink-0 w-32 text-sm text-slate-500 dark:text-slate-400'>主题</span>
                <span className='flex-1 text-sm text-slate-900 dark:text-slate-100 font-mono break-all'>
                  {result.subject}
                </span>
              </div>
              <div className='flex gap-3 py-1.5'>
                <span className='shrink-0 w-32 text-sm text-slate-500 dark:text-slate-400'>
                  密钥
                </span>
                <span className='flex-1 text-sm text-slate-900 dark:text-slate-100 font-mono'>
                  {result.keyType}
                  {result.keySize != null && ` · ${result.keySize} 位`}
                  {result.curveName && ` · 曲线 ${result.curveName}`}
                </span>
              </div>
              {result.sans.length > 0 && (
                <div className='flex gap-3 py-1.5'>
                  <span className='shrink-0 w-32 text-sm text-slate-500 dark:text-slate-400'>
                    SAN
                  </span>
                  <span className='flex flex-wrap gap-1.5'>
                    {result.sans.map((san) => (
                      <span
                        key={san}
                        className='px-2 py-0.5 rounded text-xs font-mono bg-primary-50 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300'>
                        {san}
                      </span>
                    ))}
                  </span>
                </div>
              )}
            </section>

            <OutputCard
              title='证书签名请求（CSR）'
              description='提交给 CA 签发证书时使用，可安全分享'
              text={result.csrPem}
              fileExtension='csr'
              baseFileName={nameBase}
              textareaClassName='h-48'
            />

            <OutputCard
              title='私钥（PKCS#8）'
              description='泄露私钥将危及对应证书的安全，切勿提交到代码仓库或聊天工具'
              text={result.privateKeyPem}
              fileExtension='key'
              baseFileName={`${nameBase}_private_key`}
              textareaClassName='h-48'
            />

            <OutputCard
              title='公钥（SPKI）'
              description='可用于核对私钥与 CSR 是否匹配'
              text={result.publicKeyPem}
              fileExtension='pem'
              baseFileName={`${nameBase}_public_key`}
              textareaClassName='h-32'
            />
          </>
        )}
      </div>
    </ToolLayout>
  )
}

export default CsrGenerator
