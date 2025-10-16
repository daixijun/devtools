import bcrypt from 'bcryptjs'
import CryptoJS from 'crypto-js'
import React, { useCallback, useState } from 'react'
import { Button } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'

type HashAlgorithm = 'bcrypt' | 'md5' | 'sha256' | 'sha512' | 'pbkdf2'

interface AlgorithmInfo {
  name: string
  description: string
  secure: boolean
  useCase: string
}

const algorithmInfoMap: Record<HashAlgorithm, AlgorithmInfo> = {
  bcrypt: {
    name: 'bcrypt',
    description: '业界标准的密码哈希算法，内置盐值和可调节的计算成本',
    secure: true,
    useCase: '推荐用于密码存储',
  },
  pbkdf2: {
    name: 'PBKDF2',
    description: '基于密码的密钥派生函数，使用 HMAC-SHA256',
    secure: true,
    useCase: '推荐用于密码存储和密钥派生',
  },
  sha256: {
    name: 'SHA-256',
    description: '256位安全哈希算法，单向哈希函数',
    secure: false,
    useCase: '仅用于数据完整性校验，不推荐用于密码存储',
  },
  sha512: {
    name: 'SHA-512',
    description: '512位安全哈希算法，比SHA-256更长的输出',
    secure: false,
    useCase: '仅用于数据完整性校验，不推荐用于密码存储',
  },
  md5: {
    name: 'MD5',
    description: '已被破解的哈希算法，仅供兼容性使用',
    secure: false,
    useCase: '不安全，仅用于非安全场景的数据校验',
  },
}

/**
 * 密码加密与验证工具
 * 支持多种常见加密算法
 */
