import React, { useState, useEffect } from 'react'
import { ToolLayout } from '../layouts'
import { Button, InputField } from '../common'
import { useCopyToClipboard } from '../../hooks'

/**
 * ToolTemplate - 工具组件模板
 *
 * @description
 * 这是一个标准的工具组件模板,所有工具都应遵循这个结构。
 * 包含标准的布局、样式模式、交互流程。
 *
 * @features
 * - 使用 ToolLayout 统一布局
 * - 使用设计系统规范的颜色和样式
 * - 完整的亮色/暗色主题支持
 * - 错误处理和用户反馈
 * - 复制功能
 *
 * @accessibility
 * - 完整键盘导航支持
 * - 所有按钮有 aria-label
 * - 焦点管理符合 WCAG 2.1 AA
 *
 * @example
 * ```tsx
 * <MyTool />
 * ```
 *
 * @see DESIGN_SYSTEM.md - 设计系统规范
 */
interface ToolTemplateProps {}

const ToolTemplate: React.FC<ToolTemplateProps> = () => {
  // 状态管理
  const [input, setInput] = useState('')
  const [output, setOutput] = useState('')
  const [error, setError] = useState('')
  const { copy, copied } = useCopyToClipboard()

  // 处理函数
  const handleClear = () => {
    setInput('')
    setOutput('')
    setError('')
  }

  const handleCopy = async () => {
    if (output) {
      await copy(output)
    }
  }

  const handleLoadExample = () => {
    const example = '示例文本'
    setInput(example)
  }

  // 快捷操作按钮组
  const actions = (
    <div className="flex space-x-2">
      <Button
        variant="ghost"
        size="sm"
        onClick={handleClear}
        disabled={!input && !output}
        aria-label="清空内容"
      >
        清空
      </Button>
      <Button
        variant="ghost"
        size="sm"
        onClick={handleCopy}
        disabled={!output}
        aria-label={copied ? '已复制' : '复制结果'}
      >
        {copied ? '已复制 ✓' : '复制'}
      </Button>
      <Button
        variant="ghost"
        size="sm"
        onClick={handleLoadExample}
        aria-label="加载示例"
      >
        示例
      </Button>
    </div>
  )

  return (
    <ToolLayout
      title="工具名称"
      subtitle="工具简短描述"
      description="详细说明工具的用途和使用方法"
      actions={actions}
    >
      {/* 工具主要内容 */}
      <div className="space-y-4">
        {/* 输入区域 */}
        <div className="bg-white/80 dark:bg-slate-800/80 backdrop-blur-md
          rounded-lg shadow-lg p-4 border border-slate-200 dark:border-slate-600">
          <div className="mb-3">
            <label className="block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2">
              输入
            </label>
            <InputField
              value={input}
              onChange={setInput}
              placeholder="请输入内容..."
            />
          </div>

          {/* 错误提示 */}
          {error && (
            <div className="mt-3 bg-red-50 dark:bg-red-900/20
              border border-red-200 dark:border-red-800
              text-red-600 dark:text-red-400
              rounded-lg p-3 flex items-start space-x-2">
              <span className="flex-shrink-0">❌</span>
              <span className="text-sm">{error}</span>
            </div>
          )}
        </div>

        {/* 输出区域 */}
        <div className="bg-white/80 dark:bg-slate-800/80 backdrop-blur-md
          rounded-lg shadow-lg p-4 border border-slate-200 dark:border-slate-600">
          <label className="block text-sm font-medium text-slate-700 dark:text-slate-300 mb-2">
            输出
          </label>
          {output ? (
            <div className="p-3 bg-slate-50 dark:bg-slate-900/50
              rounded-lg border border-slate-200 dark:border-slate-700
              text-slate-900 dark:text-slate-100 font-mono text-sm
              overflow-x-auto">
              {output}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-12
              text-slate-500 dark:text-slate-400">
              <p className="text-sm">结果将在这里显示...</p>
            </div>
          )}
        </div>
      </div>
    </ToolLayout>
  )
}

export default ToolTemplate
