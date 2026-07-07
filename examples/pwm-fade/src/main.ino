// pwm-fade:D9 上的 LED 0→255→0 呼吸灯
// 验证 MoXin 的 PWM 追踪:TUI 中 led1 显示占空比百分比而不是 ON/OFF
const int LED_PIN = 9; // Timer1 OC1A,analogWrite 走真硬件 PWM(约 490Hz)

void setup() {
  Serial.begin(9600);
  pinMode(LED_PIN, OUTPUT);
  Serial.println("pwm-fade ready");
}

void loop() {
  static int duty = 0;
  static int dir = 5;
  analogWrite(LED_PIN, duty);
  Serial.print("duty=");
  Serial.println(duty);
  duty += dir;
  if (duty >= 255) { duty = 255; dir = -5; }
  if (duty <= 0)   { duty = 0;   dir = 5; }
  delay(50);
}
