# C4 · pot-led-brightness example

## 任务

新建 `examples/pot-led-brightness/`,电位器接 A0,LED 接 D9 (PWM 引脚)。固件读 A0 模拟值,映射成 0-255 PWM,控制 LED 亮度。

依赖 A3 (ADC) + A5 (电位器) 完成。

## 允许动的文件

- 新增 `examples/pot-led-brightness/README.md`
- 新增 `examples/pot-led-brightness/moxin.toml`
- 新增 `examples/pot-led-brightness/firmware/platformio.ini`
- 新增 `examples/pot-led-brightness/firmware/src/main.cpp`

## 验收

```powershell
moxin run examples/pot-led-brightness
# 启动后 TUI 切 INTERACT 模式,选中 pot1,按 ] ] ] 让位置=15%
# led1.brightness 跟随变化,大约 38 左右 (15% of 255)
```

README 必须教用户：怎么进 INTERACT 模式、怎么调电位器、怎么观察 LED 亮度变化。这个 example 是 TUI 交互演示的招牌,README 要写清楚。

## 约束

- 固件用 `map()` 而不是手算 (展示 Arduino 风格)
- 不要加防抖、滤波 (留 Phase 2)
- PWM 引脚必须用 D9 (UNO 上是 OC1A,标准教程都用它)

## commit message

`example(C4): pot-led-brightness 电位器调光 demo`
