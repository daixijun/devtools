import { invoke } from '@tauri-apps/api/core'
import React, { useCallback, useEffect, useState } from 'react'
import Button from '../components/common/Button'
import ErrorMessage from '../components/common/ErrorMessage'
import InputField from '../components/common/InputField'
import { ToolLayout } from '../components/layouts'
import { useDebounce } from '../hooks/useDebounce'
import { useToast } from '../hooks/useToast'

interface RegexFlags {
  case_insensitive: boolean
  multiline: boolean
  dot_matches_new_line: boolean
  swap_greed: boolean
  unicode: boolean
}

interface RegexMatch {
  full_match: string
  start: number
  end: number
  groups: (string | null)[]
  named_groups: Record<string, string | null>
}

interface RegexTestResult {
  is_valid: boolean
  error_message?: string
  matches: RegexMatch[]
  match_count: number
}

interface RegexReplaceResult {
  is_valid: boolean
  error_message?: string
  result?: string
  replacement_count: number
}

export const RegexTester: React.FC = () => {
  const [pattern, setPattern] = useState('')
  const [testText, setTestText] = useState('')
  const [replacement, setReplacement] = useState('')
  const [engine, setEngine] = useState<
    'rust' | 're2' | 'pcre' | 'golang' | 'javascript'
  >('rust')
  const [mode, setMode] = useState<'test' | 'replace'>('test')
  const [replaceAll, setReplaceAll] = useState(true)
  const [flags, setFlags] = useState<RegexFlags>({
    case_insensitive: false,
    multiline: false,
    dot_matches_new_line: false,
    swap_greed: false,
    unicode: true,
  })

  const [testResult, setTestResult] = useState<RegexTestResult | null>(null)
  const [replaceResult, setReplaceResult] = useState<RegexReplaceResult | null>(
    null,
  )
  const [isValidPattern, setIsValidPattern] = useState<boolean | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string>('')
  const [highlightedText, setHighlightedText] = useState<string>('')

  const { showToast } = useToast()
  const debouncedPattern = useDebounce(pattern, 300)

  // 更新高亮文本
  useEffect(() => {
    if (
      mode === 'test' &&
      testResult &&
      testResult.is_valid &&
      testResult.matches.length > 0
    ) {
      setHighlightedText(highlightMatches(testText, testResult.matches))
    } else {
      setHighlightedText('')
    }
  }, [testText, testResult, mode])

  // 验证正则表达式
  useEffect(() => {
    const validatePattern = () => {
      if (!debouncedPattern) {
        setIsValidPattern(null)
        return
      }

      invoke<boolean>('validate_regex', {
        pattern: debouncedPattern,
        engine: engine,
      })
        .then((response) => {
          setIsValidPattern(response)
          setError('')
        })
        .catch((err) => {
          console.error('验证正则表达式失败:', err)
          setIsValidPattern(false)
          setError('验证正则表达式失败')
        })
    }

    validatePattern()
  }, [debouncedPattern, engine])

  const handleTest = useCallback(() => {
    if (!pattern || !testText) {
      showToast('请输入正则表达式和测试文本')
      return
    }

    setIsLoading(true)
    setError('')

    console.log('发送测试请求:', {
      pattern,
      text: testText,
      flags,
      engine,
    })

    invoke<RegexTestResult>('test_regex', {
      pattern: pattern,
      text: testText,
      flags: flags,
      engine: engine,
    })
      .then((response) => {
        console.log('收到响应:', response)
        setTestResult(response)
        if (!response.is_valid && response.error_message) {
          setError(response.error_message)
        }
      })
      .catch((err) => {
        console.error('正则表达式测试失败:', err)
        setError(`正则表达式测试失败: ${err}`)
        setTestResult(null)
      })
      .finally(() => {
        setIsLoading(false)
      })
  }, [pattern, testText, flags, engine, showToast])

  const handleReplace = useCallback(() => {
    if (!pattern || !testText) {
      showToast('请输入正则表达式和测试文本')
      return
    }

    setIsLoading(true)
    setError('')

    invoke<RegexReplaceResult>('replace_regex', {
      pattern: pattern,
      text: testText,
      replacement: replacement || '',
      flags: flags,
      engine: engine,
      replace_all: replaceAll,
    })
      .then((response) => {
        setReplaceResult(response)
        if (!response.is_valid && response.error_message) {
          setError(response.error_message)
        }
      })
      .catch((err) => {
        console.error('正则表达式替换失败:', err)
        setError('正则表达式替换失败')
        setReplaceResult(null)
      })
      .finally(() => {
        setIsLoading(false)
      })
  }, [pattern, testText, replacement, flags, engine, replaceAll, showToast])

  const handleFlagChange = (flag: keyof RegexFlags) => {
    setFlags((prev) => ({ ...prev, [flag]: !prev[flag] }))
  }

  const highlightMatches = (text: string, matches: RegexMatch[]) => {
    if (!matches.length) return text

    let highlighted = ''
    let lastEnd = 0

    matches.forEach((match, index) => {
      highlighted += text.slice(lastEnd, match.start)
      highlighted += `<mark class="bg-yellow-200 dark:bg-yellow-800" title="匹配 ${
        index + 1
      }: ${match.start}-${match.end}">${match.full_match}</mark>`
      lastEnd = match.end
    })

    highlighted += text.slice(lastEnd)
    return highlighted
  }

  const renderTestResult = () => {
    if (!testResult) return null

    if (!testResult.is_valid) {
      return (
        <div className='p-4 border rounded-lg bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800'>
          <p className='text-red-600 dark:text-red-400 font-medium'>
            正则表达式无效
          </p>
          {testResult.error_message && (
            <p className='text-red-500 dark:text-red-300 text-sm mt-1'>
              {testResult.error_message}
            </p>
          )}
        </div>
      )
    }

    return (
      <div className='space-y-4 h-full flex flex-col'>
        {testResult.matches.length > 0 && (
          <div className='space-y-2 h-full flex flex-col'>
            <h4 className='font-medium text-gray-700 dark:text-gray-300'>
              匹配详情:
            </h4>
            <div className='h-full overflow-y-auto border rounded-lg bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-600'>
              {testResult.matches.map((match, index) => (
                <div
                  key={index}
                  className='p-3 border-b last:border-b-0 border-gray-100 dark:border-gray-700'>
                  <div className='grid grid-cols-2 gap-2 text-sm'>
                    <div>
                      <span className='font-medium text-gray-600 dark:text-gray-400'>
                        匹配 {index + 1}:
                      </span>
                      <span className='ml-2 font-mono text-gray-900 dark:text-gray-100'>
                        {match.full_match}
                      </span>
                    </div>
                    <div>
                      <span className='font-medium text-gray-600 dark:text-gray-400'>
                        位置:
                      </span>
                      <span className='ml-2 text-gray-900 dark:text-gray-100'>
                        {match.start}-{match.end}
                      </span>
                    </div>
                  </div>

                  {match.groups.length > 0 && (
                    <div className='mt-2'>
                      <span className='font-medium text-gray-600 dark:text-gray-400'>
                        捕获组:
                      </span>
                      <div className='ml-4 space-y-1'>
                        {match.groups.map((group, groupIndex) => (
                          <div key={groupIndex} className='text-sm'>
                            <span className='text-gray-500 dark:text-gray-400'>
                              组 {groupIndex + 1}:
                            </span>
                            <span className='ml-2 font-mono text-gray-900 dark:text-gray-100'>
                              {group || '(未匹配)'}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}

                  {Object.keys(match.named_groups).length > 0 && (
                    <div className='mt-2'>
                      <span className='font-medium text-gray-600 dark:text-gray-400'>
                        命名组:
                      </span>
                      <div className='ml-4 space-y-1'>
                        {Object.entries(match.named_groups).map(
                          ([name, value]) => (
                            <div key={name} className='text-sm'>
                              <span className='text-gray-500 dark:text-gray-400'>
                                {name}:
                              </span>
                              <span className='ml-2 font-mono text-gray-900 dark:text-gray-100'>
                                {value || '(未匹配)'}
                              </span>
                            </div>
                          ),
                        )}
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        {testResult.matches.length === 0 && (
          <div className='p-4 border rounded-lg bg-gray-50 dark:bg-gray-800 border-gray-200 dark:border-gray-600'>
            <p className='text-gray-600 dark:text-gray-400 text-center'>
              没有找到匹配项
            </p>
          </div>
        )}
      </div>
    )
  }

  const renderReplaceResult = () => {
    if (!replaceResult) return null

    if (!replaceResult.is_valid) {
      return (
        <div className='p-4 border rounded-lg bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800'>
          <p className='text-red-600 dark:text-red-400 font-medium'>
            正则表达式无效
          </p>
          {replaceResult.error_message && (
            <p className='text-red-500 dark:text-red-300 text-sm mt-1'>
              {replaceResult.error_message}
            </p>
          )}
        </div>
      )
    }

    return (
      <div className='space-y-4'>
        <div className='flex items-center gap-4 p-3 bg-green-50 dark:bg-green-900/20 rounded-lg'>
          <span className='text-green-600 dark:text-green-400 font-medium'>
            完成 {replaceResult.replacement_count} 个替换
          </span>
        </div>

        {replaceResult.result && (
          <div className='space-y-2'>
            <h4 className='font-medium text-gray-700 dark:text-gray-300'>
              替换结果:
            </h4>
            <div className='p-3 border rounded-lg bg-gray-50 dark:bg-gray-800 border-gray-300 dark:border-gray-600 font-mono text-sm whitespace-pre-wrap text-gray-900 dark:text-gray-100'>
              {replaceResult.result}
            </div>
          </div>
        )}
      </div>
    )
  }

  const leftContent = (
    <div className='space-y-6 h-full flex flex-col'>
      {/* 配置与控制区域 */}
      <div className='flex-shrink-0'>
        <h3 className='text-lg font-medium text-gray-700 dark:text-gray-300 mb-4'>
          配置与控制
        </h3>
        <div className='space-y-4 p-4 border rounded-lg bg-gray-50 dark:bg-gray-800 border-gray-200 dark:border-gray-600'>
          {/* 引擎选择 */}
          <div className='space-y-2'>
            <label className='block text-sm font-medium text-gray-700 dark:text-gray-300'>
              正则引擎
            </label>
            <div className='grid grid-cols-3 gap-2'>
              <label className='flex items-center text-gray-700 dark:text-gray-300'>
                <input
                  type='radio'
                  value='rust'
                  checked={engine === 'rust'}
                  onChange={(e) => setEngine(e.target.value as typeof engine)}
                  className='mr-2'
                />
                Rust
              </label>
              <label className='flex items-center text-gray-700 dark:text-gray-300'>
                <input
                  type='radio'
                  value='re2'
                  checked={engine === 're2'}
                  onChange={(e) => setEngine(e.target.value as typeof engine)}
                  className='mr-2'
                />
                RE2
              </label>
              <label className='flex items-center text-gray-700 dark:text-gray-300'>
                <input
                  type='radio'
                  value='pcre'
                  checked={engine === 'pcre'}
                  onChange={(e) => setEngine(e.target.value as typeof engine)}
                  className='mr-2'
                />
                PCRE
              </label>
              <label className='flex items-center text-gray-700 dark:text-gray-300'>
                <input
                  type='radio'
                  value='golang'
                  checked={engine === 'golang'}
                  onChange={(e) => setEngine(e.target.value as typeof engine)}
                  className='mr-2'
                />
                Golang
              </label>
              <label className='flex items-center text-gray-700 dark:text-gray-300'>
                <input
                  type='radio'
                  value='javascript'
                  checked={engine === 'javascript'}
                  onChange={(e) => setEngine(e.target.value as typeof engine)}
                  className='mr-2'
                />
                JavaScript
              </label>
            </div>
          </div>

          {/* 模式选择 */}
          <div className='space-y-2'>
            <label className='block text-sm font-medium text-gray-700 dark:text-gray-300'>
              操作模式
            </label>
            <div className='flex gap-4'>
              <label className='flex items-center text-gray-700 dark:text-gray-300'>
                <input
                  type='radio'
                  value='test'
                  checked={mode === 'test'}
                  onChange={(e) =>
                    setMode(e.target.value as 'test' | 'replace')
                  }
                  className='mr-2'
                />
                测试匹配
              </label>
              <label className='flex items-center text-gray-700 dark:text-gray-300'>
                <input
                  type='radio'
                  value='replace'
                  checked={mode === 'replace'}
                  onChange={(e) =>
                    setMode(e.target.value as 'test' | 'replace')
                  }
                  className='mr-2'
                />
                替换文本
              </label>
            </div>
          </div>

          {/* 正则表达式输入 */}
          <div className='space-y-2'>
            <div className='flex items-center justify-between'>
              <label className='block text-sm font-medium text-gray-700 dark:text-gray-300'>
                正则表达式
                {isValidPattern === true && (
                  <span className='ml-2 text-green-600 text-xs'>
                    ✓ 语法正确
                  </span>
                )}
                {isValidPattern === false && (
                  <span className='ml-2 text-red-600 text-xs'>✗ 语法错误</span>
                )}
              </label>
              {/* 匹配计数显示 */}
              {mode === 'test' && testResult && testResult.is_valid && (
                <span className='text-sm text-blue-600 dark:text-blue-400 font-medium'>
                  找到 {testResult.match_count} 个匹配
                </span>
              )}
            </div>
            <InputField
              value={pattern}
              onChange={setPattern}
              placeholder='输入正则表达式...'
              className={`font-mono ${
                isValidPattern === false
                  ? 'border-red-300 dark:border-red-600'
                  : ''
              }`}
            />
          </div>

          {/* 替换文本输入 */}
          {mode === 'replace' && (
            <div className='space-y-2'>
              <label className='block text-sm font-medium text-gray-700 dark:text-gray-300'>
                替换文本
              </label>
              <InputField
                value={replacement}
                onChange={setReplacement}
                placeholder='输入替换文本...'
                className='font-mono'
              />
              <label className='flex items-center text-gray-700 dark:text-gray-300'>
                <input
                  type='checkbox'
                  checked={replaceAll}
                  onChange={(e) => setReplaceAll(e.target.checked)}
                  className='mr-2'
                />
                替换所有匹配
              </label>
            </div>
          )}

          {/* 正则标志 */}
          <div className='space-y-2'>
            <label className='block text-sm font-medium text-gray-700 dark:text-gray-300'>
              正则标志
            </label>
            <div className='grid grid-cols-2 gap-2'>
              <label className='flex items-center text-sm text-gray-700 dark:text-gray-300'>
                <input
                  type='checkbox'
                  checked={flags.case_insensitive}
                  onChange={() => handleFlagChange('case_insensitive')}
                  className='mr-2'
                />
                忽略大小写 (i)
              </label>
              <label className='flex items-center text-sm text-gray-700 dark:text-gray-300'>
                <input
                  type='checkbox'
                  checked={flags.multiline}
                  onChange={() => handleFlagChange('multiline')}
                  className='mr-2'
                />
                多行模式 (m)
              </label>
              <label className='flex items-center text-sm text-gray-700 dark:text-gray-300'>
                <input
                  type='checkbox'
                  checked={flags.dot_matches_new_line}
                  onChange={() => handleFlagChange('dot_matches_new_line')}
                  className='mr-2'
                />
                . 匹配换行 (s)
              </label>
              <label className='flex items-center text-sm text-gray-700 dark:text-gray-300'>
                <input
                  type='checkbox'
                  checked={flags.swap_greed}
                  onChange={() => handleFlagChange('swap_greed')}
                  className='mr-2'
                />
                非贪婪模式 (U)
              </label>
              <label className='flex items-center text-sm text-gray-700 dark:text-gray-300'>
                <input
                  type='checkbox'
                  checked={flags.unicode}
                  onChange={() => handleFlagChange('unicode')}
                  className='mr-2'
                />
                Unicode 支持
              </label>
            </div>
          </div>

          {/* 操作按钮 */}
          <div className='flex gap-2'>
            {mode === 'test' ? (
              <Button
                onClick={handleTest}
                disabled={!pattern || !testText || isLoading}
                className='flex-1'>
                {isLoading ? '测试中...' : '测试匹配'}
              </Button>
            ) : (
              <Button
                onClick={handleReplace}
                disabled={!pattern || !testText || isLoading}
                className='flex-1'>
                {isLoading ? '替换中...' : '执行替换'}
              </Button>
            )}
          </div>

          {/* 错误信息 */}
          {error && <ErrorMessage message={error} />}
        </div>
      </div>

      {/* 测试文本输入区域 */}
      <div className='flex-1 flex flex-col min-h-0'>
        <h3 className='text-lg font-medium text-gray-700 dark:text-gray-300 mb-4'>
          测试文本
        </h3>
        <div className='flex-1 relative'>
          {/* 高亮层 - 显示在输入框后面 */}
          {highlightedText && (
            <div
              className='absolute inset-0 p-3 border border-transparent rounded-lg font-mono text-sm whitespace-pre-wrap break-words pointer-events-none z-10 text-transparent overflow-hidden'
              style={{
                background: 'transparent',
                color: 'transparent',
                caretColor: 'transparent',
              }}
              dangerouslySetInnerHTML={{ __html: highlightedText }}
            />
          )}
          {/* 输入框 - 背景透明以显示高亮 */}
          <textarea
            value={testText}
            onChange={(e) => setTestText(e.target.value)}
            placeholder='输入要测试的文本...'
            className={`w-full h-full p-3 border rounded-lg font-mono text-sm border-gray-300 dark:border-gray-600 placeholder-gray-500 dark:placeholder-gray-400 focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-none relative z-20 ${
              highlightedText
                ? 'bg-transparent text-gray-900 dark:text-gray-100'
                : 'bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100'
            }`}
            style={{
              backgroundColor: highlightedText ? 'transparent' : undefined,
            }}
          />
        </div>
      </div>
    </div>
  )

  const rightContent = (
    <div className='h-full flex flex-col'>
      <h3 className='text-lg font-medium text-gray-700 dark:text-gray-300 mb-4'>
        {mode === 'test' ? '匹配结果' : '替换结果'}
      </h3>
      <div className='flex-1 border rounded-lg p-4 bg-white dark:bg-gray-900 border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-100 overflow-auto'>
        {mode === 'test' ? renderTestResult() : renderReplaceResult()}
      </div>
    </div>
  )

  return (
    <ToolLayout
      title='正则表达式测试器'
      subtitle='测试正则表达式匹配和替换，支持 Rust、RE2、PCRE、Golang、JavaScript 语法'>
      <div className='grid grid-cols-1 xl:grid-cols-2 gap-6 h-full'>
        {/* 左侧：配置控制区 + 测试文本区 */}
        <div className='min-h-0'>{leftContent}</div>

        {/* 右侧：匹配结果区 */}
        <div className='min-h-0 h-full flex flex-col'>{rightContent}</div>
      </div>
    </ToolLayout>
  )
}
