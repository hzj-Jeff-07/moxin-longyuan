# C1 · multi-led example + RunState 扩展

## 任务

新建 `examples/multi-led/`,8 颗 LED 接 D2-D9,固件代码做跑马灯。

依赖 A1 (14 数字引脚仿真) 完成。

## 允许动的文件

- 新增 `examples/multi-led/README.md`
- 新增 `examples/multi-led/moxin.toml`
- 新增 `examples/multi-led/firmware/platformio.ini`
- 新增 `examples/multi-led/firmware/src/main.cpp`
- 不动 src/

## 验收

```powershell
moxin run examples/multi-led
# TUI 看到 8 颗 LED 按 250ms 间隔顺序点亮
```

README 必须含：硬件清单 (8x LED + 8x 220Ω 电阻 + 面包板 + 杜邦线)、接线表格 (D2→led1.anode, ..., D9→led8.anode)、运行命令、预期现象。

## 约束

- 固件代码不超过 50 行 (跑马灯就是 for 循环 + digitalWrite)
- LED 实例名 led1-led8
- 不引入额外仿真功能,纯用 A1 提供的能力

## commit message

`example(C1): multi-led 8 颗跑马灯`
