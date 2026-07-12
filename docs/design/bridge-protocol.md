# MoXin Bridge Protocol

## Current State (v0.5.0 / Phase 2-full Step 2)
Protocol version is announced via the `hello` event (AVR bridge, protocol "1").
Bridges that predate `hello` (STM32, old AVR builds) have no version field —
the Rust side treats them as capability-less and refuses `adc` injection.
Unknown event types are silently ignored by the Rust side.

## Transport
- **stdout**: JSON Lines, one event per line.
- **stdin** (AVR bridge, protocol ≥1): line-based command channel, polled
  non-blocking between `avr_run` chunks (single-threaded — simavr is not
  thread-safe). Unrecognized lines are ignored.

## stdin Commands (AVR bridge)

### adc
```
adc <channel> <value>
```
- channel: 0..7 (Uno exposes ADC0..ADC5 = A0..A5; Nano adds 6/7 = A6/A7)
- value: 0..1023 (10-bit raw; clamped). Bridge converts to mV against
  AVCC=5000mV and raises the simavr ADC IRQ, then echoes an `adc` event.

### sr04 (v0.6.0)
```
sr04 <trig_port> <trig_bit> <echo_port> <echo_bit>
```
- ports: B/C/D, bits 0..7. Declares the HC-SR04 wiring; sent automatically
  by moxin right after spawn based on the project's `ultrasonic` component
  wires. After configuration, a >=2us high pulse on the trigger pin
  schedules an echo pulse via simavr cycle timers: high after ~200us,
  low after another 58us x distance_cm (the real module's formula).

### dist (v0.6.0)
```
dist <cm>
```
- 2..400 (clamped). Sets the simulated obstacle distance; default 50.
  No echo event — moxin records the value locally for rendering.

### dht (v0.7.0)
```
dht <port> <bit>
```
- Declares the DHT11 data pin (B/C/D, 0..7); sent automatically by moxin
  based on the project's `dht11` component wires. Once configured, a
  host-driven low of >=500us followed by release triggers the DHT11
  response waveform (edge player over cycle timers): 80us low + 80us high
  ack, then 40 bits (50us low lead + 27us/70us high = 0/1), byte order
  hum / 0 / temp / 0 / checksum.

### env (v0.7.0)
```
env <temp_c> <hum_pct>
```
- temp 0..50, hum 20..90 (clamped; DHT11 range). Default 25/60.
  Echoed back as a `dht` event.

### ir (v0.7.0)
```
ir <port> <bit>
```
- Declares the IR receiver output pin (auto-sent by moxin from
  `ir_receiver` component wires). 500ms after configuration the bridge
  plays one self-test frame (code 20DF10EF) so first-run and CI e2e see
  a decode without manual injection.

### lcd (v0.7.0)
```
lcd <hex_addr>
```
- Enables the LCD1602 I2C slave (PCF8574 backpack) at the given 7-bit
  address (0x08..0x77; conventionally 27). Auto-sent by moxin when the
  project has an `lcd1602` component. Until enabled the bridge ACKs no
  I2C address, so old firmware is unaffected. The slave decodes the
  PCF8574→HD44780 4-bit protocol (P0=RS, P2=EN latch on falling edge,
  P4-7=data; single-nibble init handled) and emits throttled `lcd`
  events. Read transfers are not ACKed (backpack is write-only here).

### serial (v0.7.0)
```
serial <text>
```
- Injects `<text>` into the firmware's UART RX one byte at a time, paced
  at ~9600 baud (simavr's UDR holds one byte, so a same-tick burst would
  be overwritten). The payload is the rest of the line — spaces kept,
  no trailing newline added. Firmware `Serial.read()` receives the bytes.
  Sent by `moxin send <text>`, `assert --send <text>`, and TUI keystrokes
  when the input bar is empty.

### oled (v0.7.0)
```
oled <hex_addr>
```
- Enables the SSD1306 OLED I2C slave at the given 7-bit address
  (conventionally 3C). Auto-sent by moxin when the project has an
  `oled_ssd1306` component. The slave parses the control byte (bit6=D/C#)
  to split command vs data streams, tracks the horizontal-addressing
  window (0x21 column, 0x22 page), writes data bytes into a 128x64
  framebuffer (col auto-increment, page wrap), and emits throttled
  `oled` events with a braille-downsampled frame. Read transfers are not
  ACKed.

### irtx (v0.7.0)
```
irtx <hex32>
```
- Plays one NEC frame on the declared pin: 9ms leader low + 4.5ms space,
  32 bits (560us burst + 560us/1690us space = 0/1), 560us stop burst.
  Bytes transmitted high-byte-first, bits within a byte LSB-first (NEC
  convention). Echoed back as an `ir` event. Dropped if the edge player
  is busy (e.g. mid-DHT-read).

## Events

### hello (protocol ≥1, AVR bridge)
{"event":"hello","protocol":"1","capabilities":["adc","serial","serialrx","sr04","dht","ir","lcd","oled"]}
Emitted once, before `ready`. Rust side stores capabilities;
`RunningSim::set_adc` refuses when "adc" is absent (old bridge → clear error
instead of a silently dropped command).

### ready
{"event":"ready","mcu":"<id>","freq":<hz>}
Emitted once at startup.

### pin
{"event":"pin","t_us":<us>,"port":"<port>","bit":<n>,"value":0|1}
- AVR: 全 PORTB / PORTC / PORTD 引脚均 hook(Phase 2-mini Step 1 起):
  - port="B" bit=0..5 → D8-D13 (PB0-PB5)
  - port="C" bit=0..5 → A0-A5  (PC0-PC5)
  - port="D" bit=0..7 → D0-D7  (PD0-PD7)
- STM32: port="GPIO", bit=13 → PA13 (bridge-stm32 convention)

### serial
{"event":"serial","t_us":<us>,"line":"<escaped>"}
- STM32: non-PIN UART output lines (since v0.2).
- AVR: UART0 output, line-buffered, hooked via UART_IRQ_OUTPUT (since
  protocol 1). simavr's default UART→stdout dump is disabled so raw text
  cannot pollute the JSON Lines stream. Before protocol 1 the AVR bridge
  never emitted serial events — Uno serial output was silently lost.

### adc (protocol ≥1, AVR bridge)
{"event":"adc","t_us":<us>,"channel":<0..7>,"value":<0..1023>}
Echo of a processed stdin `adc` command. Rust side updates
`RunState.adc_values[channel]`.

### dht (v0.7.0, AVR bridge)
{"event":"dht","t_us":<us>,"temp":<0..50>,"hum":<20..90>}
Echo of a processed stdin `env` command. Rust side updates
`RunState.dht_env`.

### ir (v0.7.0, AVR bridge)
{"event":"ir","t_us":<us>,"code":<u32>}
Emitted when an NEC frame is played (irtx command or self-test frame).
Rust side updates `RunState.ir_code`.

### lcd (v0.7.0, AVR bridge)
{"event":"lcd","t_us":<us>,"row0":"<16 chars>","row1":"<16 chars>"}
Visible 16x2 window of the HD44780 DDRAM, emitted at most every ~30ms
while dirty (per-nibble I2C transactions would flood otherwise).
Rust side updates `RunState.lcd`.

### oled (v0.7.0, AVR bridge)
{"event":"oled","t_us":<us>,"rows":["<64 braille>", ...16 rows]}
Braille-downsampled 128x64 framebuffer (2x4 px per cell → 64 cols x 16
rows), emitted at most every ~40ms while dirty. Rust side updates
`RunState.oled`.

### button
{"event":"button","t_us":<us>,"pressed":true|false}
Optional. Emitted when firmware self-reports button state (e.g. BTN=1 on UART).
Rust side updates RunState.button_pressed on receipt.

### exit
{"event":"exit","state":<n>}

## Notes
- STM32 bridge uses "GPIO" as port placeholder; real port names (PA/PB/PC) are not yet used.
- STM32 firmware must emit "PIN<n>=<0|1>\n" on USART2 for GPIO events to be detected.
- AVR: simavr instruments GPIO directly, no firmware convention needed.
- Serial RX injection (AVR bridge, v0.7.0): the `serial <text>` stdin command
  feeds bytes into the firmware UART RX (see above). TUI keystrokes now route
  through it, so interactive examples (serial-echo, led-control) actually work.
  STM32 bridge still passes qemu stdin to /dev/null (no RX injection there).
- PWM has no bridge event: duty/freq are derived Rust-side from `pin` edge
  timing (see phase-2-full RFC Step 3).

## 三处同步规则
加新事件类型 = 同步改 `sim.rs::BridgeEvent` enum + bridge C 源码 + 本文档,
三处一起改否则丢事件。
