# UI 重构迁移指南

> 从现有样式迁移到新的设计系统

**更新日期**: 2025-02-05
**版本**: 1.0.0

---

## 🎯 迁移目标

将所有组件和工具从现有样式迁移到统一的设计系统:
- ✅ Slate 色系
- ✅ 玻璃态效果
- ✅ 统一圆角和过渡
- ✅ 完整的主题支持
- ✅ WCAG AA 可访问性

---

## 📋 迁移步骤

### Step 1: 评估现有组件

在开始迁移前,先评估组件的当前状态:

```bash
# 运行样式检查脚本
npm run check:styles

# 或手动检查
grep -r "gray-" src/
grep -r "rounded-md" src/
grep -r "duration-150" src/
```

**评估清单**:
- [ ] 列出所有需要修改的文件
- [ ] 识别使用的颜色、圆角、过渡
- [ ] 记录自定义样式
- [ ] 规划迁移优先级

### Step 2: 应用样式替换

按照以下模式进行替换:

#### 颜色替换

```tsx
// ❌ 旧样式
className="bg-gray-100"
className="text-gray-900"
className="border-gray-300"

// ✅ 新样式
className="bg-slate-100"
className="text-slate-900"
className="border-slate-300"
```

**完整替换规则**:
```bash
# 查找和替换 gray-* 为 slate-*
find src/ -name "*.tsx" -type f -exec sed -i '' 's/gray-/slate-/g' {} +
```

#### 圆角替换

```tsx
// ❌ 旧样式
className="rounded-sm"
className="rounded-md"

// ✅ 新样式
className="rounded-lg"
```

#### 过渡替换

```tsx
// ❌ 旧样式
className="duration-150"
className="duration-300"

// ✅ 新样式
className="duration-200"
```

#### 添加玻璃态效果

```tsx
// ❌ 旧样式
className="bg-white shadow-lg"

// ✅ 新样式
className="bg-white/80 dark:bg-slate-800/80 backdrop-blur-md shadow-lg border border-slate-200 dark:border-slate-600"
```

#### 添加 Focus Ring

```tsx
// ❌ 旧样式
<Button>点击</Button>
<input type="text" />

// ✅ 新样式
<Button className="focus:ring-2 focus:ring-primary-500">点击</Button>
<input className="focus:ring-2 focus:ring-primary-500 focus:border-transparent" type="text" />
```

#### 添加 Cursor Pointer

```tsx
// ❌ 旧样式
<button onClick={handleClick}>点击</button>

// ✅ 新样式
<button onClick={handleClick} className="cursor-pointer">点击</button>
```

#### 添加暗色模式

```tsx
// ❌ 旧样式
className="bg-white text-slate-900"

// ✅ 新样式
className="bg-white dark:bg-slate-800 text-slate-900 dark:text-slate-100"
```

### Step 3: 验证迁移结果

完成替换后,验证以下内容:

**样式验证**:
- [ ] 无 gray-* 类名残留
- [ ] 圆角统一为 rounded-lg
- [ ] 过渡统一为 duration-200
- [ ] 所有颜色有 dark: 变体

**交互验证**:
- [ ] 按钮有 hover/active/focus 状态
- [ ] 输入框有 focus ring
- [ ] 可点击元素有 cursor-pointer
- [ ] 过渡动画流畅

**主题验证**:
- [ ] 亮色模式正常
- [ ] 暗色模式正常
- [ ] 主题切换无闪烁

**功能验证**:
- [ ] 所有功能正常工作
- [ ] 无控制台错误
- [ ] 无性能问题

---

## 🔧 常见替换模式

### 卡片组件

```tsx
// ❌ 旧样式
<div className="bg-white shadow-md rounded-md p-4">
  内容
</div>

// ✅ 新样式
<div className="bg-white/80 dark:bg-slate-800/80 backdrop-blur-md
  shadow-lg rounded-lg p-4
  border border-slate-200 dark:border-slate-600">
  内容
</div>
```

### 按钮组件

```tsx
// ❌ 旧样式
<button className="bg-blue-500 text-white px-4 py-2 rounded-md hover:bg-blue-600">
  按钮
</button>

// ✅ 新样式
<button className="bg-primary-500 text-white px-4 py-2 rounded-lg
  hover:bg-primary-600 active:scale-[0.98]
  transition-all duration-200 cursor-pointer
  focus:outline-none focus:ring-2 focus:ring-primary-500">
  按钮
</button>
```

### 输入框组件

```tsx
// ❌ 旧样式
<input
  className="w-full px-3 py-2 border border-gray-300 rounded-md
    focus:outline-none focus:ring-2 focus:ring-blue-500"
/>

// ✅ 新样式
<input
  className="w-full px-3 py-2 border border-slate-300
    dark:border-slate-600 rounded-lg
    bg-white dark:bg-slate-800
    text-slate-900 dark:text-slate-100
    placeholder:text-slate-400
    transition-all duration-200
    focus:outline-none focus:ring-2 focus:ring-primary-500
    focus:border-transparent cursor-pointer"
/>
```

