#!/usr/bin/env python3
"""MCP server 端到端冒烟:模拟一个 AI Agent 通过 stdio 驱动 MoXin。

用法:  scripts/mcp_smoke.py <moxin-binary> <project-dir>

流程(adc-potentiometer 场景):
  initialize → build → run → 轮询 sim_state 到 ready → inject adc A0=800
  → 再读 sim_state,断言快照里 adc["0"] == 800。
覆盖 v3 M2 的"AI 编译→跑→注入→读状态"闭环。需要 simavr(CI verify job 里跑)。

退出码:0 = 通过,1 = 失败。stdlib only。
"""
import json
import subprocess
import sys
import time


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: mcp_smoke.py <moxin-binary> <project-dir>", file=sys.stderr)
        return 2
    moxin, proj = sys.argv[1], sys.argv[2]

    p = subprocess.Popen(
        [moxin, "mcp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        bufsize=1,
    )
    next_id = [0]

    def call(method, params):
        next_id[0] += 1
        rid = next_id[0]
        p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": rid, "method": method, "params": params}) + "\n")
        p.stdin.flush()
        line = p.stdout.readline()
        if not line:
            raise RuntimeError("mcp server closed stdout")
        return json.loads(line)

    def tool(name, args):
        resp = call("tools/call", {"name": name, "arguments": args})
        result = resp.get("result", {})
        text = result.get("content", [{}])[0].get("text", "")
        return result.get("isError", False), text

    try:
        # 握手
        r = call("initialize", {})
        assert r["result"]["serverInfo"]["name"] == "moxin", r

        # 编译
        err, text = tool("build", {"path": proj})
        if err:
            print(f"build failed: {text}", file=sys.stderr)
            return 1

        # 启动仿真
        err, text = tool("run", {"path": proj})
        if err:
            print(f"run failed: {text}", file=sys.stderr)
            return 1

        # 轮询 sim_state 到 ready(最多 5 秒)
        ready = False
        for _ in range(50):
            err, text = tool("sim_state", {})
            if not err:
                state = json.loads(text)
                if state.get("ready"):
                    ready = True
                    break
            time.sleep(0.1)
        if not ready:
            print("sim never became ready", file=sys.stderr)
            return 1

        # 注入 ADC A0 = 800,再读回快照断言
        err, text = tool("inject", {"kind": "adc", "channel": 0, "value": 800})
        if err:
            print(f"inject failed: {text}", file=sys.stderr)
            return 1
        time.sleep(0.3)
        err, text = tool("sim_state", {})
        state = json.loads(text)
        got = state.get("adc", {}).get("0")
        if got != 800:
            print(f"expected adc[0]=800, got {got} (state adc={state.get('adc')})", file=sys.stderr)
            return 1

        # assert tool:对同一个运行中的仿真做串口断言(A0= 行必然打印)
        err, text = tool("assert", {"serial_contains": "A0=", "within": "5s"})
        if err or text.strip() != "PASS":
            print(f"assert tool failed: isError={err} verdict={text!r}", file=sys.stderr)
            return 1

        tool("stop", {})
        print("mcp e2e ok: build → run → inject adc=800 → sim_state confirms → assert PASS")
        return 0
    finally:
        try:
            p.stdin.close()
        except Exception:
            pass
        p.terminate()
        try:
            p.wait(timeout=3)
        except Exception:
            p.kill()


if __name__ == "__main__":
    sys.exit(main())
