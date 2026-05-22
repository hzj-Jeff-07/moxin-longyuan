// button-counter — Arduino Uno
//
// Phase 1 现实：simavr bridge 还没接真实 GPIO 输入注入，所以"按按钮"
// 通过 TUI Serial Monitor 输入字符 'b' 来模拟。每收到一次 'b'：
//   - counter++
//   - 翻转 D13 LED
//   - 通过 Serial 打印 COUNT=<n> 和 LED=<0|1>
//
// 同时也读 D2 上的物理按钮（INPUT_PULLUP）；transition 也算一次计数，
// 等真实 GPIO 注入落地后这条路径会自动激活，不用改 firmware。

const int BTN = 2;
const int LED = 13;

unsigned long count = 0;
int last_btn = HIGH;
int led_state = LOW;

static void on_press() {
  count++;
  led_state = !led_state;
  digitalWrite(LED, led_state);
  Serial.print("COUNT=");
  Serial.println(count);
  Serial.print("LED=");
  Serial.println(led_state ? 1 : 0);
}

void setup() {
  Serial.begin(9600);
  pinMode(LED, OUTPUT);
  pinMode(BTN, INPUT_PULLUP);
  digitalWrite(LED, led_state);
  Serial.println("button-counter ready. press 'b' to simulate button.");
}

void loop() {
  if (Serial.available()) {
    char c = Serial.read();
    if (c == 'b') on_press();
    else if (c == '?') Serial.println("b=press s=status r=reset");
    else if (c == 's') {
      Serial.print("COUNT=");
      Serial.println(count);
    }
    else if (c == 'r') {
      count = 0;
      Serial.println("RESET");
    }
  }

  int btn = digitalRead(BTN);
  if (btn != last_btn) {
    if (btn == LOW) on_press();
    last_btn = btn;
  }
}