### 错误提示

```tsx
// ❌ 旧样式
<div className="bg-red-50 border border-red-200 text-red-600 p-3 rounded">
  {error}
</div>

// ✅ 新样式
<div className="bg-red-50 dark:bg-red-900/20
  border border-red-200 dark:border-red-800
  text-red-600 dark:text-red-400
  rounded-lg p-3 flex items-start space-x-2">
  <span className="flex-shrink-0">❌</span>
  <span className="text-sm">{error}</span>
</div>
```

### 侧边栏

```tsx
// ❌ 旧样式
<nav className="w-56 bg-white shadow-md p-4 rounded-md">
  内容
</nav>

// ✅ 新样式
<nav className="w-56
  bg-white/80 dark:bg-slate-800/80
  backdrop-blur-md
  shadow-lg p-4 rounded-lg
  border border-slate-200 dark:border-slate-600">
  内容
</nav>
```

---

## ✅ 验证清单

迁移完成后,使用此清单确保质量:

### 样式一致性
- [ ] 所有 `gray-*` 替换为 `slate-*`
- [ ] 所有 `rounded-md/sm` 替换为 `rounded-lg`
- [ ] 所有 `duration-150/300` 替换为 `duration-200`
- [ ] 所有交互元素有 `cursor-pointer`
- [ ] 所有交互元素有 `focus:ring-2 focus:ring-primary-500`

### 主题兼容
- [ ] 亮色模式文本对比度 ≥ 4.5:1
- [ ] 暗色模式文本对比度 ≥ 4.5:1
- [ ] 玻璃态效果在两种模式下可见
- [ ] 边框在两种模式下可见
- [ ] 测试主题切换

### 交互完整性
- [ ] Button: default, hover, active, focus, disabled
- [ ] Input: default, hover, focus, error
- [ ] 过渡动画平滑 (200ms)
- [ ] 加载状态有反馈
- [ ] 错误状态清晰

### 响应式
- [ ] 移动端 (375px) 布局正确
- [ ] 平板 (768px) 布局正确
- [ ] 桌面 (1024px+) 布局正确
- [ ] 无横向滚动
- [ ] 触摸目标 ≥ 44x44px

### 可访问性
- [ ] 所有交互元素有 `aria-label`
- [ ] 表单输入有关联 `label`
- [ ] 颜色不是唯一指示器
- [ ] 键盘导航完整
- [ ] 焦点管理正确

---

## 🐛 常见问题

### Q: 暗色模式下边框不可见

**A:** 确保添加了 `dark:border-slate-600`
```tsx
// ✅ 正确
className="border-slate-300 dark:border-slate-600"

// ❌ 错误
className="border-slate-300"
```

### Q: 玻璃态效果在暗色模式下失效

**A:** 检查是否使用了正确的透明度
```tsx
// ✅ 正确
className="bg-white/80 dark:bg-slate-800/80 backdrop-blur-md"

// ❌ 错误
className="bg-white dark:bg-slate-800 backdrop-blur-md"
```

### Q: Focus ring 不显示

**A:** 确保 outline 和 focus ring 不冲突
```tsx
// ✅ 正确
className="focus:outline-none focus:ring-2 focus:ring-primary-500"

// ❌ 错误
className="outline-none focus:ring-2"
```

### Q: 主题切换后组件样式不更新

**A:** 确保所有颜色都有 `dark:` 变体
```tsx
// ✅ 正确
className="bg-white dark:bg-slate-800"

// ❌ 错误
className="bg-white"
```

### Q: 过渡动画不平滑

**A:** 使用 transform 代替布局属性
```tsx
// ✅ 正确 - 使用 transform
className="transition-transform duration-200 hover:scale-105"

// ❌ 错误 - 避免过渡 width/height
className="transition-all duration-200 hover:w-full"
```

---

## 📚 参考资源

- [DESIGN_SYSTEM.md](../DESIGN_SYSTEM.md) - 完整设计系统
- [DEVELOPMENT.md](../DEVELOPMENT.md) - 快速开发参考
- [VISUAL_TEST_CHECKLIST.md](./VISUAL_TEST_CHECKLIST.md) - 视觉测试清单
- [Tailwind CSS 文档](https://tailwindcss.com/docs)

---

## 🚀 快速开始

```bash
# 1. 运行样式检查
npm run check:styles

# 2. 查看需要修改的文件
grep -r "gray-" src/

# 3. 应用替换
find src/ -name "*.tsx" -exec sed -i '' 's/gray-/slate-/g' {} +

# 4. 验证结果
npm run check:styles

# 5. 测试应用
npm run dev
```

---

**最后更新**: 2025-02-05
