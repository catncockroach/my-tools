# screen-hud

Overlay nhỏ, mờ ở góc dưới phải màn hình Windows (ngay trên taskbar), hiện những thông tin bình thường phải bấm vài lần mới xem được. Hiện tại: **% pin các thiết bị Bluetooth đang kết nối**.

- Click xuyên qua, không lấy focus, không hiện trên taskbar/Alt-Tab.
- Tự ẩn khi có app toàn màn hình (video, game, trình chiếu).
- Pin ≤ 20% hiện màu đỏ nhạt.
- Process nền viết bằng Rust + Win32 thuần, không webview: chỉ làm việc khi có thay đổi.

## Nguồn dữ liệu pin

Lấy đúng giá trị mà Windows hiện trong **Settings > Bluetooth & devices** (tai nghe qua HFP, bàn phím/chuột BLE). Thiết bị mà Windows không hiện % (một số tai nghe dùng giao thức riêng như AirPods, thiết bị dùng dongle 2.4GHz) thì chưa hỗ trợ.

## Dùng

- Chỉ hiện thiết bị đang kết nối; không có thiết bị nào thì overlay ẩn hẳn.
- Icon ở khay hệ thống, chuột phải (hoặc trái) để mở menu: bật/tắt overlay, thoát.
- `screen-hud.exe --all`: hiện cả thiết bị chưa kết nối (chỉ để chẩn đoán).
- `screen-hud.exe --list`: in danh sách thiết bị (tên, %, trạng thái) rồi thoát.
- `screen-hud.exe --log`: ghi nhật ký sự kiện vào `%TEMP%\screen-hud.log` để chẩn đoán.

## Phát triển (từ WSL)

Cần trên Windows: rustup với toolchain `stable-x86_64-pc-windows-gnu` (không cần Visual Studio). Bản release do GitHub Actions build (MSVC): push tag `screen-hud-vX.Y.Z` khớp version trong `Cargo.toml` sẽ tạo GitHub Release kèm exe.

```bash
./dev.sh build        # rsync sang C:\Users\haihv\dev\screen-hud rồi cargo build --release
./dev.sh run          # build, tắt bản đang chạy, chạy bản mới
./dev.sh run --all
./dev.sh list         # in danh sách thiết bị
./dev.sh mem          # RAM/CPU của bản đang chạy
./dev.sh stop
```
