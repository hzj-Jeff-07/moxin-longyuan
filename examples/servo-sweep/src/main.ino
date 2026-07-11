// servo-sweep:D9 舵机 0°→180°→0° 来回扫
// 验证 servo 元件:TUI 从 50Hz PWM 脉宽推导角度显示
// 不用 <Servo.h>(避免依赖库下载):手写 50Hz 软 PWM,脉宽 500-2500us
const int SERVO_PIN = 9;

void setup() {
  Serial.begin(9600);
  pinMode(SERVO_PIN, OUTPUT);
  Serial.println("servo-sweep ready");
}

void writeAngle(int deg) {
  // 500 + deg/180 * 2000 us 高脉冲,补足 20ms 周期
  long pulse = 500 + (long)deg * 2000 / 180;
  for (int i = 0; i < 5; i++) { // 每个角度保持 5 个周期(100ms)
    digitalWrite(SERVO_PIN, HIGH);
    delayMicroseconds(pulse);
    digitalWrite(SERVO_PIN, LOW);
    delayMicroseconds(20000 - pulse);
  }
}

void loop() {
  static int deg = 0;
  static int dir = 15;
  writeAngle(deg);
  Serial.print("angle=");
  Serial.println(deg);
  deg += dir;
  if (deg >= 180) { deg = 180; dir = -15; }
  if (deg <= 0)   { deg = 0;   dir = 15; }
}
