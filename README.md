# KeySteer

<p align="center">
  <a href="https://github.com/dccif/KeySteer/actions/workflows/pages.yml"><img alt="Page status" src="https://img.shields.io/github/actions/workflow/status/dccif/KeySteer/pages.yml?branch=main&amp;label=Page&amp;style=flat&amp;logo=github&amp;logoColor=white"></a>
  <a href="https://github.com/dccif/KeySteer/actions/workflows/build.yml"><img alt="Build status" src="https://img.shields.io/github/actions/workflow/status/dccif/KeySteer/build.yml?branch=main&amp;label=Build&amp;style=flat&amp;logo=github&amp;logoColor=white"></a>
  <a href="https://github.com/dccif/KeySteer/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/dccif/KeySteer?display_name=tag&amp;sort=semver&amp;label=Release&amp;style=flat"></a>
  <a href="rust-toolchain.toml"><img alt="Rust 1.98" src="https://img.shields.io/badge/Rust-1.98-dea584?style=flat&amp;logo=rust&amp;logoColor=white"></a>
  <img alt="Windows 10 and 11" src="https://img.shields.io/badge/Windows-10%2F11-0078D4?style=flat">
  <img alt="macOS 14 or later" src="https://img.shields.io/badge/macOS-14%2B-000000?style=flat&amp;logo=apple&amp;logoColor=white">
  <a href="LICENSE"><img alt="License: GPL-3.0-or-later" src="https://img.shields.io/github/license/dccif/KeySteer?label=License&amp;style=flat"></a>
</p>

**从点击到分屏，把整个工作区交给键盘。**

语言 / Language · **简体中文** · [English（原项目）](https://github.com/dccif/KeySteer/blob/main/README.en.md)

KeySteer 是 Windows 和 macOS 上的原生键盘操控工具。用 `hjkl` 移动和点击，用标签定位界面，再用 Window 模式移动、分屏、组合窗口。少一些来回伸手，多一些连贯操作。

[原项目](https://github.com/dccif/KeySteer) · [原版下载](https://github.com/dccif/KeySteer/releases/latest) · [快速上手](https://dccif.github.io/KeySteer/guide/getting-started) · [Window 操作指南](https://dccif.github.io/KeySteer/modes/window)

## 个性化改动

本分支基于 **[dccif/KeySteer v0.10.12](https://github.com/dccif/KeySteer/releases/tag/v0.10.12)** 修改。原项目及其主要工作归功于原作者 **[dccif](https://github.com/dccif)**；本仓库继续遵循 [GPL-3.0-or-later](LICENSE) 许可证。

增加了不规则堆叠窗口组之间的快速切换：

- `C`：将堆叠组中的下一个窗口切到前面。
- `X`：将堆叠组中的上一个窗口切到前面。
- 使用独立动作名 `window_activate_next_2` 和 `window_activate_previous_2`，不改变原版动作。
- 改进部分 Qt / 沙盒窗口（例如 Telegram）的识别。

默认配置：

```toml
[normal.bindings]
c = "window_activate_next_2"
x = "window_activate_previous_2"
```

## 操作演示

[▶ 点击查看操作视频（MP4）](assets/demo/window-stack-cycle.mp4)

视频已转换为常见的 H.264 / AAC 格式，便于在浏览器中播放。
