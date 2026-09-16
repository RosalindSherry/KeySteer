[原项目](https://github.com/dccif/KeySteer) · [原版下载](https://github.com/dccif/KeySteer/releases/latest) · [快速上手](https://dccif.github.io/KeySteer/guide/getting-started) · [Window 操作指南](https://dccif.github.io/KeySteer/modes/window)

# 个性化改动

基于 **[dccif/KeySteer v0.10.12](https://github.com/dccif/KeySteer/releases/tag/v0.10.12)** 修改。

原项目及其主要工作归功于原作者 **[dccif](https://github.com/dccif)**；本修改版继续遵循 [GPL-3.0-or-later](LICENSE) 许可证。

- 增加不规则堆叠窗口组之间的快速切换。
- `C`：将堆叠组中的下一个窗口切到前面。
- `X`：将堆叠组中的上一个窗口切到前面。
- 使用独立动作名 `window_activate_next_2` 和 `window_activate_previous_2`，不改变原版动作。
- 改进部分 Qt / 沙盒窗口（例如 Telegram）的识别。

## 操作视频

[▶ 点击查看操作视频（MP4）](assets/demo/window-stack-cycle.mp4)
