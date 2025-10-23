import CryptoJS from 'crypto-js'
import React, { useCallback, useState } from 'react'
import { Button, InputField } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'

type AesMode = 'CBC' | 'ECB' | 'CFB' | 'OFB' | 'CTR'
type TabType = 'encrypt' | 'decrypt'

interface AesConfig {
  mode: AesMode
  keySize: 128 | 192 | 256
  padding:
    | 'Pkcs7'
    | 'Iso97971'
    | 'AnsiX923'
    | 'Iso10126'
    | 'ZeroPadding'
    | 'NoPadding'
}

const modeMap = {
  CBC: CryptoJS.mode.CBC,
  ECB: CryptoJS.mode.ECB,
  CFB: CryptoJS.mode.CFB,
  OFB: CryptoJS.mode.OFB,
  CTR: CryptoJS.mode.CTR,
}

const paddingMap = {
  Pkcs7: CryptoJS.pad.Pkcs7,
  Iso97971: CryptoJS.pad.Iso97971,
  AnsiX923: CryptoJS.pad.AnsiX923,
  Iso10126: CryptoJS.pad.Iso10126,
  ZeroPadding: CryptoJS.pad.ZeroPadding,
  NoPadding: CryptoJS.pad.NoPadding,
}

/**
 * AES 加密/解密工具
 * 支持多种 AES 模式和配置选项
 */
