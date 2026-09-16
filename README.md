[下载体验](https://github.com/dccif/KeySteer/releases/latest) · [快速上手](https://dccif.github.io/KeySteer/guide/getting-started) · [Window 操作指南](https://dccif.github.io/KeySteer/modes/window) · [在线模拟器](https://dccif.github.io/KeySteer/editor/)

## 个性化改动

基于 [dccif/KeySteer v0.10.12](https://github.com/dccif/KeySteer/releases/tag/v0.10.12) 修改，增加重叠窗口之间的快速切换。

```toml
c = "window_activate_next_2"
x = "window_activate_previous_2"
```

### 切换规则

- 鼠标放在哪个窗口，就从它所在的重叠窗口组开始切换。
- `C` 切到下一个窗口，`X` 切到上一个窗口。
- 切换后的窗口会来到最前面，鼠标也会移动到它的中心。
- 不重叠或已经最小化的窗口会被跳过。

例如一组窗口的前后顺序是 `1、2、3`：

```text
C：1 → 2 → 3 → 1
X：1 → 3 → 2 → 1
```

## 演示视频

https://github.com/user-attachments/assets/9fdbfbc7-b5ff-4749-821e-3c9cc5de3739
