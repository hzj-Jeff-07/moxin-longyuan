// pin-state-snapshot — Arduino Uno
//
// setup() 把 D2..D12 翻成固定棋盘模式(偶数引脚=HIGH,奇数=LOW),
// 然后 loop() 静默不动。
//
// 用来验证 Phase 2-mini Step 4 (AI 接口完整性):
// firmware 跑过一次 setup 后,`moxin status --pin Dx` 应能稳定
// 报出 HIGH/LOW(不再退化为 UNKNOWN),覆盖整个 D2..D12 区间。
//
// 棋盘模式:
//   HIGH: D2 D4 D6 D8 D10 D12
//   LOW : D3 D5 D7 D9 D11

void setup() {
  Serial.begin(9600);
  for (uint8_t pin = 2; pin <= 12; pin++) {
    pinMode(pin, OUTPUT);
    // 偶数 HIGH,奇数 LOW
    digitalWrite(pin, (pin % 2 == 0) ? HIGH : LOW);
  }
  Serial.println("pin-state-snapshot ready. D2..D12 set, idling.");
  Serial.println("PATTERN: even=HIGH odd=LOW");
}

void loop() {
  // 静默:每 5s 心跳一次确认还活着,但不动 GPIO
  delay(5000);
  Serial.println("HEARTBEAT");
}
