# 变更日志

## [2.0.0] - 2025-02-05

### 🎨 重大变更 - UI 全面重构

#### 新增 ✨
- 完整的设计系统文档 (DESIGN_SYSTEM.md)
- 迁移指南文档 (docs/ui/MIGRATION_GUIDE.md)
- 视觉测试清单 (docs/ui/VISUAL_TEST_CHECKLIST.md)
- 样式检查脚本 (scripts/check-styles.js)
- 工具组件模板 (src/components/templates/ToolTemplate.tsx)
- 可复用的数据处理Hooks:
  - useToolProcessor - 同步数据处理
  - useToolAsyncProcessor - 异步数据处理

#### 变更 🔄
- **色彩系统**: 从 gray 色系全面迁移到 slate 色系
- **主色调**: 统一使用 primary-500 (#3B82F6) 替代 blue-600
- **圆角规范**: 统一使用 rounded-lg (8px)
- **过渡时长**: 统一使用 duration-200 (200ms)
- **玻璃态效果**: 添加 backdrop-blur-md 和透明度

#### 组件重构
- **Button组件**:
  - 添加 ghost variant
  - 完善可访问性 (aria-label, aria-busy)
  - 统一样式规范

- **InputField组件**:
  - 添加 success 状态支持
  - 完善 aria 属性
  - 统一样式规范

- **Card组件**:
  - 添加玻璃态效果
  - 统一阴影和圆角

- **ErrorMessage组件**:
  - 改进视觉样式
  - 添加图标支持

#### 工具重构
- 批量重构28个工具组件
- 统一应用设计系统规范
- 完善暗色模式支持

### 📝 文档更新
- 新增完整的UI设计方案文档
- 新增开发者迁移指南
- 新增视觉测试清单

### 🎯 设计原则
- 使用 Slate 色系替代 gray
- 统一圆角 rounded-lg
- 标准过渡 duration-200
- 主色调 primary-500
- 完整的亮色/暗色主题
- WCAG AA 可访问性

---

## [1.x] - 之前版本

### 功能
- 25+ 开发工具
- 侧边栏导航
- Monaco编辑器集成
- 暗色模式支持

---

**最后更新**: 2025-02-05
