# C9 · 录视频 3-5 分钟 + 中英双字幕

## 任务

录一段 3-5 分钟的 demo 视频,讲清楚 moxin 是什么、为什么有用、怎么和 AI 配合。剪辑完后做中英双字幕。

**这是整个 7 天最重要的一票。其它都可以打折,这个不行。**

视频结构 (脚本提前一晚写好):

```
0:00-0:20  开场  这是 MoXin (一句话定位 + 屏幕展示 TUI)
0:20-1:00  痛点  传统 Arduino 仿真为什么不好 (假代码截图 / 文字描述)
1:00-2:30  Demo  跑 examples/pot-led-brightness,展示交互模式
2:30-4:00  AI 协作  从 C8 session-log 剪一段精华:bug → AI 修
4:00-4:30  Outro  GitHub 地址 + cargo install 命令
```

录制：OBS + 摄像头不出镜,只录屏 + 旁白。

剪辑：剪映 / Premiere / DaVinci 任选,导出 1080p mp4。

字幕：中文是旁白逐字稿,英文用机翻 + 人工校对 (`baoyu-translate` 可辅助)。

## 允许动的文件

- 新增 `docs/demo/video-zh.mp4`(中文旁白版)
- 新增 `docs/demo/video-en.mp4`(英文配音或硬字幕版,可选,优先做中文版)
- 新增 `docs/demo/subtitles-zh.srt`
- 新增 `docs/demo/subtitles-en.srt`
- 新增 `docs/demo/video-script.md`(脚本,留档)

## 验收

```powershell
Test-Path docs/demo/video-zh.mp4
Test-Path docs/demo/subtitles-zh.srt
Test-Path docs/demo/subtitles-en.srt
# 视频长度 3-5 分钟
# 字幕条数对得上 (中英行数相同)
(Get-Content docs/demo/subtitles-zh.srt | Select-String "^\d+$").Count -eq (Get-Content docs/demo/subtitles-en.srt | Select-String "^\d+$").Count
```

## 约束

- 视频文件大,push 前确认 .gitattributes / .gitignore (建议走 GitHub Release 或 LFS,不直接进 main)
- 录屏期间 A/B 窗口禁止 push 到 main (避免 `git log` 截图过期)
- 旁白讲技术,不讲商业话术
- 如果 5 天里没全做完 example,视频里只展示已经做完的 example,不要硬塞

## commit message

`demo(C9): 路演视频 + 中英双字幕`
