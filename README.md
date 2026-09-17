[快速上手](https://dccif.github.io/KeySteer/guide/getting-started) · [Window 操作指南](https://dccif.github.io/KeySteer/modes/window) · [在线模拟器](https://dccif.github.io/KeySteer/editor/)

## 个性化改动

基于 [dccif/KeySteer v0.10.16](https://github.com/dccif/KeySteer/releases/tag/v0.10.16) 修改，增加焦点优先的重叠窗口组快速切换。原项目及原作者：[dccif/KeySteer](https://github.com/dccif/KeySteer)。

```toml
x = "window_overlap_next_2"
c = "window_overlap_previous_2"
```

仓库内的 [`keysteer.user.toml`](keysteer.user.toml) 已包含这两个绑定。

### 切换规则

#### 1. 从哪个窗口开始

按下 `X` 或 `C` 时，优先使用当前获得焦点的窗口来确定重叠窗口组。这样即使鼠标仍停在旧窗口组，只要焦点已经切到另一组，就会在新焦点所在的窗口组中切换。

如果当前焦点窗口无法识别，才退回使用鼠标光标下面的窗口。

它不是按照屏幕位置从左到右、从上到下切换，而是在当前焦点所属的重叠窗口组内循环。

#### 2. 怎样算同一组窗口

窗口之间存在有效交叉区域时，会被认为属于同一个重叠窗口组。窗口只覆盖一部分也可以识别；完全被其他窗口遮住的普通窗口，只要没有最小化，通常也能识别。

目前采用连锁关联：如果 `A` 与 `B` 重叠、`B` 与 `C` 重叠，那么 `A、B、C` 会被视为同一组，即使 `A` 与 `C` 没有直接重叠。

#### 3. `X` 和 `C` 的切换顺序

第一次识别窗口组时，会按照 Windows 当前的窗口前后层级建立固定顺序。

例如当前层级为 `1、2、3`，其中 `1` 在最前面：

```text
X：1 → 2 → 3 → 1
C：1 → 3 → 2 → 1
```

程序会保留第一次得到的循环顺序，避免窗口激活后层级发生变化，导致只在两个窗口之间来回跳。

#### 4. 每次切换会发生什么

1. 根据当前焦点确定窗口组。
2. 找到组内的下一个或上一个目标窗口。
3. 将目标窗口激活并拉到最前面。
4. 把鼠标移动到目标窗口的正中心。
5. 下一次从这个新窗口继续循环。

#### 5. 不会加入切换的窗口

- 已经最小化的窗口
- 不与当前窗口组关联的窗口
- 无法识别或无法激活的系统内部窗口、子控件和工具浮层
- 已经关闭的窗口

## 演示视频

https://github.com/user-attachments/assets/26f906d1-3c07-47b9-a889-b0d4186be9ed
