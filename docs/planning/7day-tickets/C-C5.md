# C5 · button-led example

## 任务

新建 `examples/button-led/`,按钮接 D2,LED 接 D13。按一下 LED 翻转 (toggle)。

依赖 S2 (Button bug 修复) 完成,A1 提供数字引脚。

## 允许动的文件

- 新增 `examples/button-led/README.md`
- 新增 `examples/button-led/moxin.toml`
- 新增 `examples/button-led/firmware/platformio.ini`
- 新增 `examples/button-led/firmware/src/main.cpp`

## 验收

```powershell
moxin run examples/button-led
# 切 INTERACT 模式,按 space 模拟按钮,LED 状态翻转
# 再按 space,LED 再翻转
```

README 含：按钮硬件简介 (4 脚按钮的"对角连通"特性)、为什么需要上拉电阻 (用 INPUT_PULLUP 模式)、防抖说明 (不做防抖,说明这是有意为之让用户看到原生行为)。

## 约束

- 固件用 `pinMode(BTN, INPUT_PULLUP)`,不外接上拉电阻
- 用 edge detection (检测 HIGH→LOW 跳变) 而不是 level 检测
- 不做软件防抖 (Phase 2,且能引出"为什么需要 assert DSL" 的话题)

## commit message

`example(C5): button-led 按钮翻转 LED demo`
