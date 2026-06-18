# Issue tracker：GitHub

本仓库的 issue 和 PRD 以 GitHub issue 形式存在。所有操作使用 `gh` CLI。

## 约定

- **创建 issue**：`gh issue create --title "..." --body "..."`。多行正文用 heredoc。
- **读取 issue**：`gh issue view <number> --comments`，用 `jq` 过滤评论并同时获取标签。
- **列出 issue**：`gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'`，按需加 `--label` 和 `--state` 过滤。
- **评论 issue**：`gh issue comment <number> --body "..."`
- **添加 / 移除标签**：`gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **关闭 issue**：`gh issue close <number> --comment "..."`

仓库从 `git remote -v` 推断——在 clone 内运行时 `gh` 会自动识别。

## 当某个 skill 说「发布到 issue tracker」

创建一个 GitHub issue。

## 当某个 skill 说「获取相关 ticket」

运行 `gh issue view <number> --comments`。
