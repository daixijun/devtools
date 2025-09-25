import Editor from '@monaco-editor/react'
import React, { useEffect, useRef } from 'react'
import { useTheme } from '../../../hooks'

interface CodeEditorProps {
  value: string
  onChange?: (value: string) => void
  language: string
  readOnly?: boolean
  minimap?: boolean
  lineNumbers?: 'on' | 'off' | 'relative' | 'interval'
  wordWrap?: 'on' | 'off' | 'wordWrapColumn' | 'bounded'
  fontSize?: number
  className?: string
  placeholder?: string
  onMount?: (editor: any, monaco: any) => void
  options?: any
}

/**
 * 统一的代码编辑器组件
 * 基于Monaco Editor，支持多种编程语言和主题
 */
const CodeEditor: React.FC<CodeEditorProps> = ({
  value,
  onChange,
  language,
  readOnly = false,
  minimap = true,
  lineNumbers = 'on',
  wordWrap = 'on',
  fontSize = 14,
  className = '',
  placeholder,
  onMount,
  options = {},
}) => {
  const { isDark } = useTheme()
  const editorRef = useRef<any>(null)
  const monacoRef = useRef<any>(null)

  const defaultOptions = {
    readOnly,
    minimap: { enabled: minimap },
    lineNumbers,
    wordWrap,
    fontSize,
    scrollBeyondLastLine: false,
    smoothScrolling: true,
    cursorStyle: 'line',
    renderWhitespace: 'boundary',
    bracketPairColorization: {
      enabled: true,
    },
    suggest: {
      showKeywords: true,
      showSnippets: true,
      showClasses: true,
      showFunctions: true,
      showVariables: true,
    },
    quickSuggestions: {
      other: true,
      comments: true,
      strings: true,
    },
    parameterHints: {
      enabled: true,
    },
    formatOnPaste: true,
    formatOnType: true,
    ...(placeholder && !value && { placeholder }),
    ...options,
  }

  // 主题变化时更新编辑器主题
  useEffect(() => {
    if (monacoRef.current && editorRef.current) {
      monacoRef.current.editor.setTheme(isDark ? 'vs-dark' : 'vs')
    }
  }, [isDark])

  const handleEditorChange = (value: string | undefined) => {
    if (onChange && value !== undefined) {
      onChange(value)
    }
  }

  const handleEditorMount = (editor: any, monaco: any) => {
    editorRef.current = editor
    monacoRef.current = monaco

    // 设置初始主题
    monaco.editor.setTheme(isDark ? 'vs-dark' : 'vs')

    // 注册常用的语言配置
    monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
      validate: true,
      allowComments: false,
      schemas: [],
    })

    if (onMount) {
      onMount(editor, monaco)
    }
  }

  return (
    <div
      className={`border border-gray-300 dark:border-gray-600 rounded-md overflow-hidden h-full ${className}`}>
      <Editor
        height='100%'
        language={language}
        value={value}
        onChange={handleEditorChange}
        theme={isDark ? 'vs-dark' : 'vs'}
        options={defaultOptions}
        onMount={handleEditorMount}
        loading={
          <div className='flex items-center justify-center h-full'>
            <div className='animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600'></div>
          </div>
        }
      />
    </div>
  )
}

export default CodeEditor