const PasswordHasher: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'hash' | 'verify'>('hash')

  // 加密相关状态
  const [password, setPassword] = useState('')
  const [algorithm, setAlgorithm] = useState<HashAlgorithm>('bcrypt')
  const [hash, setHash] = useState('')
  const [saltRounds, setSaltRounds] = useState(10)
  const [iterations, setIterations] = useState(10000)
  const [isHashing, setIsHashing] = useState(false)

  // 验证相关状态
  const [verifyPassword, setVerifyPassword] = useState('')
  const [verifyHash, setVerifyHash] = useState('')
  const [verifyAlgorithm, setVerifyAlgorithm] = useState<HashAlgorithm>('bcrypt')
  const [verifyResult, setVerifyResult] = useState<{
    isValid: boolean | null
    message: string
  }>({
    isValid: null,
    message: '',
  })
  const [isVerifying, setIsVerifying] = useState(false)

  const { copy, copied } = useCopyToClipboard()

  // 加密密码
  const hashPassword = useCallback(async () => {
    if (!password) {
      setHash('请输入密码')
      return
    }

    try {
      setIsHashing(true)
      let hashed: string

      switch (algorithm) {
        case 'bcrypt':
          if (saltRounds < 4 || saltRounds > 20) {
            setHash('盐值轮数必须在 4-20 之间')
            return
          }
          const salt = await bcrypt.genSalt(saltRounds)
          hashed = await bcrypt.hash(password, salt)
          break

        case 'md5':
          hashed = CryptoJS.MD5(password).toString()
          break

        case 'sha256':
          hashed = CryptoJS.SHA256(password).toString()
          break

        case 'sha512':
          hashed = CryptoJS.SHA512(password).toString()
          break

        case 'pbkdf2':
          if (iterations < 1000 || iterations > 1000000) {
            setHash('迭代次数必须在 1000-1000000 之间')
            return
          }
          const pbkdf2Hash = CryptoJS.PBKDF2(password, 'salt', {
            keySize: 256 / 32,
            iterations: iterations,
          })
          hashed = pbkdf2Hash.toString()
          break

        default:
          hashed = 'Unsupported algorithm'
      }

      setHash(hashed)
    } catch (error) {
      setHash(
        '加密过程中发生错误: ' +
          (error instanceof Error ? error.message : '未知错误'),
      )
    } finally {
      setIsHashing(false)
    }
  }, [password, algorithm, saltRounds, iterations])

  // 验证密码
  const verifyPasswordHash = useCallback(async () => {
    if (!verifyPassword) {
      setVerifyResult({ isValid: null, message: '请输入要验证的密码' })
      return
    }

    if (!verifyHash) {
      setVerifyResult({ isValid: null, message: '请输入哈希值' })
      return
    }

    try {
      setIsVerifying(true)
      let isValid = false

      switch (verifyAlgorithm) {
        case 'bcrypt':
          isValid = await bcrypt.compare(verifyPassword, verifyHash)
          break

        case 'md5':
          isValid = CryptoJS.MD5(verifyPassword).toString() === verifyHash
          break

        case 'sha256':
          isValid = CryptoJS.SHA256(verifyPassword).toString() === verifyHash
          break

        case 'sha512':
          isValid = CryptoJS.SHA512(verifyPassword).toString() === verifyHash
          break

        case 'pbkdf2':
          // PBKDF2 需要相同的盐值和迭代次数，这里简化处理
          const pbkdf2Hash = CryptoJS.PBKDF2(verifyPassword, 'salt', {
            keySize: 256 / 32,
            iterations: iterations,
          })
          isValid = pbkdf2Hash.toString() === verifyHash
          break

        default:
          setVerifyResult({
            isValid: false,
            message: '不支持的算法',
          })
          return
      }

      setVerifyResult({
        isValid,
        message: isValid ? '✅ 密码验证成功！' : '❌ 密码验证失败！',
      })
    } catch (error) {
      setVerifyResult({
        isValid: false,
        message:
          '验证过程中发生错误: ' +
          (error instanceof Error ? error.message : '未知错误'),
      })
    } finally {
      setIsVerifying(false)
    }
  }, [verifyPassword, verifyHash, verifyAlgorithm, iterations])

  // 复制哈希值
  const handleCopyHash = async () => {
    if (
      hash &&
      !hash.startsWith('请输入密码') &&
      !hash.startsWith('盐值轮数必须在') &&
      !hash.startsWith('迭代次数必须在') &&
      !hash.startsWith('加密过程中发生错误')
    ) {
      await copy(hash)
    }
  }

  // 重置加密状态
  const handleResetHash = () => {
    setPassword('')
    setHash('')
    setAlgorithm('bcrypt')
    setSaltRounds(10)
    setIterations(10000)
  }

  // 重置验证状态
  const handleResetVerify = () => {
    setVerifyPassword('')
    setVerifyHash('')
    setVerifyAlgorithm('bcrypt')
    setVerifyResult({ isValid: null, message: '' })
  }

  return (
    <ToolLayout
      title='密码加密与验证工具'
      subtitle='支持多种加密算法的密码哈希与验证'
      description='提供 bcrypt、PBKDF2、SHA-256、SHA-512、MD5 等多种加密算法，支持密码加密和验证功能。'>
      <div className='flex flex-col h-full overflow-hidden'>
        {/* Tab 导航 */}
        <div className='flex border-b border-gray-200 dark:border-gray-600 mb-6'>
          <button
            className={`px-6 py-3 font-medium text-sm transition-colors ${
              activeTab === 'hash'
                ? 'text-blue-600 dark:text-blue-400 border-b-2 border-blue-600 dark:border-blue-400'
                : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
            }`}
            onClick={() => setActiveTab('hash')}>
            密码加密
          </button>
          <button
            className={`px-6 py-3 font-medium text-sm transition-colors ${
              activeTab === 'verify'
                ? 'text-blue-600 dark:text-blue-400 border-b-2 border-blue-600 dark:border-blue-400'
                : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
            }`}
            onClick={() => setActiveTab('verify')}>
            密码验证
          </button>
        </div>

        {/* Tab 内容 */}
        <div className='flex-1 overflow-y-auto'>
          {activeTab === 'hash' && (
            <div className='space-y-6'>
              {/* 算法选择 */}
              <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-600 p-4'>
                <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-3'>
                  选择加密算法
                </label>
                <div className='grid grid-cols-2 md:grid-cols-3 gap-3'>
                  {(
                    Object.entries(algorithmInfoMap) as [
                      HashAlgorithm,
                      AlgorithmInfo,
                    ][]
                  ).map(([alg, info]) => (
                    <button
                      key={alg}
                      onClick={() => setAlgorithm(alg)}
                      className={`p-3 rounded-lg border-2 transition-all text-left ${
                        algorithm === alg
                          ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                          : 'border-gray-200 dark:border-gray-600 hover:border-gray-300 dark:hover:border-gray-500'
                      }`}>
                      <div className='flex items-center justify-between mb-1'>
                        <span className='font-medium text-gray-900 dark:text-white'>
                          {info.name}
                        </span>
                        {info.secure ? (
                          <span className='text-xs px-2 py-0.5 bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300 rounded'>
                            安全
                          </span>
                        ) : (
                          <span className='text-xs px-2 py-0.5 bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300 rounded'>
                            不安全
                          </span>
                        )}
                      </div>
                      <p className='text-xs text-gray-600 dark:text-gray-400'>
                        {info.useCase}
                      </p>
                    </button>
                  ))}
                </div>
              </div>

              {/* 密码输入和参数设置 */}
              <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-600 p-4'>
                <h3 className='text-lg font-medium text-gray-900 dark:text-white mb-4'>
                  加密参数
                </h3>

                <div className='space-y-4'>
                  {/* 密码输入 */}
                  <div>
                    <label
                      htmlFor='password'
                      className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      输入密码
                    </label>
                    <input
                      type='password'
                      id='password'
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                      placeholder='请输入要加密的密码...'
                      className='w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500'
                    />
                  </div>

                  {/* bcrypt 和 PBKDF2 的强度参数 */}
                  {(algorithm === 'bcrypt' || algorithm === 'pbkdf2') && (
                    <div>
                      <label
                        htmlFor='strengthParam'
                        className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
                        {algorithm === 'bcrypt'
                          ? '盐值轮数 (Salt Rounds)'
                          : '迭代次数 (Iterations)'}
                      </label>
                      <input
                        type='number'
                        id='strengthParam'
                        min={algorithm === 'bcrypt' ? '4' : '1000'}
                        max={algorithm === 'bcrypt' ? '20' : '1000000'}
                        step={algorithm === 'bcrypt' ? '1' : '1000'}
                        value={algorithm === 'bcrypt' ? saltRounds : iterations}
                        onChange={(e) => {
                          const value = parseInt(e.target.value)
                          if (!isNaN(value)) {
                            if (algorithm === 'bcrypt') {
                              setSaltRounds(value)
                            } else {
                              setIterations(value)
                            }
                          }
                        }}
                        placeholder={
                          algorithm === 'bcrypt'
                            ? '输入盐值轮数 (4-20)'
                            : '输入迭代次数 (1000-1000000)'
                        }
                        className='w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500'
                      />
                      <p className='text-xs text-gray-500 dark:text-gray-400 mt-2'>
                        {algorithm === 'bcrypt'
                          ? '盐值轮数越高，安全性越好，但加密时间越长。推荐值为 10-12。范围：4 - 20'
                          : '迭代次数越多，安全性越好，但加密时间越长。推荐值为 10,000 以上。范围：1,000 - 1,000,000'}
                      </p>
                    </div>
                  )}

                  {/* 加密按钮 */}
                  <div className='flex space-x-3'>
                    <Button
                      variant='primary'
                      onClick={hashPassword}
                      disabled={isHashing}
                      className='flex-1'>
                      {isHashing ? '加密中...' : '加密密码'}
                    </Button>
                    <Button
                      variant='secondary'
                      onClick={handleResetHash}
                      className='flex-1'>
                      重置
                    </Button>
                  </div>
                </div>
              </div>

              {/* 加密结果 */}
              {hash && (
                <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-600 p-4'>
                  <div className='flex items-center justify-between mb-2'>
                    <h3 className='text-lg font-medium text-gray-900 dark:text-white'>
                      加密结果
                    </h3>
                    <Button
                      variant='primary'
                      size='sm'
                      onClick={handleCopyHash}
                      disabled={
                        !hash ||
                        hash.startsWith('请输入密码') ||
                        hash.startsWith('盐值轮数必须在') ||
                        hash.startsWith('迭代次数必须在') ||
                        hash.startsWith('加密过程中发生错误')
                      }
                      className={copied ? 'bg-green-600 hover:bg-green-700' : ''}>
                      {copied ? '已复制' : '复制'}
                    </Button>
                  </div>
                  <div className='bg-gray-50 dark:bg-gray-700 rounded-lg p-4 font-mono text-sm break-all text-gray-900 dark:text-gray-100'>
                    {hash}
                  </div>
                  <div className='mt-3 text-xs text-gray-500 dark:text-gray-400'>
                    算法: {algorithmInfoMap[algorithm].name} | 哈希长度:{' '}
                    {hash.length} 字符
                  </div>
                </div>
              )}

              {/* 算法说明 */}
              <div className='bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800 p-4'>
                <h4 className='text-sm font-medium text-blue-800 dark:text-blue-200 mb-2'>
                  当前算法: {algorithmInfoMap[algorithm].name}
                </h4>
                <p className='text-xs text-blue-700 dark:text-blue-300 mb-2'>
                  {algorithmInfoMap[algorithm].description}
                </p>
                <p className='text-xs text-blue-600 dark:text-blue-400'>
                  <strong>使用场景:</strong>{' '}
                  {algorithmInfoMap[algorithm].useCase}
                </p>
              </div>
            </div>
          )}

          {activeTab === 'verify' && (
            <div className='space-y-6'>
              {/* 算法选择 */}
              <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-600 p-4'>
                <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-3'>
                  选择加密算法
                </label>
                <div className='grid grid-cols-2 md:grid-cols-3 gap-3'>
                  {(
                    Object.entries(algorithmInfoMap) as [
                      HashAlgorithm,
                      AlgorithmInfo,
                    ][]
                  ).map(([alg, info]) => (
                    <button
                      key={alg}
                      onClick={() => setVerifyAlgorithm(alg)}
                      className={`p-3 rounded-lg border-2 transition-all text-left ${
                        verifyAlgorithm === alg
                          ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                          : 'border-gray-200 dark:border-gray-600 hover:border-gray-300 dark:hover:border-gray-500'
                      }`}>
                      <div className='flex items-center justify-between mb-1'>
                        <span className='font-medium text-gray-900 dark:text-white'>
                          {info.name}
                        </span>
                        {info.secure ? (
                          <span className='text-xs px-2 py-0.5 bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300 rounded'>
                            安全
                          </span>
                        ) : (
                          <span className='text-xs px-2 py-0.5 bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300 rounded'>
                            不安全
                          </span>
                        )}
                      </div>
                      <p className='text-xs text-gray-600 dark:text-gray-400'>
                        {info.useCase}
                      </p>
                    </button>
                  ))}
                </div>
              </div>

              {/* 验证输入 */}
              <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-600 p-4'>
                <h3 className='text-lg font-medium text-gray-900 dark:text-white mb-4'>
                  验证参数
                </h3>

                <div className='space-y-4'>
                  {/* 密码输入 */}
                  <div>
                    <label
                      htmlFor='verifyPassword'
                      className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      输入要验证的密码
                    </label>
                    <input
                      type='password'
                      id='verifyPassword'
                      value={verifyPassword}
                      onChange={(e) => setVerifyPassword(e.target.value)}
                      placeholder='请输入要验证的密码...'
                      className='w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500'
                    />
                  </div>

                  {/* 哈希值输入 */}
                  <div>
                    <label
                      htmlFor='verifyHash'
                      className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
                      输入哈希值
                    </label>
                    <textarea
                      id='verifyHash'
                      value={verifyHash}
                      onChange={(e) => setVerifyHash(e.target.value)}
                      placeholder='请输入要验证的哈希值...'
                      rows={3}
                      className='w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm'
                    />
                  </div>

                  {/* bcrypt 和 PBKDF2 的强度参数 */}
                  {(verifyAlgorithm === 'bcrypt' ||
                    verifyAlgorithm === 'pbkdf2') && (
                    <div>
                      <label
                        htmlFor='verifyStrengthParam'
                        className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
                        {verifyAlgorithm === 'bcrypt'
                          ? '盐值轮数 (Salt Rounds)'
                          : '迭代次数 (Iterations)'}
                      </label>
                      {verifyAlgorithm === 'bcrypt' ? (
                        <div className='bg-gray-50 dark:bg-gray-700 rounded-lg p-3 border border-gray-200 dark:border-gray-600'>
                          <p className='text-sm text-gray-600 dark:text-gray-400'>
                            bcrypt 的盐值轮数已包含在哈希值中，验证时无需手动设置
                          </p>
                        </div>
                      ) : (
                        <>
                          <input
                            type='number'
                            id='verifyStrengthParam'
                            min='1000'
                            max='1000000'
                            step='1000'
                            value={iterations}
                            onChange={(e) => {
                              const value = parseInt(e.target.value)
                              if (!isNaN(value)) {
                                setIterations(value)
                              }
                            }}
                            placeholder='输入迭代次数 (1000-1000000)'
                            className='w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500'
                          />
                          <p className='text-xs text-gray-500 dark:text-gray-400 mt-2'>
                            验证时需要使用与加密时相同的迭代次数。范围：1,000 -
                            1,000,000
                          </p>
                        </>
                      )}
                    </div>
                  )}

                  {/* 验证按钮 */}
                  <div className='flex space-x-3'>
                    <Button
                      variant='primary'
                      onClick={verifyPasswordHash}
                      disabled={isVerifying}
                      className='flex-1'>
                      {isVerifying ? '验证中...' : '验证密码'}
                    </Button>
                    <Button
                      variant='secondary'
                      onClick={handleResetVerify}
                      className='flex-1'>
                      重置
                    </Button>
                  </div>
                </div>
              </div>

              {/* 验证结果 */}
              {(verifyResult.isValid !== null || verifyResult.message) && (
                <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-600 p-4'>
                  <h3 className='text-lg font-medium text-gray-900 dark:text-white mb-3'>
                    验证结果
                  </h3>
                  <div
                    className={`p-4 rounded-lg ${
                      verifyResult.isValid === true
                        ? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-200'
                        : verifyResult.isValid === false
                          ? 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-200'
                          : 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-200'
                    }`}>
                    <div className='text-base font-medium'>
                      {verifyResult.message}
                    </div>
                  </div>
                </div>
              )}

              {/* 安全提示 */}
              <div className='bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-200 dark:border-yellow-800 p-4'>
                <h4 className='text-sm font-medium text-yellow-800 dark:text-yellow-200 mb-2 flex items-center'>
                  <span className='mr-2'>⚠️</span>
                  安全提示
                </h4>
                <ul className='text-xs text-yellow-700 dark:text-yellow-300 space-y-1'>
                  <li>• 不同的哈希值需要使用对应的算法进行验证</li>
                  <li>• bcrypt 每次加密会产生不同的哈希值（内置随机盐值）</li>
                  <li>
                    • MD5 和 SHA 系列算法不推荐用于密码存储，仅用于数据完整性校验
                  </li>
                  <li>• 推荐使用 bcrypt 或 PBKDF2 算法进行密码存储</li>
                </ul>
              </div>
            </div>
          )}
        </div>
      </div>
    </ToolLayout>
  )
}

export default PasswordHasher
