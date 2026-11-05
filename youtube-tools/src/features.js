// Danh sách tính năng (dùng chung cho content script và popup).
// Thêm tính năng mới: thêm 1 dòng ở đây + 1 file trong src/features/ + khai báo file đó trong manifest.json.
var YTT_FEATURES = [
  {
    id: "autoSkipAd",
    name: "Tự động skip quảng cáo",
    description: "Nút \"Bỏ qua\" vừa xuất hiện là tự bấm luôn, không cần thao tác.",
    defaultEnabled: true
  },
  {
    id: "skipAdKey",
    name: "Bấm phím bất kỳ để skip quảng cáo",
    description: "Khi quảng cáo có nút \"Bỏ qua\", bấm phím bất kỳ (trừ Ctrl/Shift/Alt) để skip.",
    defaultEnabled: true
  },
  {
    id: "speedUpAd",
    name: "Tua nhanh quảng cáo x16",
    description: "Đang có quảng cáo thì tắt tiếng và chạy nhanh x16, kể cả quảng cáo không cho skip. Hết quảng cáo tự trả lại tốc độ và âm thanh.",
    defaultEnabled: true
  },
  {
    id: "hideFeedAds",
    name: "Ẩn thẻ quảng cáo trong danh sách video",
    description: "Ẩn thẻ quảng cáo (Sponsored) ở trang chủ, kết quả tìm kiếm, cột video đề xuất và banner đầu trang.",
    defaultEnabled: true
  }
];
