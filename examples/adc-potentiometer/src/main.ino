// adc-potentiometer:读 A0 电位器,串口打印原始值和百分比
// 验证 MoXin 的 ADC 注入通道:TUI 里 Tab 聚焦 pot1 后 ←/→ 转旋钮,
// 或 REPL 里 `adc A0 512`,固件读数随之变化
const int POT_PIN = A0;

void setup() {
  Serial.begin(9600);
  Serial.println("adc-potentiometer ready");
}

void loop() {
  int raw = analogRead(POT_PIN);           // 0..1023
  int pct = (int)((long)raw * 100 / 1023); // 0..100
  Serial.print("A0=");
  Serial.print(raw);
  Serial.print(" (");
  Serial.print(pct);
  Serial.println("%)");
  delay(200);
}
