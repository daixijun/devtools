# 复制按钮修复说明

## 问题描述
在亮色主题下，复制按钮的文本颜色与背景一致，导致不可见。

## 根本原因
Button 组件的 `primary` variant 使用 `text-white`，当通过 `className` 覆盖背景色时，白色文本在某些背景色下不可见。

## 解决方案
**修复前**:
```tsx
<Button
  variant='primary'
  className={copied ? 'bg-green-600 hover:bg-green-700' : ''}>
  {copied ? '已复制' : '复制'}
</Button>
```
问题：`text-white` 在绿色背景上不够清晰。

**修复后**:
```tsx
<Button
  variant={copied ? 'ghost' : 'primary'}
  className={copied ? 'bg-green-100 text-green-700 hover:bg-green-200 dark:bg-green-900 dark:text-green-200 border-green-300 dark:border-green-700' : ''}>
  {copied ? '已复制 ✓' : '复制'}
</Button>
```
改进：
1. 使用 `ghost` variant 移除默认的白色文本
2. 通过 `className` 明确设置文本颜色和背景颜色
3. 亮色主题：绿色背景 + 深绿色文本
4. 暗色主题：深绿色背景 + 浅绿色文本

## 修复的工具
- ✅ PasswordGenerator
- ✅ Base64Converter
- ✅ JsonFormatter
- ✅ Md5Crypto (3个按钮)
- ✅ AesCrypto (2个按钮)
- ✅ JwtEncode
- ✅ JwtDecode
- ✅ PasswordHasher
- ✅ ShaCrypto

## 测试验证
在两种主题下验证：
- **亮色主题**: 已复制状态显示为绿色背景 + 深绿色文本
- **暗色主题**: 已复制状态显示为深绿色背景 + 浅绿色文本

---

**修复日期**: 2025-02-05
**提交**: 51e1cb7
