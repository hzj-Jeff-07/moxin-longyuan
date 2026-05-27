# pin-state-snapshot

Arduino Uno demo:setup() 把 D2..D12 一次性翻成固定棋盘模式
(偶数 HIGH / 奇数 LOW),loop() 静默,供 AI 接口批量查询。

**验证目标**:Phase 2-mini Step 4(`moxin status --pin` 全引脚可查)
— firmware 翻过一次的引脚应**全部**能从 status 命令拿到 HIGH/LOW,
不会退化为 UNKNOWN。

## 接线

```
D2  ─── led_d2  (red anode)
D3  ─── led_d3  (red anode)
D4  ─── led_d4  (red anode)
...
D12 ─── led_d12 (red anode)
GND ── 各 LED cathode (共地)
```

11 颗 LED,接 D2..D12。

## 预期状态

setup() 跑完后:

| 引脚 | 期望 |
|------|------|
| D2   | HIGH |
| D3   | LOW  |
| D4   | HIGH |
| D5   | LOW  |
| D6   | HIGH |
| D7   | LOW  |
| D8   | HIGH |
| D9   | LOW  |
| D10  | HIGH |
| D11  | LOW  |
| D12  | HIGH |

## 运行 + AI 接口验证

```bash
cd examples/pin-state-snapshot
moxin build
moxin shell
moxin> run            # 启动 simavr;看到 "ready"/"PATTERN" 后再:
moxin> stop           # firmware 已把 GPIO 翻好,状态写入 build/.moxin-state.json

# 退出 TUI 后,逐个查所有引脚:
moxin status --pin D2     # HIGH
moxin status --pin D3     # LOW
moxin status --pin D7     # LOW
moxin status --pin D8     # HIGH
moxin status --pin D12    # HIGH

# 一键验证棋盘模式(bash):
for n in 2 3 4 5 6 7 8 9 10 11 12; do
  printf "D%d: %s\n" $n "$(moxin status --pin D$n)"
done
```

## 预期串口输出

```
pin-state-snapshot ready. D2..D12 set, idling.
PATTERN: even=HIGH odd=LOW
HEARTBEAT
HEARTBEAT
...
```

## 依赖

需要 `simavr` + `arduino-cli`。`moxin doctor` 缺什么就装什么。
