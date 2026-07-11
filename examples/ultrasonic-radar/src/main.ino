// ultrasonic-radar:HC-SR04 测距,串口打印
// 验证 ultrasonic 元件:bridge 按注入距离(默认 50cm)生成 echo 回波
const int TRIG_PIN = 7;
const int ECHO_PIN = 8;

void setup() {
  Serial.begin(9600);
  pinMode(TRIG_PIN, OUTPUT);
  pinMode(ECHO_PIN, INPUT);
  Serial.println("ultrasonic-radar ready");
}

void loop() {
  digitalWrite(TRIG_PIN, LOW);
  delayMicroseconds(5);
  digitalWrite(TRIG_PIN, HIGH);
  delayMicroseconds(10);
  digitalWrite(TRIG_PIN, LOW);

  long echo_us = pulseIn(ECHO_PIN, HIGH, 60000UL); // 60ms 超时 ≈ 400cm+
  long cm = echo_us / 58;
  Serial.print("cm=");
  Serial.println(cm);
  delay(300);
}
