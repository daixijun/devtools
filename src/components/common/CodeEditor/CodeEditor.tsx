import Editor from '@monaco-editor/react'
import React, { useEffect, useRef, useState } from 'react'
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
  const { isDark, isLoading, theme } = useTheme()
  const editorRef = useRef<any>(null)
  const monacoRef = useRef<any>(null)
  const [editorReady, setEditorReady] = useState(false)

  // 添加双重检查：主题加载完成 且 编辑器准备就绪
  const shouldShowEditor = !isLoading

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

    // 确保主题正确设置
    monaco.editor.setTheme(isDark ? 'vs-dark' : 'vs')

    // 标记编辑器已准备就绪
    setEditorReady(true)

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

  // 如果正在加载主题，显示加载状态
  if (isLoading) {
    return (
      <div
        className={`border border-gray-300 dark:border-gray-600 rounded-md overflow-hidden h-full ${className}`}>
        <div className='flex items-center justify-center h-full'>
          <div className='animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600'></div>
          <span className='ml-2 text-gray-600 dark:text-gray-400'>加载主题中...</span>
        </div>
      </div>
    )
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
            <span className='ml-2 text-gray-600 dark:text-gray-400'>编辑器加载中...</span>
          </div>
        }
      />
    </div>
  )
}

export default CodeEditor
