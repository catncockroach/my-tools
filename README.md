# Chrome Extensions

Nơi chứa các Chrome extension dùng cá nhân. Mỗi extension là một thư mục riêng, có `manifest.json`.

| Extension | Mô tả |
|---|---|
| [youtube-tools](youtube-tools/) | Tiện ích khi xem YouTube: tự động skip quảng cáo, bấm phím bất kỳ để skip, tua nhanh quảng cáo x16. |

## Cài vào Chrome

1. Copy extension sang ổ Windows: `./sync-to-windows.sh` (mặc định vào `C:\Users\haihv\Downloads\chrome-extension\`, đổi bằng biến `WIN_EXT_DIR`).
2. Mở `chrome://extensions`, bật **Developer mode**.
3. Bấm **Load unpacked** → chọn thư mục extension, ví dụ `C:\Users\haihv\Downloads\chrome-extension\youtube-tools`.

Sau khi sửa code: chạy lại `./sync-to-windows.sh`, bấm ↻ (Reload) trên thẻ extension trong `chrome://extensions`, rồi F5 trang web.

## Thêm extension mới

Tạo thư mục mới cạnh `youtube-tools/` có `manifest.json` (Manifest V3), rồi thêm một dòng vào bảng trên.
