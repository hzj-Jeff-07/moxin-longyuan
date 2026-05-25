// assert-blink-toggles — Arduino Uno
//
// 最小 blink 程序，专门给 `moxin assert --pin D13 --toggles --within 3s` 验证用。
// loop 里每 500ms 翻转一次 D13，3 秒窗口内必然能观察到至少一次翻转 → exit 0。
//
// 故意做坏验证：把下面的 digitalWrite(LED, HIGH) 注释掉，再跑 assert，
// 应得到 exit 2 (TIMEOUT)，证明断言能抓到"灯不闪"的回归。

const int LED = 13;

void setup() {
  pinMode(LED, OUTPUT);
}

void loop() {
  digitalWrite(LED, HIGH);
  delay(500);
  digitalWrite(LED, LOW);
  delay(500);
}
