# Oj-Agent

> 输入一道算法题，自动生成带有动画演示的题解。

Oj-Agent 是一款基于 Wails3 的跨平台桌面应用，结合 LLM 大模型能力，将枯燥的算法题解转化为可视化的动画演示，帮助用户更直观地理解算法思路。

---

## 核心功能

- **智能题解生成**：输入题目描述，自动调用 LLM 生成解题思路、代码实现和时间/空间复杂度分析
- **动画演示**：将算法执行过程（如双指针、递归、动态规划、图遍历等）以动画形式逐步展示
- **本地持久化**：所有题目和题解存储在本地 SQLite 数据库中，支持历史记录检索
- **多语言支持**：支持生成多种编程语言的题解代码（Go、Python、C++、Java 等）

---

## 技术栈

| 层级     | 技术                                      |
| -------- | ----------------------------------------- |
| 后端     | Go 1.25+                                  |
| AI 编排  | [eino](https://github.com/cloudwego/eino) |
| 桌面框架 | [Wails3](https://v3.wails.io/)            |
| 数据存储 | SQLite                                    |
| 前端     | Vue 3 + Vite                              |

---

## 项目结构

```
Oj-Agent/
├── backend/            # Go 后端核心逻辑
│   ├── agent/          # eino agent 编排（prompt 模板、工具链等）
│   ├── llm/            # LLM 调用封装（OpenAI / 本地模型）
│   ├── parser/         # 题解内容解析（提取算法步骤、代码块）
│   ├── anim/           # 动画数据生成（将算法步骤转为动画帧）
│   ├── db/             # SQLite 数据库操作
│   └── server/         # Wails3 后端 API
├── frontend/           # 前端页面 (Vue 3)
│   ├── src/
│   │   ├── components/ # UI 组件
│   │   ├── pages/      # 页面
│   │   └── anim/       # 动画渲染引擎（Canvas / SVG）
│   └── package.json
├── build/              # 构建配置（多平台）
├── go.mod
├── go.sum
├── main.go             # 应用入口
├── greetservice.go     # 示例 Service
├── Taskfile.yml        # Task 任务配置
└── README.md
```

---

## 快速开始

### 环境要求

- Go 1.25+
- [Wails3 CLI](https://v3.wails.io/getting-started/your-first-app/)
- Node.js 18+
- SQLite 3

### 安装与运行

```bash
# 克隆项目
git clone https://github.com/your-org/Oj-Agent.git
cd Oj-Agent

# 设置环境变量（将 xxx 替换为你的 API Key）
cp .env.example .env

# 安装前端依赖
cd frontend && npm install && cd ..

# 生成 Wails3 绑定
wails3 generate bindings

# 开发模式启动
wails3 dev
```

---

## 配置

在 `.env` 中配置 LLM 相关参数：

```env
# OpenAI 兼容 API
OPENAI_API_KEY=sk-xxx
OPENAI_BASE_URL=https://api.openai.com/v1
OPENAI_MODEL=gpt-4o

# 本地模型（可选，使用 Ollama 等）
# OPENAI_BASE_URL=http://localhost:11434/v1
# OPENAI_MODEL=qwen2.5:7b
```

---

## 开发计划

- [ ] eino agent 编排（调用 LLM 生成题解）
- [ ] 题解文本解析（区分思路、代码、复杂度分析）
- [ ] 动画数据生成引擎
- [ ] 前端动画渲染器
- [ ] Wails3 桌面应用集成
- [ ] SQLite 数据持久化
- [ ] 本地模型支持（Ollama）

---

## License

MIT License
