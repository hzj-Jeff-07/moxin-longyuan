# serial-echo

Arduino Uno demo：串口回显。从 Serial 收到的每个字符都回显为 `echo: <c>`，
同时翻转 D13 LED 作为 RX 活动指示灯。

## 接线

```
D13 ──── LED (green)  anode
GND ──── LED          cathode
```

## 行为

| 输入 | 行为 |
|------|------|
| 任意字符 | 回显 `echo: <字符>`，D13 LED 翻转一次 |

## 运行

```bash
cd examples/serial-echo
moxin build           # arduino-cli 编译 src/main.ino
moxin shell --no-tui  # REPL 模式
moxin> run            # 启动 simavr
moxin> send hello     # 注入串口 RX(bridge 按 9600 波特逐字节喂)
moxin> stop
```

TUI 模式下(`moxin shell`)：输入条为空时直接敲字符即注入串口 RX，
Serial Monitor 面板显示 `echo: <字符>`、D13 LED 翻转。

## 断言验证（CI / AI 用）

```bash
moxin assert --serial-contains "echo: Z" --send Z --within 5s
```

## 预期输出

```
serial-echo ready. type a char, it echoes back.
echo: a
echo: b
echo: c
```

## 依赖

需要 `simavr` + `arduino-cli`。`moxin doctor` 缺什么就装什么。
