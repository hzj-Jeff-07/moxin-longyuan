#!/bin/bash
# MoXin led-control demo script
# Run: bash docs/demo/demo.sh
cd examples/led-control
moxin build
echo -e "run\nsleep 2000\nstop" | moxin shell --no-tui
