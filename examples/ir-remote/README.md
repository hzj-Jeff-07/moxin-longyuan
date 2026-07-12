# ir-remote

Arduino Uno demo: NEC 红外遥控（Phase 3 批次 B / v0.7.0）。

## Wiring

```
D2  ──── VS1838 out    ← 解调后 NEC 波形(空闲高)
D13 ──── LED (blue) anode
5V/GND ── VS1838 供电
```

## 30 秒跑通

```bash
cd examples/ir-remote
moxin build
moxin shell --no-tui
```

```
moxin> run              # 500ms 后自动收到自检帧
moxin> sleep 1500
moxin> ir 20DF10EF      # 再按一次"电源键"
✓ ir 20DF10EF
```

## Expected Behavior

1. 仿真启动时 moxin 自动把 out=D2 下发给 bridge，500ms 后 bridge 自发一帧自检码
2. 固件手写 NEC 解码打印 `code=20DF10EF`；电源键码会翻转 D13 并打印 `power toggled`
3. `ir <hex>` 可发任意 32 位码；TUI 里 `ir1` 显示最近一帧 `[IR 20DF10EF]`
4. 帧按真实 NEC 时序回放：9ms 引导 + 4.5ms 空 + 32bit（560us 载波 + 560/1690us 空），字节内 LSB 先发

## 断言验证（CI / AI 用）

```bash
moxin assert --serial-contains "code=20DF10EF" --within 5s   # 靠自检帧
```
