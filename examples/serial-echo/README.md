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

Phase 1 的 simavr bridge 串口注入是单字符的（没有行缓冲），所以一次输入
一个字符即可看到回显。

## 运行

```bash
cd examples/serial-echo
moxin build           # arduino-cli 编译 src/main.ino
moxin shell           # 进 shell
moxin> run            # 启动 simavr，进 TUI
# 在 Serial Monitor 逐个输入字符 → 看 echo 回显、D13 LED 翻转
moxin> stop
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
