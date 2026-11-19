# my-tools

Các tool dùng cá nhân. Mỗi tool là một thư mục riêng.

| Tool | Loại | Mô tả |
|---|---|---|
| [youtube-tools](youtube-tools/) | Chrome extension | Tiện ích khi xem YouTube: chặn quảng cáo video từ gốc, tự động skip quảng cáo, bấm phím bất kỳ để skip, tua nhanh quảng cáo x3, ẩn thẻ quảng cáo trong danh sách video. |
| [screen-hud](screen-hud/) | App Windows (Rust) | Overlay nhỏ, mờ ở góc dưới phải (hoặc trái) màn hình, hiện % pin các thiết bị Bluetooth đang kết nối. Process nền tốn rất ít RAM. |

## Chrome extension

### Cài vào Chrome

1. Copy extension sang ổ Windows: `./sync-to-windows.sh` (mặc định vào `C:\Users\haihv\Downloads\chrome-extension\`, đổi bằng biến `WIN_EXT_DIR`).
2. Mở `chrome://extensions`, bật **Developer mode**.
3. Bấm **Load unpacked** → chọn thư mục extension, ví dụ `C:\Users\haihv\Downloads\chrome-extension\youtube-tools`.

Sau khi sửa code: chạy lại `./sync-to-windows.sh`, bấm ↻ (Reload) trên thẻ extension trong `chrome://extensions`, rồi F5 trang web. Với youtube-tools có thể bấm vào ô version trong popup thay cho bước Reload (ô hiện `↻ vX` khi có bản mới chưa nạp).

### Thêm extension mới

Tạo thư mục mới cạnh `youtube-tools/` có `manifest.json` (Manifest V3), rồi thêm một dòng vào bảng trên.

## App Windows

Xem README trong thư mục của từng app, ví dụ [screen-hud](screen-hud/README.md).
