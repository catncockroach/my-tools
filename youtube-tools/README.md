# YouTube Tools

Tiện ích khi xem YouTube. Bấm vào icon extension để bật/tắt từng chức năng.

## Chức năng

- **Tự động skip quảng cáo**: nút "Bỏ qua" vừa xuất hiện là tự bấm luôn.
- **Bấm phím bất kỳ để skip**: khi nút "Bỏ qua" đã hiện, bấm phím bất kỳ (trừ Ctrl/Shift/Alt) để skip.
- **Tua nhanh quảng cáo x16**: đang có quảng cáo thì tắt tiếng và chạy x16 (cả quảng cáo không cho skip), hết quảng cáo thì trả lại tốc độ và âm thanh.
- **Ẩn thẻ quảng cáo trong danh sách video**: ẩn thẻ "Sponsored" ở trang chủ, kết quả tìm kiếm, cột video đề xuất và banner đầu trang (chỉ dùng CSS, không tốn CPU).

Skip chỉ dùng được với quảng cáo có nút "Bỏ qua"; quảng cáo không cho skip thì được tua nhanh. Đây không phải chặn quảng cáo: quảng cáo vẫn được tải và phát, chỉ là rất nhanh.

## Cấu trúc

```
manifest.json
src/features.js        Danh sách chức năng (tên, mô tả, bật mặc định) - dùng chung cho popup
src/core.js            Hàm dùng chung: tìm video, kiểm tra quảng cáo, bấm Skip, thông báo
src/features/*.js      Mỗi chức năng một file, đăng ký bằng YTT.register(id, { start, stop })
src/main.js            Bật/tắt chức năng theo cài đặt
popup/                 Giao diện bật/tắt
icons/
```

## Thêm chức năng mới

1. Thêm mô tả vào `src/features.js`.
2. Tạo `src/features/<ten>.js` gọi `YTT.register("<id>", { start() {}, stop() {} })`.
3. Khai báo file trong `manifest.json` → `content_scripts.js` (trước `src/main.js`).
