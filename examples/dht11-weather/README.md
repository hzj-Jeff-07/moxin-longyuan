# dht11-weather

Arduino Uno demo: DHT11 温湿度读取（Phase 3 批次 B / v0.7.0）。

## Wiring

```
D2  ──── DHT11 data    ← 单总线
5V  ──── DHT11 vcc
GND ──── DHT11 gnd
```

## 30 秒跑通

```bash
cd examples/dht11-weather
moxin build
moxin shell --no-tui
```

```
moxin> run
moxin> sleep 1500
moxin> env 31 75        # 升温到 31°C / 75%
✓ env = 31°C 75%
```

## Expected Behavior

1. 仿真启动时 moxin 自动把 data=D2 下发给 bridge（stdin `dht D 2`）
2. 固件拉低 data 20ms 后释放，bridge 按 DHT11 真实时序回 40bit
3. 未注入时用默认 25°C/60%：串口打印 `temp=25C hum=60%`
4. `env 31 75` 之后读数变为 `temp=31C hum=75%`；TUI 里 `dht1` 显示 `[DHT11 31°C 75%]`
5. checksum 校验在固件侧做，任何时序错误都会打印 `dht read failed`

## 断言验证（CI / AI 用）

```bash
moxin assert --serial-contains "temp=" --within 5s
```

## 原理

bridge 把应答 + 40bit 预排成 84 个边沿的时间表，用一个自重排的 simavr
cycle timer 逐个注入 data 引脚（0 = 高 27us，1 = 高 70us）——固件的手写
bit-bang 读取按脉宽判 0/1，与真实模块的驱动方式一致。
协议见 `docs/design/bridge-protocol.md`。
