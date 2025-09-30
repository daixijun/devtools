import React, { useEffect, useState } from 'react'
import { ToolLayout } from '../components/layouts'

const TimestampConverter: React.FC = () => {
  const [inputValue, setInputValue] = useState('')
  const [convertedValue, setConvertedValue] = useState('')
  const [conversionType, setConversionType] = useState<
    'timestampToDatetime' | 'datetimeToTimestamp'
  >('timestampToDatetime')
  const [isMilliseconds, setIsMilliseconds] = useState(true)
  const [currentTime, setCurrentTime] = useState({
    seconds: 0,
    milliseconds: 0,
  })
  const [timeZone, setTimeZone] = useState('Asia/Shanghai')

  // 获取所有支持的时区列表
  // @ts-ignore: Intl.supportedValuesOf 在某些环境可能不存在
  const allTimeZones = Intl.supportedValuesOf('timeZone').map((tz) => ({
    value: tz,
    label: tz.replace(/_/g, ' '),
  }))

  // 更新当前时间
  useEffect(() => {
    const updateCurrentTime = () => {
      const now = Date.now()
      setCurrentTime({
        seconds: Math.floor(now / 1000),
        milliseconds: now,
      })
    }

    updateCurrentTime()
    const interval = setInterval(updateCurrentTime, 1000)
    return () => clearInterval(interval)
  }, [])

  // 处理转换
  const handleConvert = () => {
    if (!inputValue) {
      setConvertedValue('请输入值')
      return
    }

    if (conversionType === 'timestampToDatetime') {
      // 时间戳转日期时间
      const timestamp = parseInt(inputValue)
      if (isNaN(timestamp)) {
        setConvertedValue('请输入有效数字')
        return
      }

      // 根据单位转换为毫秒时间戳
      const millisecondTimestamp = isMilliseconds ? timestamp : timestamp * 1000

      // 检查时间戳是否合理
      if (millisecondTimestamp < 0 || millisecondTimestamp > 9999999999999) {
        setConvertedValue('时间戳超出合理范围')
        return
      }

      const date = new Date(millisecondTimestamp)
      setConvertedValue(date.toLocaleString('zh-CN', { timeZone: timeZone }))
    } else {
      // 日期时间转时间戳
      const date = new Date(inputValue)
      if (isNaN(date.getTime())) {
        setConvertedValue('请输入有效的日期时间格式')
        return
      }

      const timestamp = isMilliseconds
        ? date.getTime()
        : Math.floor(date.getTime() / 1000)
      setConvertedValue(timestamp.toString())
    }
  }

  // 使用当前时间
  const useCurrentTime = () => {
    if (conversionType === 'timestampToDatetime') {
      setInputValue(
        isMilliseconds
          ? currentTime.milliseconds.toString()
          : currentTime.seconds.toString(),
      )
    } else {
      // 修复：使用ISO格式的日期时间字符串而不是本地化字符串
      setInputValue(new Date().toISOString())
    }
    setTimeout(handleConvert, 0)
  }

  return (
    <ToolLayout
      title='时间戳与日期时间转换'
      subtitle='时间戳与日期时间格式之间的双向转换，支持多时区和毫秒级精度'>
      <div className='flex flex-col h-full'>
        <div className='flex-1 overflow-auto p-4'>
          <div className='mb-4'>
            <div className='mb-4'>
              <label className='block text-gray-700 dark:text-gray-200 mb-2'>
                当前时间
              </label>
              <div className='p-3 bg-gray-100 dark:bg-gray-700 rounded-md'>
                <p className='text-gray-700 dark:text-gray-200'>
                  时间戳（秒）: {currentTime.seconds}
                </p>
                <p className='text-gray-700 dark:text-gray-200'>
                  时间戳（毫秒）: {currentTime.milliseconds}
                </p>
                <p className='text-gray-700 dark:text-gray-200'>
                  当前时间:{' '}
                  {new Date(currentTime.milliseconds).toLocaleString('zh-CN', {
                    timeZone: timeZone,
                  })}
                </p>
              </div>
            </div>

            {/* 水平表单结构 */}
            <div className='grid grid-cols-1 md:grid-cols-1 gap-4 mb-4'>
              <div className='flex-1'>
                <div className='flex items-center gap-2 mb-2'>
                  <label className='text-gray-700 dark:text-gray-200 whitespace-nowrap w-24'>
                    输入
                  </label>
                  <div className='flex-1'>
                    <div className='flex'>
                      <input
                        type='text'
                        className='w-full p-1 border border-gray-300 rounded-l-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
                        placeholder='输入时间戳或日期时间'
                        value={inputValue}
                        onChange={(e) => setInputValue(e.target.value)}
                      />
                      <button
                        className='min-w-[80px] px-1 py-1 bg-blue-500 text-white rounded-r-md hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2'
                        onClick={useCurrentTime}>
                        当前时间
                      </button>
                    </div>
                  </div>
                </div>

                <div className='flex-1'>
                  <div className='flex items-center gap-2 mb-2'>
                    <label className='text-gray-700 dark:text-gray-200 whitespace-nowrap w-24'>
                      转换类型
                    </label>
                    <div className='flex-1'>
                      <select
                        className='w-full p-4 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
                        value={conversionType}
                        onChange={(e) =>
                          setConversionType(
                            e.target.value as
                              | 'timestampToDatetime'
                              | 'datetimeToTimestamp',
                          )
                        }>
                        <option value='timestampToDatetime'>
                          时间戳 → 日期时间
                        </option>
                        <option value='datetimeToTimestamp'>
                          日期时间 → 时间戳
                        </option>
                      </select>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div className='mt-2'>
              <div className='flex items-center gap-2 mb-2'>
                <label className='text-gray-700 dark:text-gray-200 whitespace-nowrap w-24'>
                  时区选择
                </label>
                <div className='flex-1'>
                  <select
                    className='w-full p-4 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
                    value={timeZone}
                    onChange={(e) => setTimeZone(e.target.value)}>
                    {allTimeZones.map(
                      (tz: { value: string; label: string }) => (
                        <option key={tz.value} value={tz.value}>
                          {tz.label}
                        </option>
                      ),
                    )}
                  </select>
                </div>
              </div>
            </div>

            {conversionType === 'timestampToDatetime' && (
              <div className='mb-4 flex items-center mt-4'>
                <label className='block text-gray-700 dark:text-gray-200 mr-2 w-24'>
                  毫秒单位
                </label>
                <div className='relative inline-block mr-2 align-middle select-none'>
                  <input
                    type='checkbox'
                    id='toggle-milliseconds'
                    className='toggle-checkbox'
                    checked={isMilliseconds}
                    onChange={(e) => setIsMilliseconds(e.target.checked)}
                  />
                  <label
                    htmlFor='toggle-milliseconds'
                    className='toggle-label'></label>
                </div>
                <span className='text-gray-700 dark:text-gray-200'>
                  {isMilliseconds ? '开启' : '关闭'}
                </span>
              </div>
            )}

            <button
              className='px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2'
              onClick={handleConvert}>
              转换
            </button>

            <div className='mt-4'>
              <div className='flex items-center gap-2 mb-2'>
                <label className='text-gray-700 dark:text-gray-200 whitespace-nowrap w-24'>
                  转换结果
                </label>
                <div className='flex-1'>
                  <input
                    className='w-full p-1 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white'
                    placeholder='转换结果'
                    value={convertedValue}
                    readOnly
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </ToolLayout>
  )
}

export default TimestampConverter
