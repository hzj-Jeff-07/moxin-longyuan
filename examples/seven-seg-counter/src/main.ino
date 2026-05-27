// seven-seg-counter — Arduino Uno
//
// 共阴 1 位 7 段数码管,每秒 +1,0→9→0 循环。
// 用来验证 Phase 2-mini Step 5 数码管真驱动:
//   moxin run --output json | grep '"pin"'
// 应能看到 D2-D8 的 8 路段位事件,且 `moxin show` TUI 里
// 7SEG 字符随真 GPIO 跳数字。
//
// 段位映射(共阴 → 高电平点亮):
//   a=D2  b=D3  c=D4  d=D5  e=D6  f=D7  g=D8  dp=D9

const uint8_t SEG_PINS[8] = {2, 3, 4, 5, 6, 7, 8, 9};
//                           a  b  c  d  e  f  g  dp

// 共阴段位码,bit 顺序 = a,b,c,d,e,f,g,dp(与 SEG_PINS 同序)
const uint8_t DIGITS[10] = {
  0b11111100, // 0
  0b01100000, // 1
  0b11011010, // 2
  0b11110010, // 3
  0b01100110, // 4
  0b10110110, // 5
  0b10111110, // 6
  0b11100000, // 7
  0b11111110, // 8
  0b11110110, // 9
};

uint8_t digit = 0;

static void show_digit(uint8_t d) {
  uint8_t pat = DIGITS[d];
  for (uint8_t i = 0; i < 8; i++) {
    digitalWrite(SEG_PINS[i], (pat >> (7 - i)) & 1 ? HIGH : LOW);
  }
}

void setup() {
  Serial.begin(9600);
  for (uint8_t i = 0; i < 8; i++) {
    pinMode(SEG_PINS[i], OUTPUT);
    digitalWrite(SEG_PINS[i], LOW);
  }
  Serial.println("seven-seg-counter ready. 0..9 every second.");
}

void loop() {
  show_digit(digit);
  Serial.print("DIGIT=");
  Serial.println(digit);
  digit = (digit + 1) % 10;
  delay(1000);
}
