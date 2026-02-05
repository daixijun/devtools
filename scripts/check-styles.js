#!/usr/bin/env node

/**
 * 样式一致性检查脚本
 *
 * 检查所有组件文件是否符合设计系统规范:
 * - 使用 Slate 色系 (不是 gray)
 * - 统一圆角 rounded-lg
 * - 标准过渡 duration-200
 * - Focus ring
 * - cursor-pointer
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// 检查规则
const rules = {
  // 禁止使用 gray-*, 应使用 slate-*
  noGray: {
    pattern: /(?:className=["'][^"']*\b)?gray-\d+/g,
    message: '应使用 slate-* 而不是 gray-*',
    fix: (match) => match.replace('gray-', 'slate-'),
  },

  // 禁止使用 rounded-md/sm, 应使用 rounded-lg
  rounded: {
    pattern: /rounded-(?:md|sm)/g,
    message: '应使用 rounded-lg 而不是 rounded-md/sm',
    fix: (match) => match.replace(/rounded-(md|sm)/, 'rounded-lg'),
  },

  // 禁止使用 duration-150/300, 应使用 duration-200
  duration: {
    pattern: /duration-(?:150|300)/g,
    message: '应使用 duration-200 而不是 duration-150/300',
    fix: (match) => match.replace(/duration-(150|300)/, 'duration-200'),
  },

  // 检查是否有 cursor-pointer (可点击元素)
  cursorPointer: {
    pattern: /onClick=.*?(?!.*cursor-pointer)/g,
    message: '可点击元素应添加 cursor-pointer',
    type: 'warning',
  },

  // 检查是否有 focus ring (交互元素)
  focusRing: {
    pattern: /(?:className=["'][^"']*\b)?(?:input|button)/gi,
    message: '交互元素应有 focus:ring-2 focus:ring-primary-500',
    type: 'warning',
  },
};

// 需要检查的文件
const checkPaths = [
  'src/components/**/*.tsx',
  'src/tools/*.tsx',
  'src/layouts/*.tsx',
];

let totalErrors = 0;
let totalWarnings = 0;
const results = [];

/**
 * 检查单个文件
 */
function checkFile(filePath) {
  const content = fs.readFileSync(filePath, 'utf8');
  const fileResults = [];
  let fileErrors = 0;
  let fileWarnings = 0;

  // 检查每个规则
  for (const [ruleName, rule] of Object.entries(rules)) {
    const matches = content.match(rule.pattern);

    if (matches) {
      matches.forEach(match => {
        const isWarning = rule.type === 'warning';
        const result = {
          file: filePath,
          rule: ruleName,
          message: rule.message,
          match: match,
          isWarning: isWarning,
        };

        fileResults.push(result);

        if (isWarning) {
          fileWarnings++;
          totalWarnings++;
        } else {
          fileErrors++;
          totalErrors++;
        }
      });
    }
  }

  if (fileResults.length > 0) {
    results.push({
      file: filePath,
      errors: fileErrors,
      warnings: fileWarnings,
      issues: fileResults,
    });
  }
}

/**
 * 递归检查目录
 */
function checkDirectory(pattern) {
  const files = execSync(`find ${pattern} -type f`, { encoding: 'utf8' })
    .trim()
    .split('\n')
    .filter(Boolean);

  files.forEach(checkFile);
}

/**
 * 打印结果
 */
function printResults() {
  console.log('\n🎨 样式一致性检查结果\n');

  if (results.length === 0) {
    console.log('✅ 所有文件都符合设计系统规范!\n');
    return;
  }

  results.forEach(result => {
    const relativePath = path.relative(process.cwd(), result.file);
    console.log(`\n📄 ${relativePath}`);
    console.log(`   ❌ 错误: ${result.errors}   ⚠️  警告: ${result.warnings}`);

    result.issues.forEach(issue => {
      const icon = issue.isWarning ? '⚠️' : '❌';
      console.log(`   ${icon} ${issue.rule}: ${issue.message}`);
      if (!issue.isWarning) {
        console.log(`      找到: "${issue.match}"`);
      }
    });
  });

  console.log('\n📊 总结:');
  console.log(`   ❌ 总错误: ${totalErrors}`);
  console.log(`   ⚠️  总警告: ${totalWarnings}`);
  console.log(`   📁 检查文件: ${results.length} 个\n`);

  if (totalErrors > 0) {
    console.log('❌ 检查失败! 请修复上述错误后重试。\n');
    process.exit(1);
  } else {
    console.log('⚠️  有一些警告,建议修复以提升代码质量。\n');
  }
}

// 主函数
function main() {
  console.log('🔍 开始检查样式一致性...\n');

  checkPaths.forEach(pattern => {
    try {
      checkDirectory(pattern);
    } catch (error) {
      // 忽略不存在的文件
    }
  });

  printResults();
}

// 运行
main();
