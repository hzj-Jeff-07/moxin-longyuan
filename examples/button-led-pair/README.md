# button-led-pair

Arduino Uno demo:Button 控 D4 LED(故意挑非 D13)。

**验证目标**:Phase 2-mini Step 3 — 非 D13 LED 也跟真 GPIO 走,
不再退化成静态 OFF。是任务书"只有 D13 真实仿真"红线被拔掉的实证。

## 接线

```
D2  ──── Button     pin A   (INPUT_PULLUP)
D4  ──── LED        anode   (green)
GND ──── LED cathode + Button pin B
```

## Serial 命令

Phase 1 现实:simavr bridge 还没接真实 GPIO 输入注入,所以"按按钮"
用 Serial Monitor 输入字符模拟:

| 输入 | 行为 |
|------|------|
| `b` | 模拟按键 → 翻转 D4 LED |
| `s` | 打印当前 LED 状态 |
| `?` | 帮助 |

## 运行

```bash
cd examples/button-led-pair
moxin build
moxin shell
moxin> run            # 启动 simavr,进 TUI
# Serial Monitor 输入 b → D4 LED 切换 ON/OFF
# Inspector 里 D4 行真切换,不再永远 OFF
moxin> stop
```

## 预期行为

```
> b
LED=1
# Inspector: D4 ━━━━━━━━━━━━━━━━━━━━━━ ● led1 [GRE ON ]
> b
LED=0
# Inspector: D4 ━━━━━━━━━━━━━━━━━━━━━━ ○ led1 [GRE OFF]
```

## AI 接口验证

```bash
moxin status --pin D4    # HIGH/LOW 跟着翻转
moxin status --pin D13   # 应保持 LOW(没用到)
```

## 依赖

需要 `simavr` + `arduino-cli`。`moxin doctor` 缺什么就装什么。
