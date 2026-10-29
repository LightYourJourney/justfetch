# justfetch

**Just fetch your system. Beautifully.**

一个紧凑的离线 Rust 命令行工具，显示真实的本地系统信息：操作系统、主机型号、内核、运行时间、CPU、GPU、内存、交换、磁盘、电池、显示器（按需）、各包管理器已安装数量、桌面会话（Linux）、区域设置、Shell、终端与本地地址。

> **本文件是简体中文摘要。** [README.md](README.md) 是权威版本；两者冲突时以英文版为准。

**状态：** 当前源码是 **v1.0.0-rc.1 发布候选**，已作为 GitHub prerelease 发布，**未发布到 crates.io**，也**没有**可用的 Homebrew / Scoop 正式清单。已在 Apple Silicon macOS 上实际运行验证；Intel macOS 二进制可交叉构建并在 Rosetta 下运行；Linux 与 Windows 由 CI 原生构建并跑测试，打包后的二进制也会在这些 runner 上运行——但没有任何测试把采集结果与这两个平台自带的工具比对，且 `display` 因不在默认模块中而从不在 CI 运行；WSL 未测试。详见 [兼容性矩阵](docs/compatibility.md)。

## 功能

- 十七个模块，默认显示十五行（macOS/Windows）；Linux 默认十六行。`display` 在所有平台都按需启用（它在 macOS 上的查询要花几十毫秒），`desktop` 只在 Linux 默认显示。
- 明确的状态语义：`available` / `unavailable` / `unsupported` / `redacted` / `deselected` / `error`，JSON 里带来源与置信度。
- 模块选择与排序、自定义标签与分隔符、TOML 配置文件、`justfetch init`、`justfetch doctor`。
- 六套配色主题、四种布局、自定义主题与 Logo 文件；`justfetch themes` 预览。
- `--privacy` 隐去用户名、主机名与本地地址，JSON 与文本一致。
- 原创 ASCII Logo，Unicode 显示宽度对齐，窄终端自动降级，ANSI 颜色能力自动匹配。
- 无网络请求、无遥测、无子进程、不需要管理员权限。

## 安装

从源码检出目录：

```sh
cargo install --path . --locked
```

或从 GitHub Release 下载对应平台的压缩包（内含二进制、`LICENSE` 与 `README.md`），校验 `SHA256SUMS` 后解压放入 `PATH`。

## 用法

```sh
justfetch
justfetch --format json
justfetch --no-color
justfetch --privacy
justfetch --modules os,cpu,memory
justfetch --theme nord --layout compact
justfetch themes
justfetch init
justfetch doctor
```

## 平台

| 平台 | 状态 |
| --- | --- |
| macOS ARM64 | 已实际运行验证 |
| macOS Intel | 交叉构建，并在 Rosetta 下运行验证（非 Intel 硬件） |
| Linux x86-64 / ARM64 | CI 构建、测试与打包冒烟；未在真实发行版上交互验证 |
| Windows x64 | CI 构建、测试与打包冒烟；未在真实机器上交互验证 |

## 文档

- [架构](docs/architecture.md)
- [配置参考](docs/configuration.md)
- [主题、布局与 Logo](docs/themes.md)
- [兼容性矩阵](docs/compatibility.md)
- [性能与测量方法](docs/benchmark.md)
- [开发指南](docs/development.md)
- [验证记录](docs/validation.md)
- [安全策略](SECURITY.md)

## 许可证

[GNU GPL v3](LICENSE)，声明为 `GPL-3.0-only`。内置 ASCII 素材为本项目原创并以同一许可证分发。
