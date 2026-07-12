// lcd-hello:LCD1602(PCF8574 I2C 背包 @0x27)显示两行文字
// 裸 Wire.h 手写背包驱动(不依赖 LiquidCrystal_I2C 库);
// 统计每次 endTransmission 的 ACK,全部成功才打 "lcd ok" —— 这就是 CI 断言
#include <Wire.h>

const uint8_t LCD_ADDR = 0x27;
const uint8_t BL = 0x08;      // P3 背光常开
int nack = 0;

void pcf(uint8_t v) {
  Wire.beginTransmission(LCD_ADDR);
  Wire.write(v | BL);
  if (Wire.endTransmission() != 0) nack++;
}

// 发一个 nibble:EN 高 → EN 低(下降沿锁存)
void nib(uint8_t n, uint8_t rs) {
  uint8_t v = (uint8_t)((n << 4) | rs);
  pcf(v | 0x04);
  pcf(v);
}

void cmd(uint8_t c)  { nib(c >> 4, 0); nib(c & 0x0F, 0); delayMicroseconds(50); }
void chr(uint8_t d)  { nib(d >> 4, 1); nib(d & 0x0F, 1); delayMicroseconds(50); }
void prn(const char *s) { while (*s) chr((uint8_t)*s++); }

void lcdInit() {
  delay(50);
  nib(0x3, 0); delay(5);
  nib(0x3, 0); delayMicroseconds(150);
  nib(0x3, 0);
  nib(0x2, 0);           // 进 4-bit 模式
  cmd(0x28);             // function set: 4-bit, 2 行
  cmd(0x0C);             // display on
  cmd(0x06);             // entry mode: 递增
  cmd(0x01); delay(2);   // 清屏
}

void setup() {
  Serial.begin(9600);
  Wire.begin();
  lcdInit();
  cmd(0x80);             // 第一行
  prn("Hello MoXin!");
  cmd(0xC0);             // 第二行
  prn("LCD1602 via I2C");
  Serial.println(nack == 0 ? "lcd ok" : "lcd err");
}

void loop() {
  delay(1000);
}
