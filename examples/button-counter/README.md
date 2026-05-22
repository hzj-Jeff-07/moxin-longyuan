# button-counter

Arduino Uno demo：按一次按钮 → 计数 +1，D13 LED 翻转，串口打印 `COUNT=<n>`。

## 接线

```
D13 ──── LED   (red)   anode
D2  ──── Button         pin A   (INPUT_PULLUP)
GND ──── LED cathode + Button pin B
```

## Serial 命令

Phase 1 simavr bridge 还没接真实 GPIO 输入注入，所以"按按钮"用 TUI Serial
Monitor 输入字符模拟：

| 输入 | 行为 |
|------|------|
| `b` | 模拟一次按钮按下 → COUNT++ + LED 翻转 |
| `s` | 打印当前 COUNT |
| `r` | 重置 COUNT 为 0 |
| `?` | 打印命令帮助 |

等后续 ticket 加上真实 GPIO 输入注入后，D2 上的物理 button 事件会自动算
进 COUNT，firmware 代码不用改。

## 运行

```bash
cd examples/button-counter
moxin build           # arduino-cli 编译 src/main.ino
moxin shell           # 进 shell
moxin> run            # 启动 simavr，进 TUI
# 在 Serial Monitor 输入 b b b → 看 COUNT 递增、LED 状态翻转
moxin> stop
```

## 预期输出

```
button-counter ready. press 'b' to simulate button.
COUNT=1
LED=1
COUNT=2
LED=0
COUNT=3
LED=1
```

## 依赖

需要 `simavr` + `arduino-cli`。`moxin doctor` 缺什么就装什么。
