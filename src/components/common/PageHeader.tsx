import React from 'react'

interface PageHeaderProps {
  title: string
  subtitle?: string
  className?: string
}

const PageHeader: React.FC<PageHeaderProps> = ({ title, subtitle, className = '' }) => (
  <div className={className}>
    <h2 className='text-xl font-bold mb-2 text-gray-800 dark:text-white'>
      {title}
    </h2>
    {subtitle && (
      <p className='text-sm text-gray-600 dark:text-gray-400 mb-2'>
        {subtitle}
      </p>
    )}
  </div>
)

export default PageHeader