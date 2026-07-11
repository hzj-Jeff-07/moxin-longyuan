// rgb-rainbow:RGB LED 三通道相位错开呼吸,循环混色
// 验证 rgb_led 元件:TUI 里色块随三路 PWM duty 实时混色
const int R_PIN = 9;
const int G_PIN = 10;
const int B_PIN = 11;

void setup() {
  Serial.begin(9600);
  pinMode(R_PIN, OUTPUT);
  pinMode(G_PIN, OUTPUT);
  pinMode(B_PIN, OUTPUT);
  Serial.println("rgb-rainbow ready");
}

// 0-765 的相位 → 单通道三角波(0..255..0)
int tri(int phase) {
  phase %= 766;
  if (phase < 256) return phase;
  if (phase < 511) return 510 - phase;
  return 0;
}

void loop() {
  static int t = 0;
  int r = tri(t);
  int g = tri(t + 255);
  int b = tri(t + 510);
  analogWrite(R_PIN, r);
  analogWrite(G_PIN, g);
  analogWrite(B_PIN, b);
  Serial.print("rgb=");
  Serial.print(r); Serial.print(",");
  Serial.print(g); Serial.print(",");
  Serial.println(b);
  t += 10;
  delay(60);
}
