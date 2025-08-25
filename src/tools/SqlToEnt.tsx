import { CodeEditor } from '../components/common'
import { invoke } from '@tauri-apps/api/core'
import React, { useState, useEffect } from 'react'
import Split from 'react-split'
import { ToolLayout } from '../components/layouts'

const SqlToEnt: React.FC = () => {

  const [input, setInput] = useState(`CREATE TABLE users (
    id INT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    age INT,
    active BOOLEAN DEFAULT TRUE
);

CREATE TABLE posts (
    id INT PRIMARY KEY AUTO_INCREMENT,
    title VARCHAR(255) NOT NULL,
    content TEXT,
    user_id INT,
    published BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);`)
  const [outputs, setOutputs] = useState<{[key: string]: string}>({})
  const [activeTab, setActiveTab] = useState(0)
  const [tableNames, setTableNames] = useState<string[]>([])
  const [generateEdges, setGenerateEdges] = useState(true)
  const [generateMixin, setGenerateMixin] = useState(true)
  const [generateHooks, setGenerateHooks] = useState(false)
  const [generatePolicy, setGeneratePolicy] = useState(false)
  const [useUUIDPrimaryKey, setUseUUIDPrimaryKey] = useState(false)
  const [enableSoftDelete, setEnableSoftDelete] = useState(false)
  const [enablePluralization, setEnablePluralization] = useState(true)
  const [packageName, setPackageName] = useState('schema')
  const [error, setError] = useState('')

  // Auto-conversion effect
  useEffect(() => {
    const timer = setTimeout(() => {
      if (input.trim()) {
        convertSqlToEnt()
      }
    }, 500) // 500ms delay to avoid excessive processing
    return () => clearTimeout(timer)
  }, [
    input,
    generateEdges,
    generateMixin,
    generateHooks,
    generatePolicy,
    useUUIDPrimaryKey,
    enableSoftDelete,
    enablePluralization,
    packageName,
  ])

  const convertSqlToEnt = async () => {
    if (!input.trim()) {
      setOutputs({})
      setTableNames([])
      setError('')
      return
    }

    try {
      setError('')
      const options = {
        generate_edges: generateEdges,
        generate_mixin: generateMixin,
        generate_hooks: generateHooks,
        generate_policy: generatePolicy,
        use_uuid_primary_key: useUUIDPrimaryKey,
        enable_soft_delete: enableSoftDelete,
        enable_pluralization: enablePluralization,
        package_name: packageName,
      }

      // Call Rust backend
      const result: any = await invoke('convert_sql_to_ent', {
        sql: input,
        options,
      })

      // Handle response
      if (result && typeof result === 'object') {
        if (result.success) {
          const data = result.data
          setOutputs(data.outputs || {})
          setTableNames(data.table_names || [])
          setActiveTab(0)
        } else {
          setError(result.message || '转换失败')
          setOutputs({})
          setTableNames([])
        }
      }
    } catch (err) {
      console.error('Error in convertSqlToEnt:', err)
      setError(err instanceof Error ? err.message : '转换失败')
      setOutputs({})
      setTableNames([])
    }
  }

  const clearAll = () => {
    setInput('')
    setOutputs({})
    setTableNames([])
    setActiveTab(0)
    setError('')
  }

  const loadExample = () => {
    setInput(`CREATE TABLE users (
    id INT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    age INT,
    active BOOLEAN DEFAULT TRUE
);

CREATE TABLE posts (
    id INT PRIMARY KEY AUTO_INCREMENT,
    title VARCHAR(255) NOT NULL,
    content TEXT,
    user_id INT,
    published BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);`)
  }

  return (
    <ToolLayout
      title="SQL 转 Go Ent ORM"
      subtitle="将SQL表结构转换为Go Ent ORM Schema，支持多表解析和关系生成"
      padding={false}
    >
      <div className='flex flex-col h-full'>
        {/* Configuration Panel - Fixed height */}
        <div className='flex-shrink-0 p-4 border-b dark:border-gray-600 bg-white dark:bg-gray-800'>

          {/* Configuration Options */}
          <div className='grid grid-cols-2 lg:grid-cols-4 gap-4 mb-4'>
            <label className='flex items-center space-x-2'>
              <input
                type='checkbox'
                checked={enablePluralization}
                onChange={(e) => setEnablePluralization(e.target.checked)}
                className='w-4 h-4'
              />
              <span className='text-gray-700 dark:text-gray-300'>复数表名</span>
            </label>

            <label className='flex items-center space-x-2'>
              <input
                type='checkbox'
                checked={generateEdges}
                onChange={(e) => setGenerateEdges(e.target.checked)}
                className='w-4 h-4'
              />
              <span className='text-gray-700 dark:text-gray-300'>生成边缘关系</span>
            </label>

            <label className='flex items-center space-x-2'>
              <input
                type='checkbox'
                checked={generateMixin}
                onChange={(e) => setGenerateMixin(e.target.checked)}
                className='w-4 h-4'
              />
              <span className='text-gray-700 dark:text-gray-300'>生成 Mixin</span>
            </label>

            <label className='flex items-center space-x-2'>
              <input
                type='checkbox'
                checked={generateHooks}
                onChange={(e) => setGenerateHooks(e.target.checked)}
                className='w-4 h-4'
              />
              <span className='text-gray-700 dark:text-gray-300'>生成 Hooks</span>
            </label>

            <label className='flex items-center space-x-2'>
              <input
                type='checkbox'
                checked={generatePolicy}
                onChange={(e) => setGeneratePolicy(e.target.checked)}
                className='w-4 h-4'
              />
              <span className='text-gray-700 dark:text-gray-300'>生成 Policy</span>
            </label>

            <label className='flex items-center space-x-2'>
              <input
                type='checkbox'
                checked={useUUIDPrimaryKey}
                onChange={(e) => setUseUUIDPrimaryKey(e.target.checked)}
                className='w-4 h-4'
              />
              <span className='text-gray-700 dark:text-gray-300'>UUID 主键</span>
            </label>

            <label className='flex items-center space-x-2'>
              <input
                type='checkbox'
                checked={enableSoftDelete}
                onChange={(e) => setEnableSoftDelete(e.target.checked)}
                className='w-4 h-4'
              />
              <span className='text-gray-700 dark:text-gray-300'>软删除</span>
            </label>
          </div>

          {/* Package Name Input */}
          <div className='flex items-center space-x-4 mb-4'>
            <label className='text-gray-700 dark:text-gray-300'>包名:</label>
            <input
              type='text'
              value={packageName}
              onChange={(e) => setPackageName(e.target.value)}
              className='px-3 py-1 border dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-200'
              placeholder='schema'
            />
          </div>

          {/* Error Display */}
          {error && (
            <div className='mt-4 p-3 bg-red-100 dark:bg-red-900/20 border border-red-300 dark:border-red-700 rounded'>
              <p className='text-red-700 dark:text-red-400'>{error}</p>
            </div>
          )}
        </div>

        {/* Main Content Area - Flexible height */}
        <div className='flex-1'>
          <Split
            sizes={[50, 50]}
            minSize={200}
            expandToMin={true}
            gutterSize={10}
            gutterAlign='center'
            snapOffset={30}
            dragInterval={1}
            direction='horizontal'
            cursor='col-resize'
            className='flex flex-row gap-2 h-full'>
            
            {/* SQL Input Panel */}
            <div className='flex flex-col h-full'>
              <div className='p-2 bg-gray-100 dark:bg-gray-700 border-b dark:border-gray-600 flex items-center justify-between flex-shrink-0'>
                <h2 className='font-semibold text-gray-800 dark:text-gray-200'>
                  SQL 输入
                </h2>
                <div className='flex items-center space-x-2'>
                  <button
                    onClick={loadExample}
                    className='px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500'>
                    示例
                  </button>
                  <button
                    onClick={clearAll}
                    className='px-3 py-1 text-sm bg-gray-500 text-white rounded hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-gray-500'>
                    清空
                  </button>
                </div>
              </div>
              <div className='flex-1 min-h-0 h-full'>
                <CodeEditor
                  language='sql'
                  value={input}
                  onChange={setInput}
                  options={{
                    minimap: { enabled: false },
                    wordWrap: 'off',
                  }}
                />
              </div>
            </div>

            {/* Go Ent Output Panel with Tabs */}
            <div className='flex flex-col h-full'>
              <div className='p-2 bg-gray-100 dark:bg-gray-700 border-b dark:border-gray-600 flex-shrink-0'>
                <h2 className='font-semibold text-gray-800 dark:text-gray-200'>
                  Go Ent Schema 输出
                </h2>
              </div>
              
              {/* Tabs for multiple tables */}
              {tableNames.length > 0 && (
                <div className='border-b border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-800 flex-shrink-0'>
                  <div className='flex overflow-x-auto'>
                    {tableNames.map((tableName, index) => (
                      <button
                        key={tableName}
                        onClick={() => setActiveTab(index)}
                        className={`px-4 py-2 text-sm font-medium whitespace-nowrap border-b-2 ${
                          activeTab === index
                            ? 'border-blue-500 text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-900/20'
                            : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300'
                        }`}>
                        {tableName}.go
                      </button>
                    ))}
                  </div>
                </div>
              )}
              
              <div className='flex-1 min-h-0 h-full'>
                <CodeEditor
                  language='go'
                  value={tableNames.length > 0 && activeTab < tableNames.length 
                    ? outputs[tableNames[activeTab]] || '' 
                    : ''}
                  readOnly={true}
                  options={{
                    minimap: { enabled: false },
                    wordWrap: 'off',
                    tabSize: 2,
                    insertSpaces: false,
                  }}
                />
              </div>
            </div>
          </Split>
        </div>
      </div>
    </ToolLayout>
  )
}

export default SqlToEnt