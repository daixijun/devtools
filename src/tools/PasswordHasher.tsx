import bcrypt from 'bcryptjs'
import React, { useCallback, useState } from 'react'
import { Button } from '../components/common'
import { ToolLayout } from '../components/layouts'
import { useCopyToClipboard } from '../hooks'

/**
 * 密码加密与验证工具
 * 支持 bcrypt 等常用加密算法
 */
const PasswordHasher: React.FC = () => {
  const [password, setPassword] = useState('')
  const [hash, setHash] = useState('')
  const [verifyPassword, setVerifyPassword] = useState('')
  const [verifyResult, setVerifyResult] = useState<{
    isValid: boolean | null
    message: string
  }>({
    isValid: null,
    message: '',
  })
  const [saltRounds, setSaltRounds] = useState(10)
  const [isHashing, setIsHashing] = useState(false)
  const [isVerifying, setIsVerifying] = useState(false)
  const { copy, copied } = useCopyToClipboard()

  // 加密密码
  const hashPassword = useCallback(async () => {
    if (!password) {
      setHash('请输入密码')
      return
    }

    if (saltRounds < 4 || saltRounds > 20) {
      setHash('盐值轮数必须在 4-20 之间')
      return
    }

    try {
      setIsHashing(true)
      const salt = await bcrypt.genSalt(saltRounds)
      const hashed = await bcrypt.hash(password, salt)
      setHash(hashed)
      setVerifyResult({ isValid: null, message: '' })
    } catch (error) {
      setHash(
        '加密过程中发生错误: ' +
          (error instanceof Error ? error.message : '未知错误'),
      )
    } finally {
      setIsHashing(false)
    }
  }, [password, saltRounds])

  // 验证密码
  const verifyPasswordHash = useCallback(async () => {
    if (!verifyPassword) {
      setVerifyResult({ isValid: null, message: '请输入要验证的密码' })
      return
    }

    if (!hash) {
      setVerifyResult({ isValid: null, message: '请先生成密码哈希值' })
      return
    }

    try {
      setIsVerifying(true)
      const isValid = await bcrypt.compare(verifyPassword, hash)
      setVerifyResult({
        isValid,
        message: isValid ? '密码验证成功！' : '密码验证失败！',
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
  }, [verifyPassword, hash])

  // 复制哈希值
  const handleCopyHash = async () => {
    if (
      hash &&
      !hash.startsWith('请输入密码') &&
      !hash.startsWith('盐值轮数必须在') &&
      !hash.startsWith('加密过程中发生错误')
    ) {
      await copy(hash)
    }
  }

  // 重置所有状态
  const handleReset = () => {
    setPassword('')
    setHash('')
    setVerifyPassword('')
    setVerifyResult({ isValid: null, message: '' })
    setSaltRounds(10)
  }

  return (
    <ToolLayout
      title='密码加密与验证工具'
      subtitle='使用 bcrypt 算法对密码进行加密和验证'
      description='该工具使用行业标准的 bcrypt 加密算法来保护密码安全，支持自定义盐值轮数以平衡安全性和性能。'>
      <div className='flex flex-col h-full space-y-6 overflow-y-auto'>
        {/* 密码加密区域 */}
        <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-600 p-4'>
          <h3 className='text-lg font-medium text-gray-900 dark:text-white mb-4'>
            密码加密
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

            {/* 盐值轮数设置 */}
            <div>
              <label className='block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1'>
                盐值轮数: {saltRounds}
              </label>
              <input
                type='range'
                min='4'
                max='20'
                value={saltRounds}
                onChange={(e) => setSaltRounds(parseInt(e.target.value))}
                className='w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer dark:bg-gray-700'
              />
              <div className='flex justify-between text-xs text-gray-500 dark:text-gray-400 mt-1'>
                <span>4 (较快)</span>
                <span>20 (较慢但更安全)</span>
              </div>
              <p className='text-xs text-gray-500 dark:text-gray-400 mt-2'>
                盐值轮数越高，安全性越好，但加密时间越长。推荐值为 10-12。
              </p>
            </div>

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
                onClick={handleReset}
                className='flex-1'>
                重置
              </Button>
            </div>

            {/* 哈希结果显示 */}
            {hash && (
              <div className='mt-4'>
                <div className='flex items-center justify-between mb-1'>
                  <label className='block text-sm font-medium text-gray-700 dark:text-gray-300'>
                    加密结果 (哈希值)
                  </label>
                  <Button
                    variant='primary'
                    size='sm'
                    onClick={handleCopyHash}
                    disabled={
                      !hash ||
                      hash.startsWith('请输入密码') ||
                      hash.startsWith('盐值轮数必须在') ||
                      hash.startsWith('加密过程中发生错误')
                    }
                    className={copied ? 'bg-green-600 hover:bg-green-700' : ''}>
                    {copied ? '已复制' : '复制'}
                  </Button>
                </div>
                <div className='bg-gray-50 dark:bg-gray-700 rounded p-3 font-mono text-sm break-all'>
                  {hash}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* 密码验证区域 */}
        <div className='bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-600 p-4'>
          <h3 className='text-lg font-medium text-gray-900 dark:text-white mb-4'>
            密码验证
          </h3>

          <div className='space-y-4'>
            {/* 验证密码输入 */}
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

            {/* 验证按钮 */}
            <Button
              variant='primary'
              onClick={verifyPasswordHash}
              disabled={isVerifying}
              className='w-full'>
              {isVerifying ? '验证中...' : '验证密码'}
            </Button>

            {/* 验证结果 */}
            {verifyResult.isValid !== null && (
              <div
                className={`p-3 rounded-md ${
                  verifyResult.isValid
                    ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                    : 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200'
                }`}>
                <div className='flex items-center'>
                  <span className='mr-2'>
                    {verifyResult.isValid ? '✅' : '❌'}
                  </span>
                  <span>{verifyResult.message}</span>
                </div>
              </div>
            )}

            {verifyResult.isValid === null && verifyResult.message && (
              <div className='p-3 rounded-md bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'>
                <div className='flex items-center'>
                  <span className='mr-2'>ℹ️</span>
                  <span>{verifyResult.message}</span>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* bcrypt 说明 */}
        <div className='bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800 p-4'>
          <h4 className='text-sm font-medium text-blue-800 dark:text-blue-200 mb-2'>
            bcrypt 算法说明
          </h4>
          <ul className='text-xs text-blue-700 dark:text-blue-300 space-y-1'>
            <li>
              • bcrypt 是目前最广泛使用的密码哈希算法之一，具有良好的安全性
            </li>
            <li>• 通过加盐（salt）防止彩虹表攻击</li>
            <li>• 通过调整轮数（rounds）来平衡安全性和性能</li>
            <li>• 每次加密即使相同密码也会产生不同的哈希值（因为随机盐值）</li>
            <li>• 验证时通过提取盐值来比对密码</li>
          </ul>
        </div>
      </div>
    </ToolLayout>
  )
}

export default PasswordHasher
