import { Editor as MonacoEditor } from '@monaco-editor/react'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import React, { useEffect, useState } from 'react'
import Split from 'react-split'

const SqlToGo: React.FC = () => {
  const [isDark, setIsDark] = useState(false)
  useEffect(() => {
    const window = getCurrentWindow()
    window.theme().then((theme) => setIsDark(theme === 'dark'))
    window.onThemeChanged(({ payload: theme }) => {
      setIsDark(theme === 'dark')
    })
  }, [])
  const [input, setInput] = useState(`CREATE TABLE users (
    id INT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);`)
  const [output, setOutput] = useState('')
  const [enablePluralization, setEnablePluralization] = useState(true)
  const [exportedFields, setExportedFields] = useState(true)
  const [isGo124OrAbove, setIsGo124OrAbove] = useState(true)
  const [jsonNullHandling, setJsonNullHandling] = useState<
    'none' | 'omitempty' | 'omitzero'
  >('omitempty')
  const [selectedTags, setSelectedTags] = useState<Record<string, boolean>>({
    json: true,
    gorm: true,
    db: false,
    sql: false,
    yaml: false,
    toml: false,
    env: false,
    ini: false,
  })

  // Handle Go 1.24+ dependency for omitzero
  useEffect(() => {
    if (!isGo124OrAbove && jsonNullHandling === 'omitzero') {
      setJsonNullHandling('omitempty')
    }
  }, [isGo124OrAbove, jsonNullHandling])

  // Auto-conversion effect
  useEffect(() => {
    const timer = setTimeout(() => {
      if (input.trim()) {
        convertSqlToGo()
      }
    }, 500) // 500ms delay to avoid excessive API calls

    return () => clearTimeout(timer)
  }, [
    input,
    enablePluralization,
    exportedFields,
    isGo124OrAbove,
    jsonNullHandling,
    selectedTags,
  ])

  const handleLoadExample = () => {
    const exampleSql = `CREATE TABLE users (
    id INT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    age INT,
    active BOOLEAN DEFAULT TRUE,
    profile TEXT
);

CREATE TABLE posts (
    id INT PRIMARY KEY AUTO_INCREMENT,
    title VARCHAR(255) NOT NULL,
    content TEXT,
    user_id INT,
    published BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);`
    setInput(exampleSql)
  }

  const handleClearInput = () => {
    setInput('')
    setOutput('')
  }

  const convertSqlToGo = () => {
    if (!input.trim()) {
      setOutput('')
      return
    }

    const options = {
      enable_pluralization: enablePluralization,
      exported_fields: exportedFields,
      is_go_124_or_above: isGo124OrAbove,
      json_null_handling: jsonNullHandling,
      selected_tags: selectedTags,
    }

    console.log('Sending options to backend:', options)
    console.log('GORM tag enabled:', selectedTags.gorm)
    console.log('Exported fields:', exportedFields)

    invoke('convert_sql_to_go', {
      sql: input,
      options,
    })
      .then((result: any) => {
        // Handle the new direct response format (GoStructOutput)
        if (result && typeof result === 'object') {
          console.log('Backend raw output:', JSON.stringify(result))

          // Extract the first table's output for display
          if (result.outputs && typeof result.outputs === 'object') {
            const firstTableKey = Object.keys(result.outputs)[0]
            if (firstTableKey && result.outputs[firstTableKey]) {
              const goCode = result.outputs[firstTableKey]
              console.log('Backend output for table:', firstTableKey)
              setOutput(goCode)
            } else {
              setOutput('') // No output found
            }
          } else {
            setOutput('') // Invalid output structure
          }
        } else {
          throw new Error('无效的响应格式')
        }
      })
      .catch((err) => {
        console.error('Error in convertSqlToGo:', err)
        setOutput('') // Clear output on error
      })
  }

  const handleTagChange = (tag: string, checked: boolean) => {
    setSelectedTags((prev) => ({ ...prev, [tag]: checked }))
  }

  return (
    <div className='flex flex-col h-full'>
      <div className='p-4 border-b dark:border-gray-600 bg-white dark:bg-gray-800'>
        <h1 className='text-xl font-bold mb-4 text-gray-800 dark:text-gray-200'>
          SQL 转 Go 结构体
        </h1>

        <div className='flex flex-wrap gap-4 mb-4'>
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
              checked={exportedFields}
              onChange={(e) => setExportedFields(e.target.checked)}
              className='w-4 h-4'
            />
            <span className='text-gray-700 dark:text-gray-300'>导出字段</span>
          </label>

          <label className='flex items-center space-x-2'>
            <input
              type='checkbox'
              checked={isGo124OrAbove}
              onChange={(e) => setIsGo124OrAbove(e.target.checked)}
              className='w-4 h-4'
            />
            <span className='text-gray-700 dark:text-gray-300'>Go 1.24+</span>
          </label>

          <div className='flex items-center space-x-2'>
            <span className='text-gray-700 dark:text-gray-300'>JSON 处理:</span>
            <select
              value={jsonNullHandling}
              onChange={(e) => setJsonNullHandling(e.target.value as any)}
              className='px-2 py-1 border dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-200'>
              <option value='none'>无</option>
              <option value='omitempty'>omitempty</option>
              {isGo124OrAbove && <option value='omitzero'>omitzero</option>}
            </select>
          </div>
        </div>

        <div className='flex flex-wrap gap-4 mb-4'>
          <span className='font-medium text-gray-700 dark:text-gray-300'>
            标签:
          </span>
          {Object.entries(selectedTags).map(([tag, checked]) => (
            <label key={tag} className='flex items-center space-x-2'>
              <input
                type='checkbox'
                checked={checked}
                onChange={(e) => handleTagChange(tag, e.target.checked)}
                className='w-4 h-4'
              />
              <span className='uppercase text-gray-700 dark:text-gray-300'>
                {tag}
              </span>
            </label>
          ))}
        </div>
      </div>

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
        className='flex flex-row gap-4 h-full flex-1 overflow-hidden'>
        <div className='flex flex-col w-full h-full'>
          <div className='p-2 bg-gray-100 dark:bg-gray-700 border-b dark:border-gray-600 flex items-center justify-between flex-shrink-0'>
            <h2 className='font-semibold text-gray-800 dark:text-gray-200'>
              SQL 输入
            </h2>
            <div className='flex items-center space-x-2'>
              <button
                onClick={handleLoadExample}
                className='px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500'>
                示例
              </button>
              <button
                onClick={handleClearInput}
                className='px-3 py-1 text-sm bg-gray-500 text-white rounded hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-gray-500'>
                清空
              </button>
            </div>
          </div>
          <div className='mb-4'>
            <MonacoEditor
              height='75vh'
              language='sql'
              theme={isDark ? 'vs-dark' : 'vs'}
              value={input}
              onChange={(newValue) =>
                newValue !== undefined && setInput(newValue)
              }
              options={{
                minimap: { enabled: false },
                scrollBeyondLastLine: false,
                automaticLayout: true,
                fontSize: 14,
                tabSize: 2,
                wordWrap: 'off',
                readOnly: false,
                glyphMargin: false,
              }}
            />
          </div>
        </div>

        <div className='flex flex-col w-full h-full'>
          <div className='p-2 bg-gray-100 dark:bg-gray-700 border-b dark:border-gray-600 flex items-center justify-between flex-shrink-0'>
            <h2 className='font-semibold text-gray-800 dark:text-gray-200'>
              Go 结构体输出
            </h2>
          </div>
          <div className='mb-4'>
            <MonacoEditor
              height='75vh'
              language='go'
              theme={isDark ? 'vs-dark' : 'vs'}
              value={output}
              onChange={(newValue) => {
                newValue !== undefined && setOutput(newValue)
              }}
              options={{
                minimap: { enabled: false },
                scrollBeyondLastLine: false,
                automaticLayout: true,
                fontSize: 14,
                tabSize: 2,
                // wordWrap: 'off',
                readOnly: true,
                glyphMargin: false,
              }}
            />
          </div>
        </div>
      </Split>
    </div>
  )
}

export default SqlToGo
