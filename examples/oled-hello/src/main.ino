// oled-hello:SSD1306 128×64 OLED(I2C @0x3C)填充图案
// 裸 Wire.h 手写最小驱动(不依赖 Adafruit_SSD1306 库);
// 统计每次 endTransmission 的 ACK,全部成功才打 "oled ok" —— 这就是 CI 断言
#include <Wire.h>

const uint8_t OLED = 0x3C;
int nack = 0;

void cmd(uint8_t c) {
  Wire.beginTransmission(OLED);
  Wire.write(0x00);          // 控制字节:命令流
  Wire.write(c);
  if (Wire.endTransmission() != 0) nack++;
}

// 一次写一段数据(控制字节 0x40 + 最多 16 字节)
void dataChunk(const uint8_t *buf, uint8_t n) {
  Wire.beginTransmission(OLED);
  Wire.write(0x40);          // 控制字节:数据流
  for (uint8_t i = 0; i < n; i++) Wire.write(buf[i]);
  if (Wire.endTransmission() != 0) nack++;
}

void initOLED() {
  static const uint8_t seq[] = {
    0xAE, 0x20, 0x00, 0x21, 0x00, 0x7F, 0x22, 0x00, 0x07,
    0xA8, 0x3F, 0xD3, 0x00, 0x40, 0xA1, 0xC8, 0xDA, 0x12,
    0x81, 0x7F, 0xA4, 0xA6, 0xD5, 0x80, 0x8D, 0x14, 0xAF,
  };
  // 逐字节发(0x20/0x21/0x22 等带参命令也逐字节走命令流,bridge 会按序解析)
  for (uint8_t i = 0; i < sizeof(seq); i++) cmd(seq[i]);
}

void setup() {
  Serial.begin(9600);
  Wire.begin();
  initOLED();

  // 填充竖条纹图案(0xAA):128×64 的一半像素点亮
  uint8_t chunk[16];
  for (uint8_t i = 0; i < 16; i++) chunk[i] = 0xAA;
  for (int i = 0; i < 1024; i += 16) dataChunk(chunk, 16);

  Serial.println(nack == 0 ? "oled ok" : "oled err");
}

void loop() {
  delay(1000);
}
