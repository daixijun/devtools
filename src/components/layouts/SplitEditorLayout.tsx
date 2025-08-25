import React from 'react'
import Split from 'react-split'
import { CodeEditor } from '../common'

interface SplitEditorLayoutProps {
  leftTitle: string
  rightTitle: string
  leftValue: string
  rightValue: string
  onLeftChange?: (value: string) => void
  onRightChange?: (value: string) => void
  leftLanguage: string
  rightLanguage: string
  leftReadOnly?: boolean
  rightReadOnly?: boolean
  leftPlaceholder?: string
  rightPlaceholder?: string
  headerActions?: React.ReactNode
  leftActions?: React.ReactNode
  rightActions?: React.ReactNode
  sizes?: [number, number]
  minSize?: number
  className?: string
}

/**
 * 统一的分屏编辑器布局组件
 * 用于需要左右分栏的代码编辑场景
 */
const SplitEditorLayout: React.FC<SplitEditorLayoutProps> = ({
  leftTitle,
  rightTitle,
  leftValue,
  rightValue,
  onLeftChange,
  onRightChange,
  leftLanguage,
  rightLanguage,
  leftReadOnly = false,
  rightReadOnly = false,
  leftPlaceholder,
  rightPlaceholder,
  headerActions,
  leftActions,
  rightActions,
  sizes = [50, 50],
  minSize = 200,
  className = ''
}) => {
  return (
    <div className={`flex flex-col h-full ${className}`}>
      {/* 顶部操作区域 */}
      {headerActions && (
        <div className="p-4 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
          {headerActions}
        </div>
      )}

      {/* 分屏编辑区域 */}
      <Split
        sizes={sizes}
        minSize={minSize}
        expandToMin={true}
        gutterSize={10}
        gutterAlign='center'
        snapOffset={30}
        dragInterval={1}
        direction='horizontal'
        cursor='col-resize'
        className='flex flex-row h-full flex-1 overflow-hidden'
      >
        {/* 左侧编辑器 */}
        <div className='flex flex-col w-full h-full border-r border-gray-200 dark:border-gray-700'>
          {/* 左侧标题栏 */}
          <div className='flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600'>
            <h3 className='font-medium text-gray-800 dark:text-gray-200'>
              {leftTitle}
            </h3>
            {leftActions && (
              <div className="flex items-center space-x-2">
                {leftActions}
              </div>
            )}
          </div>
          
          {/* 左侧编辑器内容 */}
          <div className='flex-1 overflow-hidden h-full'>
            <CodeEditor
              language={leftLanguage}
              value={leftValue}
              onChange={onLeftChange}
              readOnly={leftReadOnly}
              options={{
                minimap: { enabled: false },
                tabSize: 2,
                wordWrap: 'on',
                formatOnPaste: true,
                formatOnType: true,
                placeholder: leftPlaceholder
              }}
            />
          </div>
        </div>

        {/* 右侧编辑器 */}
        <div className='flex flex-col w-full h-full'>
          {/* 右侧标题栏 */}
          <div className='flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600'>
            <h3 className='font-medium text-gray-800 dark:text-gray-200'>
              {rightTitle}
            </h3>
            {rightActions && (
              <div className="flex items-center space-x-2">
                {rightActions}
              </div>
            )}
          </div>
          
          {/* 右侧编辑器内容 */}
          <div className='flex-1 overflow-hidden h-full'>
            <CodeEditor
              language={rightLanguage}
              value={rightValue}
              onChange={onRightChange}
              readOnly={rightReadOnly}
              options={{
                minimap: { enabled: false },
                tabSize: 2,
                wordWrap: 'on',
                readOnly: rightReadOnly,
                placeholder: rightPlaceholder
              }}
            />
          </div>
        </div>
      </Split>
    </div>
  )
}

export default SplitEditorLayout