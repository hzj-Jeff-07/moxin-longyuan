// ldr-nightlight:光敏小夜灯
// A0 读环境光,低于阈值点亮 D13;验证 photoresistor 元件 + ADC 注入
const int LDR_PIN = A0;
const int LED_PIN = 13;
const int DARK_THRESHOLD = 300; // 低于此值视为"天黑"

void setup() {
  Serial.begin(9600);
  pinMode(LED_PIN, OUTPUT);
  Serial.println("ldr-nightlight ready");
}

void loop() {
  int light = analogRead(LDR_PIN);
  int dark = light < DARK_THRESHOLD;
  digitalWrite(LED_PIN, dark ? HIGH : LOW);
  Serial.print("light=");
  Serial.print(light);
  Serial.println(dark ? " (dark, LED on)" : " (bright, LED off)");
  delay(300);
}
