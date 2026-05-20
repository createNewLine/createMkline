# CLAUDE.md

## 项目概述

Windows 桌面 GUI 工具，用于将目录/文件迁移到其他磁盘，并在原位置创建软连接（mklink）。使用 Rust + Iced 框架。

## 构建/运行

```bash
cargo build --release          # 生成 release\mkLineExe.exe
cargo run                      # 开发运行
```

## 技术栈

- **GUI**: `iced` 0.12（Elm 架构：Message/update/view），Dark 主题
- **文件对话框**: `rfd` 0.14（异步）
- **异步运行时**: `tokio`（full features）
- **平台**: Windows only（依赖 `mklink` 命令和 `#![windows_subsystem = "windows"]`）
- **字体**: 从系统加载 `C:\Windows\Fonts\simhei.ttf`（黑体），不内嵌字体以减小二进制体积

## 架构

| 文件 | 职责 |
|------|------|
| `src/main.rs` | GUI 层：状态管理、消息处理、界面布局 |
| `src/ops.rs` | 核心操作：复制、删除、创建软连接、备份、回滚 |

### 消息流

用户交互 → `Message` → `update()` → 返回 `Command<Message>` → 执行异步操作 → 结果消息 → `update()` 更新状态

- 耗时操作（复制、备份）通过 `tokio::task::spawn_blocking` 在后台线程执行，避免阻塞 UI
- `Status::Running` 时禁用所有输入和操作按钮

### 核心操作流程（`ops::execute_confirm`）

对每个源路径：
1. 校验源存在、目标不存在
2. 复制源 → 目标目录
3. 删除源路径
4. 在原位置创建软连接（目录用 `mklink /J` 联接，文件用 `mklink` 符号链接）

任一步骤失败 → 回滚：删除所有已复制到目标目录的内容。

### 界面布局

- 顶部：标题
- 中上：源目录列表（动态增删，至少 1 个，最多 10 个），可滚动
- 中下：目标目录（单个）
- 底部状态栏 + 按钮行（确定/取消/备份/清空）

## 代码风格要点

- 遵循用户 CLAUDE.md 全局规则：能简则简、精准修改、不加未要求的功能
- 程序退出用 `std::process::exit(0)`，非 iced 的 `window::close`
- `mklink /J`（目录联接）无需管理员权限；文件符号链接可能需要管理员权限或开发者模式
