# C2 · buzzer-tone example

## 任务

新建 `examples/buzzer-tone/`,蜂鸣器接 D9,固件代码用 `tone()` 播一段简单旋律 (do-re-mi)。

依赖 A2 (蜂鸣器仿真) 完成。

## 允许动的文件

- 新增 `examples/buzzer-tone/README.md`
- 新增 `examples/buzzer-tone/moxin.toml`
- 新增 `examples/buzzer-tone/firmware/platformio.ini`
- 新增 `examples/buzzer-tone/firmware/src/main.cpp`

## 验收

```powershell
moxin run examples/buzzer-tone
# TUI 看到 frequency_hz 依次变成 262 (do) / 294 (re) / 330 (mi),每个持续 500ms
```

README 含：硬件清单 (有源蜂鸣器 + 杜邦线)、接线表、注意事项 ("有源蜂鸣器有正负极")。

## 约束

- 固件只用 Arduino 标准库 tone() / noTone(),不自己写 PWM
- 频率值用宏定义 NOTE_C4 / NOTE_D4 / NOTE_E4
- 不要播太长的曲子,3-5 个音符够

## commit message

`example(C2): buzzer-tone 蜂鸣器旋律 demo`
