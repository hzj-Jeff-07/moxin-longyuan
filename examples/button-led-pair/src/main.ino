// button-led-pair — Arduino Uno
//
// 按 'b'(模拟按键)→ 翻转 D4 LED。LED 故意挑非 D13,
// 用来验证 Phase 2-mini Step 3:非 D13 LED 也跟真 GPIO 走,
// 而不是退化成静态 OFF。
//
// Phase 1 现实:simavr bridge 还没接真实 GPIO 输入注入,所以
// 按按钮通过 Serial Monitor 输入 'b' 模拟。等 GPIO 输入注入
// 落地后,D2 上的物理 button 事件会自动激活,firmware 不用改。

const int BTN = 2;
const int LED = 4;

int led_state = LOW;
int last_btn = HIGH;

static void on_press() {
  led_state = !led_state;
  digitalWrite(LED, led_state);
  Serial.print("LED=");
  Serial.println(led_state ? 1 : 0);
}

void setup() {
  Serial.begin(9600);
  pinMode(LED, OUTPUT);
  pinMode(BTN, INPUT_PULLUP);
  digitalWrite(LED, LOW);
  Serial.println("button-led-pair ready. press 'b' to toggle D4 LED.");
}

void loop() {
  if (Serial.available()) {
    char c = Serial.read();
    if (c == 'b') on_press();
    else if (c == 's') {
      Serial.print("LED=");
      Serial.println(led_state ? 1 : 0);
    }
    else if (c == '?') Serial.println("b=press s=status");
  }

  int btn = digitalRead(BTN);
  if (btn != last_btn) {
    if (btn == LOW) on_press();
    last_btn = btn;
  }
}
