// ir-remote:D2 收 NEC 红外码,打印并用电源键码翻转 D13
// 手写 NEC 解码(不依赖 IRremote 库),所有等待带超时
const int IR_PIN = 2;
const int LED_PIN = 13;
const unsigned long POWER_CODE = 0x20DF10EFUL; // 自检码兼"电源键"

long waitLevel(int level, unsigned long timeout_us) {
  unsigned long t0 = micros();
  while (digitalRead(IR_PIN) != level) {
    if (micros() - t0 > timeout_us) return -1;
  }
  return (long)(micros() - t0);
}

// 收一帧:成功返回 32 位码,失败/超时返回 0
unsigned long readNEC() {
  if (waitLevel(LOW, 500000UL) < 0) return 0;   // 等引导(最多 0.5s)
  unsigned long t0 = micros();
  if (waitLevel(HIGH, 12000) < 0) return 0;     // 引导低结束
  if (micros() - t0 < 7000) return 0;           // 不足 9ms,不是 NEC 引导
  if (waitLevel(LOW, 6000) < 0) return 0;       // 4.5ms 空结束

  unsigned long code = 0;
  for (int byteIdx = 0; byteIdx < 4; byteIdx++) {
    unsigned char b = 0;
    for (int i = 0; i < 8; i++) {               // 字节内 LSB 先到
      if (waitLevel(HIGH, 1200) < 0) return 0;  // 560us 载波结束
      unsigned long hs = micros();
      if (waitLevel(LOW, 2500) < 0) return 0;   // 空结束(下一载波开始)
      if (micros() - hs > 1000) b |= (1 << i);  // >1ms 空 = 1
    }
    code = (code << 8) | b;
  }
  waitLevel(HIGH, 1200);                        // 尾载波
  return code;
}

void setup() {
  Serial.begin(9600);
  pinMode(LED_PIN, OUTPUT);
  pinMode(IR_PIN, INPUT_PULLUP);
  Serial.println("ir-remote ready");
}

void loop() {
  unsigned long code = readNEC();
  if (code == 0) return;
  char buf[9];
  sprintf(buf, "%08lX", code);
  Serial.print("code=");
  Serial.println(buf);
  if (code == POWER_CODE) {
    digitalWrite(LED_PIN, !digitalRead(LED_PIN));
    Serial.println("power toggled");
  }
}
