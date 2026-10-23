# Documentation / 文档索引

[English README](../README.md) · [中文 README](../README.zh-CN.md)

本目录集中维护使用指南、技术规范和历史参考。入门介绍与 GIF 预览保留在根目录的中英文 README；项目级 Agent 规则和 Git 工作流也保留在根目录。

## Reading paths / 阅读顺序

- **使用应用**：README → [使用指南](USAGE.md) → [对话、验证与请求配置](CHAT.md)。
- **调整外观**：[外观配置](STYLE.md) → [当前界面设计规则](reference/DESIGN.md)。
- **理解实现**：[架构](reference/ARCHITECTURE.md) → [IR](reference/IR.md) → [渲染](RENDERER.md)。
- **参与开发**：[AGENTS](../AGENTS.md) → [Git 工作流](../GIT.md) → 与修改相关的技术规范。

## User guides / 使用指南

| 文档 | 内容 |
| --- | --- |
| [USAGE.md](USAGE.md) | 桌面操作、错误提示、CLI 和数据目录 |
| [CHAT.md](CHAT.md) | 多会话、附件、Token 用量、超时重试、工具验证及动画链接 |
| [STYLE.md](STYLE.md) | 配色、系统与本地字体、字号和外观文件 |
| [RENDERER.md](RENDERER.md) | Skia 渲染、时间线、缓存、缩放、导出与性能检查 |

## Technical references / 技术规范

这些文档保留原有规范正文，并统一标题和引用路径。架构、IR、工具和会话文档中包含概念设计与接口示例；它们不表示所有设想都已实现。阅读时先看文件顶部的范围说明和当前实现章节，再结合所链接的源码确认行为。

| 文档 | 阅读范围 |
| --- | --- |
| [ARCHITECTURE.md](reference/ARCHITECTURE.md) | 当前桌面对话流程、分层职责与架构设计参考 |
| [DESIGN.md](reference/DESIGN.md) | 当前 Neobrutalism 风格及必须遵守的组件、布局、交互、可访问性和最终检查规则 |
| [ERROR_HANDLING.md](reference/ERROR_HANDLING.md) | 错误分类、阶段、上下文与恢复规范；原 `EXCEPTON.md` |
| [IR.md](reference/IR.md) | 当前结构化动画扩展、版本兼容与各层 IR 的职责和设计示例 |
| [TOOLS.md](reference/TOOLS.md) | 当前对话工具、结构化执行记录与工具接口设计参考；原 `TOOL.md` |
| [SESSIONS.md](reference/SESSIONS.md) | 当前会话与 Token 持久化，以及会话、产物和修订模型设计 |

## Contribution and history / 贡献与历史

- [AGENTS.md](../AGENTS.md)：技术栈、模块规则、文档入口与验收要求。
- [GIT.md](../GIT.md)：工作区保护、提交范围、提交格式与 Git 操作规则。
- [旧水墨设计归档](archive/INK_WASH_DESIGN.md)：被 Neobrutalism 替代的设计及旧版 UI 说明，仅供历史参考，不作为当前要求。
- [LICENSE](../LICENSE)：项目许可证。测试字体的说明仍与资源一起保存在 [tests/fixtures/fonts](../tests/fixtures/fonts/README.md)。

## Maintaining documentation / 维护约定

- 使用指南放在 `docs/`；技术规范放在 `docs/reference/`；被替代的历史说明放在 `docs/archive/`。
- 每份文档使用一个主标题；代码示例中的标题不计入文档层级。
- 使用相对 Markdown 链接，移动文件时同步更新 README、AGENTS、文档索引及交叉引用。
- 区分已实现功能、规范要求、设计示例和历史记录，避免将接口设想写成现有能力。
- 不在使用指南中重复整份技术规范；通过链接指向相应说明。
- 仅改文档时检查链接、锚点、图片引用和差异；不要求运行 Rust 检查。
