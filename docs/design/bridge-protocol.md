# MoXin Bridge Protocol

Version: 1 (v2a)

## Transport
Bridge process communicates via stdout JSON Lines. Each line is one event.

## Events

### ready
{"event":"ready","mcu":"<id>","freq":<hz>}
Emitted once at startup.

### pin
{"event":"pin","t_us":<us>,"port":"<port>","bit":<n>,"value":0|1}
- AVR: port="B", bit=5 → D13 (PB5)
- STM32: port="GPIO", bit=13 → PA13
Note: "GPIO" is a placeholder for STM32; future boards should use real port names (e.g. "PA").

### serial
{"event":"serial","t_us":<us>,"line":"<escaped>"}
Non-PIN UART output lines. Extensible for future boards.

### exit
{"event":"exit","state":<n>}

## Firmware Convention
STM32 firmware must emit "PIN<n>=<0|1>\n" on USART2 for GPIO events to be detected.
AVR: simavr instruments GPIO directly, no firmware convention needed.

## Extension
Unknown event types must be silently ignored by the Rust side (not panic).
