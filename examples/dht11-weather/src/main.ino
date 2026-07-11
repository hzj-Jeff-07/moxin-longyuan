// dht11-weather:D2 读 DHT11 温湿度,串口打印
// 手写 bit-bang 读取(不依赖 DHT 库),所有等待都带超时防卡死
const int DHT_PIN = 2;

// 等引脚变为 level,返回等待的 us;超时返回 -1
long waitLevel(int level, unsigned long timeout_us) {
  unsigned long t0 = micros();
  while (digitalRead(DHT_PIN) != level) {
    if (micros() - t0 > timeout_us) return -1;
  }
  return (long)(micros() - t0);
}

// 读一次:成功返回 1 并填 temp/hum,失败返回 0
int readDHT(int *temp, int *hum) {
  uint8_t data[5] = {0, 0, 0, 0, 0};

  // 起始:拉低 ≥18ms 后释放
  pinMode(DHT_PIN, OUTPUT);
  digitalWrite(DHT_PIN, LOW);
  delay(20);
  pinMode(DHT_PIN, INPUT_PULLUP);

  // 应答:80us 低 + 80us 高
  if (waitLevel(LOW, 200) < 0) return 0;
  if (waitLevel(HIGH, 200) < 0) return 0;

  // 40 bit:50us 低前导,高 27us=0 / 70us=1
  for (int i = 0; i < 40; i++) {
    if (waitLevel(LOW, 150) < 0) return 0;   // 进 bit 前导
    if (waitLevel(HIGH, 150) < 0) return 0;  // 前导结束,高电平开始
    unsigned long hs = micros();
    if (waitLevel(LOW, 150) < 0) return 0;   // 高电平结束
    if (micros() - hs > 45) data[i / 8] |= (0x80 >> (i % 8));
  }

  uint8_t sum = data[0] + data[1] + data[2] + data[3];
  if (sum != data[4]) return 0;
  *hum = data[0];
  *temp = data[2];
  return 1;
}

void setup() {
  Serial.begin(9600);
  Serial.println("dht11-weather ready");
}

void loop() {
  int temp = 0, hum = 0;
  if (readDHT(&temp, &hum)) {
    Serial.print("temp=");
    Serial.print(temp);
    Serial.print("C hum=");
    Serial.print(hum);
    Serial.println("%");
  } else {
    Serial.println("dht read failed");
  }
  delay(1000);
}