const AesCrypto: React.FC = () => {
  const [activeTab, setActiveTab] = useState<TabType>('encrypt')

  // 加密相关状态
  const [encryptText, setEncryptText] = useState('')
  const [encryptKey, setEncryptKey] = useState('')
  const [encryptIv, setEncryptIv] = useState('')
  const [encryptedText, setEncryptedText] = useState('')
  const [encryptError, setEncryptError] = useState('')

  // 解密相关状态
  const [decryptText, setDecryptText] = useState('')
  const [decryptKey, setDecryptKey] = useState('')
  const [decryptIv, setDecryptIv] = useState('')
  const [decryptedText, setDecryptedText] = useState('')
  const [decryptError, setDecryptError] = useState('')

  // AES 配置
  const [aesConfig, setAesConfig] = useState<AesConfig>({
    mode: 'CBC',
    keySize: 128,
    padding: 'Pkcs7',
  })

  const { copy, copied } = useCopyToClipboard()

  // 加密文本
  const encrypt = useCallback(() => {
    if (!encryptText) {
      setEncryptError('请输入要加密的文本')
      return
    }

    if (!encryptKey) {
      setEncryptError('请输入密钥')
      return
    }

    if (aesConfig.mode !== 'ECB' && !encryptIv) {
      setEncryptError('当前模式需要初始化向量(IV)')
      return
    }

    try {
      setEncryptError('')

      // 准备密钥
      const key = CryptoJS.enc.Utf8.parse(encryptKey)

      // 准备配置
      const config: any = {
        mode: modeMap[aesConfig.mode],
        padding: paddingMap[aesConfig.padding],
      }

      // 添加IV（除了ECB模式）
      if (aesConfig.mode !== 'ECB') {
        config.iv = CryptoJS.enc.Utf8.parse(encryptIv)
      }

      // 加密
      const encrypted = CryptoJS.AES.encrypt(encryptText, key, config)
      setEncryptedText(encrypted.toString())
    } catch (error) {
      setEncryptError(
        '加密过程中发生错误: ' +
          (error instanceof Error ? error.message : '未知错误'),
      )
    }
  }, [encryptText, encryptKey, encryptIv, aesConfig])

  // 解密文本
  const decrypt = useCallback(() => {
    if (!decryptText) {
      setDecryptError('请输入要解密的文本')
      return
    }

    if (!decryptKey) {
      setDecryptError('请输入密钥')
      return
    }

    if (aesConfig.mode !== 'ECB' && !decryptIv) {
      setDecryptError('当前模式需要初始化向量(IV)')
      return
    }

    try {
      setDecryptError('')

      // 准备密钥
      const key = CryptoJS.enc.Utf8.parse(decryptKey)

      // 准备配置
      const config: any = {
        mode: modeMap[aesConfig.mode],
        padding: paddingMap[aesConfig.padding],
      }

      // 添加IV（除了ECB模式）
      if (aesConfig.mode !== 'ECB') {
        config.iv = CryptoJS.enc.Utf8.parse(decryptIv)
      }

      // 解密
      const decrypted = CryptoJS.AES.decrypt(decryptText, key, config)
      const decryptedStr = decrypted.toString(CryptoJS.enc.Utf8)

      if (!decryptedStr) {
        setDecryptError('解密失败，请检查密钥、IV和密文是否正确')
        return
      }

      setDecryptedText(decryptedStr)
    } catch (error) {
      setDecryptError(
        '解密过程中发生错误: ' +
          (error instanceof Error ? error.message : '未知错误'),
      )
    }
  }, [decryptText, decryptKey, decryptIv, aesConfig])

  // 复制加密结果
  const copyEncryptedText = async () => {
    if (encryptedText) {
      await copy(encryptedText)
    }
  }

  // 复制解密结果
  const copyDecryptedText = async () => {
    if (decryptedText) {
      await copy(decryptedText)
    }
  }

  // 生成随机IV
  const generateRandomIv = (isForEncrypt: boolean) => {
    const iv = CryptoJS.lib.WordArray.random(16).toString()
    if (isForEncrypt) {
      setEncryptIv(iv)
    } else {
      setDecryptIv(iv)
    }
  }

  // 生成示例数据
  const generateExample = () => {
    setEncryptText('这是一段需要加密的示例文本')
    setEncryptKey('my-secret-key-16') // 16字节 = 128位
    setEncryptIv('initialization-ve') // 16字节 = 128位

    setDecryptText('U2FsdGVkX1+initialization-veexample-encrypted-text')
    setDecryptKey('my-secret-key-16')
    setDecryptIv('initialization-ve')
  }

  return (
    <ToolLayout
      title='AES 加密/解密工具'
      subtitle='使用 AES 算法对文本进行加密和解密，支持多种模式和配置'>
      {/* 标签页切换 */}
      <div className='flex border-b border-gray-200 dark:border-gray-700 mb-4'>
        <button
          className={`px-4 py-2 font-medium text-sm ${
            activeTab === 'encrypt'
              ? 'border-b-2 border-blue-500 text-blue-600 dark:text-blue-400'
              : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
          }`}
          onClick={() => setActiveTab('encrypt')}>
          加密
        </button>
        <button
          className={`px-4 py-2 font-medium text-sm ${
            activeTab === 'decrypt'
              ? 'border-b-2 border-blue-500 text-blue-600 dark:text-blue-400'
              : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
          }`}
          onClick={() => setActiveTab('decrypt')}>
          解密
        </button>
      </div>

      {/* AES 配置区域 */}
      <div className='bg-gray-50 dark:bg-gray-800 p-4 rounded-lg mb-4'>
        <h3 className='text-lg font-medium mb-3 text-gray-900 dark:text-gray-100'>
          AES 配置
        </h3>
        <div className='grid grid-cols-1 md:grid-cols-3 gap-4'>
          <div>
            <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
              模式
            </label>
            <select
              value={aesConfig.mode}
              onChange={(e) =>
                setAesConfig({ ...aesConfig, mode: e.target.value as AesMode })
              }
              className='w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white'>
              <option value='CBC'>CBC</option>
              <option value='ECB'>ECB</option>
              <option value='CFB'>CFB</option>
              <option value='OFB'>OFB</option>
              <option value='CTR'>CTR</option>
            </select>
          </div>
          <div>
            <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
              密钥长度
            </label>
            <select
              value={aesConfig.keySize}
              onChange={(e) =>
                setAesConfig({
                  ...aesConfig,
                  keySize: Number(e.target.value) as 128 | 192 | 256,
                })
              }
              className='w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white'>
              <option value={128}>128 位</option>
              <option value={192}>192 位</option>
              <option value={256}>256 位</option>
            </select>
          </div>
          <div>
            <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
              填充模式
            </label>
            <select
              value={aesConfig.padding}
              onChange={(e) =>
                setAesConfig({ ...aesConfig, padding: e.target.value as any })
              }
              className='w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white'>
              <option value='Pkcs7'>PKCS#7</option>
              <option value='Iso97971'>ISO/IEC 9797-1</option>
              <option value='AnsiX923'>ANSI X9.23</option>
              <option value='Iso10126'>ISO 10126</option>
              <option value='ZeroPadding'>Zero Padding</option>
              <option value='NoPadding'>No Padding</option>
            </select>
          </div>
        </div>
      </div>

      {/* 加密标签页内容 */}
      {activeTab === 'encrypt' && (
        <div className='space-y-4'>
          <div className='grid grid-cols-1 md:grid-cols-2 gap-4'>
            <div>
              <div className='mb-2'>
                <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
                  要加密的文本
                </label>
                <textarea
                  value={encryptText}
                  onChange={(e) => setEncryptText(e.target.value)}
                  placeholder='请输入要加密的文本'
                  className='w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white'
                  rows={4}
                />
              </div>
            </div>
            <div>
              <div className='mb-2'>
                <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
                  加密结果
                </label>
                <div className='relative'>
                  <textarea
                    value={encryptedText}
                    readOnly
                    placeholder='加密结果将显示在这里'
                    className='w-full px-3 py-2 pr-16 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm bg-gray-50 dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none'
                    rows={4}
                  />
                  {encryptedText && (
                    <button
                      onClick={copyEncryptedText}
                      className='absolute top-2 right-2 px-2 py-1 text-xs bg-blue-500 text-white rounded hover:bg-blue-600 focus:outline-none shadow-sm'>
                      {copied ? '已复制' : '复制'}
                    </button>
                  )}
                </div>
              </div>
            </div>
          </div>

          <div className='grid grid-cols-1 md:grid-cols-2 gap-4'>
            <div>
              <InputField
                label='密钥'
                value={encryptKey}
                onChange={setEncryptKey}
                placeholder={`请输入${aesConfig.keySize}位密钥 (${
                  aesConfig.keySize / 8
                } 字符)`}
              />
              <p className='text-xs text-gray-500 dark:text-gray-400 mt-1'>
                密钥长度必须为 {aesConfig.keySize / 8} 字符 ({aesConfig.keySize}{' '}
                位)
              </p>
            </div>
            {aesConfig.mode !== 'ECB' && (
              <div>
                <div className='mb-2'>
                  <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
                    初始化向量 (IV)
                  </label>
                  <div className='flex'>
                    <input
                      type='text'
                      value={encryptIv}
                      onChange={(e) => setEncryptIv(e.target.value)}
                      placeholder='请输入16字符的IV (128位)'
                      className='flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-l-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white'
                    />
                    <button
                      onClick={() => generateRandomIv(true)}
                      className='px-3 py-2 bg-gray-200 dark:bg-gray-600 text-gray-700 dark:text-gray-200 rounded-r-md hover:bg-gray-300 dark:hover:bg-gray-500 focus:outline-none'>
                      随机生成
                    </button>
                  </div>
                  <p className='text-xs text-gray-500 dark:text-gray-400 mt-1'>
                    IV长度必须为16字符 (128位)
                  </p>
                </div>
              </div>
            )}
          </div>

          {encryptError && (
            <div className='p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md'>
              <p className='text-red-700 dark:text-red-400 text-sm'>
                {encryptError}
              </p>
            </div>
          )}

          <div className='flex space-x-2'>
            <Button onClick={encrypt} disabled={!encryptText || !encryptKey}>
              加密
            </Button>
            <Button variant='secondary' onClick={generateExample}>
              加载示例
            </Button>
          </div>
        </div>
      )}

      {/* 解密标签页内容 */}
      {activeTab === 'decrypt' && (
        <div className='space-y-4'>
          <div className='grid grid-cols-1 md:grid-cols-2 gap-4'>
            <div>
              <div className='mb-2'>
                <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
                  要解密的文本
                </label>
                <textarea
                  value={decryptText}
                  onChange={(e) => setDecryptText(e.target.value)}
                  placeholder='请输入要解密的文本'
                  className='w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white'
                  rows={4}
                />
              </div>
            </div>
            <div>
              <div className='mb-2'>
                <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
                  解密结果
                </label>
                <div className='relative'>
                  <textarea
                    value={decryptedText}
                    readOnly
                    placeholder='解密结果将显示在这里'
                    className='w-full px-3 py-2 pr-16 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm bg-gray-50 dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none'
                    rows={4}
                  />
                  {decryptedText && (
                    <button
                      onClick={copyDecryptedText}
                      className='absolute top-2 right-2 px-2 py-1 text-xs bg-blue-500 text-white rounded hover:bg-blue-600 focus:outline-none shadow-sm'>
                      {copied ? '已复制' : '复制'}
                    </button>
                  )}
                </div>
              </div>
            </div>
          </div>

          <div className='grid grid-cols-1 md:grid-cols-2 gap-4'>
            <div>
              <InputField
                label='密钥'
                value={decryptKey}
                onChange={setDecryptKey}
                placeholder={`请输入${aesConfig.keySize}位密钥 (${
                  aesConfig.keySize / 8
                } 字符)`}
              />
              <p className='text-xs text-gray-500 dark:text-gray-400 mt-1'>
                密钥长度必须为 {aesConfig.keySize / 8} 字符 ({aesConfig.keySize}{' '}
                位)
              </p>
            </div>
            {aesConfig.mode !== 'ECB' && (
              <div>
                <div className='mb-2'>
                  <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
                    初始化向量 (IV)
                  </label>
                  <div className='flex'>
                    <input
                      type='text'
                      value={decryptIv}
                      onChange={(e) => setDecryptIv(e.target.value)}
                      placeholder='请输入16字符的IV (128位)'
                      className='flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-l-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white'
                    />
                    <button
                      onClick={() => generateRandomIv(false)}
                      className='px-3 py-2 bg-gray-200 dark:bg-gray-600 text-gray-700 dark:text-gray-200 rounded-r-md hover:bg-gray-300 dark:hover:bg-gray-500 focus:outline-none'>
                      随机生成
                    </button>
                  </div>
                  <p className='text-xs text-gray-500 dark:text-gray-400 mt-1'>
                    IV长度必须为16字符 (128位)
                  </p>
                </div>
              </div>
            )}
          </div>

          {decryptError && (
            <div className='p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md'>
              <p className='text-red-700 dark:text-red-400 text-sm'>
                {decryptError}
              </p>
            </div>
          )}

          <div className='flex space-x-2'>
            <Button onClick={decrypt} disabled={!decryptText || !decryptKey}>
              解密
            </Button>
            <Button variant='secondary' onClick={generateExample}>
              加载示例
            </Button>
          </div>
        </div>
      )}
    </ToolLayout>
  )
}

export default AesCrypto
