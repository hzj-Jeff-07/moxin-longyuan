# led-control

Arduino Uno demo: 2 LEDs + 1 button + serial command interface.

## Wiring

```
D13 ──── LED (red)    anode
D12 ──── LED (green)  anode
D2  ──── Button       pin A
GND ──── LED cathodes + Button pin B
```

## Serial Commands

| Command | Action |
|---------|--------|
| `r` | Toggle red LED (D13) |
| `g` | Toggle green LED (D12) |
| `s` | Print status: `R=<0|1> G=<0|1>` |
| `?` | Print help |

## Expected Behavior

1. `moxin build` compiles the sketch
2. `moxin shell` → `run` starts simavr
3. In Serial Monitor, type `r` → red LED toggles, firmware prints `LED_R=1`
4. Type `g` → green LED toggles
5. Button press (simulated via firmware) prints `BTN=1`

## Run

```bash
cd examples/led-control
moxin build           # arduino-cli compiles src/main.ino
moxin shell           # enter shell
moxin> run            # launch simavr, enter TUI
# In Serial Monitor: type r / g / s / ?
moxin> stop
```

## Limitations (v2b)

- Button simulation: physical button press not yet wired to simavr GPIO input
- Serial RX injection: single-char commands only (no line buffering)
- STM32 board: button events rely on firmware self-reporting, not real GPIO simulation

## Dependencies

Requires `simavr` + `arduino-cli`. Run `moxin doctor` to check.
