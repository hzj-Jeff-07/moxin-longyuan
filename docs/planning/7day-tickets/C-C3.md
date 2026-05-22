# C3 · counter-7seg example

## 任务

新建 `examples/counter-7seg/`,7 段数码管 a-g+dp 接 D2-D9,共阴极接 GND。固件每秒计数 0-9 循环显示。

依赖 A4 (7 段数码管仿真) 完成。

## 允许动的文件

- 新增 `examples/counter-7seg/README.md`
- 新增 `examples/counter-7seg/moxin.toml`
- 新增 `examples/counter-7seg/firmware/platformio.ini`
- 新增 `examples/counter-7seg/firmware/src/main.cpp`

## 验收

```powershell
moxin run examples/counter-7seg
# TUI 看到 seg1.display_char 依次 "0" "1" "2" ... "9" "0" 循环,每秒一变
```

README 含：硬件清单 (1x 共阴极 7 段数码管 + 7x 220Ω 电阻 + 杜邦线)、接线表 (a/b/c/d/e/f/g 对应 D2-D8,dp 接 D9)、字符段位查表 (常识但写出来方便用户参考)。

## 约束

- 用查表法把 0-9 转成 8 位段位,不要 if-else 写 10 个分支
- 固件含必要注释,因为有人会看着学
- 计数到 9 之后回 0,不要继续到 A-F (那是 hex,留 Phase 2)

## commit message

`example(C3): counter-7seg 数码管计数 demo`
