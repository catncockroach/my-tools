# YouTube Tools

Tiện ích khi xem YouTube. Bấm vào icon extension để bật/tắt từng chức năng.

## Chức năng

- **Tự động skip quảng cáo**: nút "Bỏ qua" vừa xuất hiện là tự bấm luôn.
- **Bấm phím bất kỳ để skip**: khi nút "Bỏ qua" đã hiện, bấm phím bất kỳ (trừ Ctrl/Shift/Alt) để skip.

Chỉ skip được quảng cáo có nút "Bỏ qua".

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
