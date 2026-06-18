# 领域文档

工程类 skill 在探索代码库时应如何使用本仓库的领域文档。

## 探索前先读这些

- 根目录的 **`CONTEXT.md`**，或
- 如果存在根目录的 **`CONTEXT-MAP.md`**——它指向每个上下文一份的 `CONTEXT.md`。读取与当前主题相关的每一份。
- **`docs/adr/`**——阅读涉及你即将动手区域的 ADR。在多上下文仓库中，还要检查 `src/<context>/docs/adr/` 里上下文范围内的决策。

如果这些文件不存在，**静默继续**。不要标记它们缺失，也不要一上来就建议创建它们。`/domain-modeling` skill（通过 `/grill-with-docs` 和 `/improve-codebase-architecture` 触达）会在术语或决策真正被定下来时惰性创建它们。

## 文件结构

单上下文仓库（多数仓库，SwiftGlance 即属此类）：

```
/
├── CONTEXT.md
├── docs/adr/
│   ├── 0001-event-sourced-orders.md
│   └── 0002-postgres-for-write-model.md
└── src/
```

多上下文仓库（根目录存在 `CONTEXT-MAP.md`）：

```
/
├── CONTEXT-MAP.md
├── docs/adr/                          ← 系统级决策
└── src/
    ├── ordering/
    │   ├── CONTEXT.md
    │   └── docs/adr/                  ← 上下文专属决策
    └── billing/
        ├── CONTEXT.md
        └── docs/adr/
```

## 使用术语表的词汇

当你的输出命名某个领域概念时（在 issue 标题、重构提案、假设、测试名中），使用 `CONTEXT.md` 中定义的术语。不要漂移到术语表明确避免的同义词。

如果你需要的概念还不在术语表里，这是一个信号——要么你在发明项目并不使用的语言（请重新考虑），要么这里存在真实缺口（记录下来交给 `/domain-modeling`）。

## 标记 ADR 冲突

如果你的输出与某条现有 ADR 矛盾，明确指出而非默默覆盖：

> _与 ADR-0007（event-sourced orders）矛盾——但值得重开，因为……_
