# MoXin Bridge Protocol

## Current State (v2a/v2b)
No version field. Protocol is fixed per bridge binary.
Unknown event types are silently ignored by the Rust side.

## Transport
Bridge process communicates via stdout JSON Lines. Each line is one event.

## Events

### ready
{"event":"ready","mcu":"<id>","freq":<hz>}
Emitted once at startup.

### pin
{"event":"pin","t_us":<us>,"port":"<port>","bit":<n>,"value":0|1}
- AVR: port="B", bit=5 → D13 (PB5)
- STM32: port="GPIO", bit=13 → PA13 (bridge-stm32 convention)

### serial
{"event":"serial","t_us":<us>,"line":"<escaped>"}
Non-PIN UART output lines.

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
- Serial RX injection: tui.rs writes single bytes to bridge stdin; simavr bridge does not
  currently consume stdin (no UART RX loop). STM32 bridge passes stdin to /dev/null.
