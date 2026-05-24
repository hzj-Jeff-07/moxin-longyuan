// serial-echo — Arduino Uno
//
// 最小串口回显 demo：从 Serial 读到的每个字符都原样回显为 `echo: <c>`，
// 同时翻转 D13 LED 作为 RX 活动指示灯。
//
// 为什么用 D13：Phase 1 里只有 D13 有真实仿真状态，翻转它能让 TUI 的
// Board 接线图和 AI Inspector 真正跟着串口输入动起来。

const int LED = 13;

void setup() {
  Serial.begin(9600);
  pinMode(LED, OUTPUT);
  Serial.println("serial-echo ready. type a char, it echoes back.");
}

void loop() {
  if (Serial.available()) {
    char c = Serial.read();
    digitalWrite(LED, !digitalRead(LED));
    Serial.print("echo: ");
    Serial.println(c);
  }
}
