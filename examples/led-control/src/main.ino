const int LED_R = 13;
const int LED_G = 12;
const int BTN   = 2;

void setup() {
  Serial.begin(9600);
  pinMode(LED_R, OUTPUT);
  pinMode(LED_G, OUTPUT);
  pinMode(BTN, INPUT_PULLUP);
  Serial.println("led-control ready. commands: r g s ?");
}

void loop() {
  if (Serial.available()) {
    char c = Serial.read();
    if (c == 'r') { digitalWrite(LED_R, !digitalRead(LED_R)); Serial.print("LED_R="); Serial.println(digitalRead(LED_R)); }
    else if (c == 'g') { digitalWrite(LED_G, !digitalRead(LED_G)); Serial.print("LED_G="); Serial.println(digitalRead(LED_G)); }
    else if (c == 's') { Serial.print("R="); Serial.print(digitalRead(LED_R)); Serial.print(" G="); Serial.println(digitalRead(LED_G)); }
    else if (c == '?') { Serial.println("r=red g=green s=status"); }
  }
  static int last_btn = HIGH;
  int btn = digitalRead(BTN);
  if (btn != last_btn) { Serial.print("BTN="); Serial.println(btn == LOW ? 1 : 0); last_btn = btn; }
}
