import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import React, { useState } from 'react'
import { Button, ErrorMessage, InputField } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'

interface RsaKeyPairResult {
  privatePkcs8Pem: string
  privatePkcs1Pem: string
  publicSpkiPem: string
  publicPkcs1Pem: string
  publicOpenssh: string
}

type PrivateFormat = 'pkcs8' | 'pkcs1'
type PublicFormat = 'pem' | 'pkcs1' | 'openssh'

interface KeyFormatTab {
  id: PrivateFormat | PublicFormat
  label: string
  text: string
  /** 该格式建议的下载文件扩展名 */
  extension: string
}

const KEY_TEXTAREA_CLASSES =
  'w-full resize-none border border-slate-300 dark:border-slate-600 rounded-lg p-3 text-xs font-mono leading-relaxed bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 focus:outline-none transition-colors duration-200'

/**
 * 密钥结果卡片: 格式切换 Tab + 只读文本域 + 复制/下载操作
 */
const KeyOutputCard: React.FC<{
  title: string
  description: string
  tabs: KeyFormatTab[]
  activeId: string
  onActiveChange: (id: string) => void
  baseFileName: string
  textareaClassName: string
}> = ({ title, description, tabs, activeId, onActiveChange, baseFileName, textareaClassName }) => {
  const { copy, copied } = useCopyToClipboard()
  const [message, setMessage] = useState('')

  const active = tabs.find((tab) => tab.id === activeId) ?? tabs[0]

  const handleDownload = async () => {
    setMessage('')
    try {
      const filePath = await save({
        filters: [
          {
            name: active.extension === 'pub' ? 'Public Key Files' : 'PEM Files',
            extensions: [active.extension],
          },
        ],
        defaultPath: `${baseFileName}_${active.id}.${active.extension}`,
      })
      if (!filePath) return
      await writeTextFile(filePath, active.text)
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
          <div className='flex rounded-lg overflow-hidden border border-slate-300 dark:border-slate-600'>
            {tabs.map((tab) => (
              <button
                key={tab.id}
                type='button'
                onClick={() => onActiveChange(tab.id)}
                className={`px-3 py-1.5 text-xs font-medium transition-colors duration-200 ${
                  tab.id === active.id
                    ? 'bg-primary-600 text-white'
                    : 'bg-white dark:bg-slate-700 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-600'
                }`}>
                {tab.label}
              </button>
            ))}
          </div>
          <Button variant={copied ? 'success' : 'secondary'} size='sm' onClick={() => copy(active.text)}>
            {copied ? '已复制 ✓' : '复制'}
          </Button>
          <Button variant='secondary' size='sm' onClick={handleDownload}>
            下载 .{active.extension}
          </Button>
        </div>
      </div>

      <textarea readOnly value={active.text} className={`${KEY_TEXTAREA_CLASSES} ${textareaClassName} mt-3`} />

      {message && (
        <p
          className={`mt-2 text-xs ${message.includes('成功') ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'}`}>
          {message}
        </p>
      )}
    </section>
  )
}

const RsaKeyGenerator: React.FC = () => {
  const [keySize, setKeySize] = useState(2048)
  const [comment, setComment] = useState('')
  const [result, setResult] = useState<RsaKeyPairResult | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState('')
  const [privateFormat, setPrivateFormat] = useState<PrivateFormat>('pkcs8')
  const [publicFormat, setPublicFormat] = useState<PublicFormat>('pem')

  const handleGenerate = async () => {
    setIsLoading(true)
    setError('')
    setResult(null)

    try {
      const keyPair = await invoke<RsaKeyPairResult>('generate_rsa_keypair', {
        request: { keySize, comment: comment.trim() || null },
      })
      setResult(keyPair)
    } catch (err) {
      console.error('生成 RSA 密钥对失败:', err)
      setError(String(err))
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <ToolLayout
      title='RSA 密钥对生成'
      subtitle='生成 RSA 公私钥对，支持 PKCS#8 / PKCS#1 私钥与 SPKI / PKCS#1 / OpenSSH 公钥格式'>
      <div className='flex flex-col h-full space-y-4 overflow-y-auto pb-4'>
        {/* 参数设置 */}
        <div className='border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-800 p-4'>
          <div className='flex flex-wrap items-end gap-4'>
            <div>
              <label className='block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2'>
                密钥长度
              </label>
              <select
                value={keySize}
                disabled={isLoading}
                onChange={(e) => setKeySize(Number(e.target.value))}
                className='h-[42px] px-3 pr-8 border border-slate-300 rounded-lg shadow-sm focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent dark:bg-slate-700 dark:border-slate-600 dark:text-white disabled:opacity-50 transition-all duration-200 cursor-pointer'>
                <option value={2048}>2048 位（推荐）</option>
                <option value={3072}>3072 位</option>
                <option value={4096}>4096 位</option>
              </select>
            </div>

            <InputField
              label='OpenSSH 注释（可选）'
              value={comment}
              onChange={setComment}
              placeholder='如 user@host，留空使用默认值'
              className='w-72'
              disabled={isLoading}
              onKeyDown={(e) => {
                if (e.key === 'Enter') void handleGenerate()
              }}
            />

            <Button variant='primary' loading={isLoading} onClick={() => void handleGenerate()}>
              {isLoading ? '生成中…' : '生成密钥对'}
            </Button>
          </div>
        </div>

        <ErrorMessage message={error || null} />

        {!result && !error && (
          <div className='text-center text-sm text-slate-500 dark:text-slate-400 py-10'>
            选择密钥长度后点击「生成密钥对」，生成的私钥请妥善保管
          </div>
        )}

        {result && (
          <>
            <KeyOutputCard
              title='私钥'
              description='泄露私钥将危及所有依赖它的系统，切勿提交到代码仓库或聊天工具'
              baseFileName='rsa_private_key'
              tabs={[
                { id: 'pkcs8', label: 'PKCS#8', text: result.privatePkcs8Pem, extension: 'pem' },
                { id: 'pkcs1', label: 'PKCS#1', text: result.privatePkcs1Pem, extension: 'pem' },
              ]}
              activeId={privateFormat}
              onActiveChange={(id) => setPrivateFormat(id as PrivateFormat)}
              textareaClassName='h-64'
            />

            <KeyOutputCard
              title='公钥'
              description='OpenSSH 格式可直接粘贴到服务器的 authorized_keys 中'
              baseFileName='rsa_public_key'
              tabs={[
                { id: 'pem', label: 'PEM', text: result.publicSpkiPem, extension: 'pem' },
                { id: 'pkcs1', label: 'PKCS#1', text: result.publicPkcs1Pem, extension: 'pem' },
                { id: 'openssh', label: 'OpenSSH', text: result.publicOpenssh, extension: 'pub' },
              ]}
              activeId={publicFormat}
              onActiveChange={(id) => setPublicFormat(id as PublicFormat)}
              textareaClassName='h-40'
            />
          </>
        )}
      </div>
    </ToolLayout>
  )
}

export default RsaKeyGenerator
