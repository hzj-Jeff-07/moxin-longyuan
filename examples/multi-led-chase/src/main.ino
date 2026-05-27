// multi-led-chase — Arduino Uno
//
// D2..D7 上的 6 颗 LED 走马灯,每 200ms 移动一次,循环往返。
// 用来验证 Phase 2-mini Step 1 (bridge 全 PORTD GPIO hook) +
// Step 3 (render 接全 GPIO 真状态)。
//
// 6 颗 LED 全在 PORTD bit 2-7,bridge 应能逐个翻转事件;
// `moxin show` TUI 里 6 颗 LED 应轮流 ON。

const uint8_t LEDS[6] = {2, 3, 4, 5, 6, 7};
int8_t pos = 0;
int8_t dir = 1;

void setup() {
  Serial.begin(9600);
  for (uint8_t i = 0; i < 6; i++) {
    pinMode(LEDS[i], OUTPUT);
    digitalWrite(LEDS[i], LOW);
  }
  Serial.println("multi-led-chase ready. 6 LEDs on D2..D7.");
}

void loop() {
  for (uint8_t i = 0; i < 6; i++) {
    digitalWrite(LEDS[i], i == pos ? HIGH : LOW);
  }
  Serial.print("POS=");
  Serial.println(pos);
  pos += dir;
  if (pos == 5 || pos == 0) dir = -dir;
  delay(200);
}
