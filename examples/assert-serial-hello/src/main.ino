// assert-serial-hello — Arduino Uno
//
// 最小串口程序，专门给 `moxin assert --serial-contains "hello" --within 2s` 验证用。
// setup() 里 print 一行 "hello world"，2 秒窗口内必然能捕获 → exit 0。
//
// 故意做坏验证：把 Serial.println("hello world") 改成 "hi world"，
// 再跑 assert，应得到 exit 2 (TIMEOUT)，证明断言能抓到"输出文案变了"的回归。

void setup() {
  Serial.begin(9600);
  Serial.println("hello world");
}

void loop() {
  // 故意保持空 loop —— 输出只在 setup 里发生一次，
  // 这样能验证 wait_serial 真的扫描了已累积的 serial_lines，
  // 而不是只盯当前帧的新增。
}
