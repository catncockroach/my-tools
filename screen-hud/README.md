# screen-hud

Overlay nhỏ, mờ ở góc dưới phải (hoặc trái) màn hình Windows (ngay trên taskbar), hiện những thông tin bình thường phải bấm vài lần mới xem được. Hiện tại: **% pin các thiết bị Bluetooth đang kết nối**.

- Click xuyên qua, không lấy focus, không hiện trên taskbar/Alt-Tab.
- Tự ẩn khi có app toàn màn hình (video, game, trình chiếu).
- Pin ≤ 20% hiện màu đỏ nhạt.
- Process nền viết bằng Rust + Win32 thuần, không webview: chỉ làm việc khi có thay đổi.

## Nguồn dữ liệu pin

Lấy đúng giá trị mà Windows hiện trong **Settings > Bluetooth & devices** (tai nghe qua HFP, bàn phím/chuột BLE). Thiết bị mà Windows không hiện % (một số tai nghe dùng giao thức riêng như AirPods, thiết bị dùng dongle 2.4GHz) thì chưa hỗ trợ.

## Cài đặt / cập nhật / gỡ

- **Cài:** tải `screen-hud-vX.Y.Z.exe` ở [Releases](https://github.com/catncockroach/my-tools/releases) rồi chạy. App tự chép vào `%LOCALAPPDATA%\Programs\screen-hud`, đăng ký trong Settings > Apps, bật khởi động cùng Windows và chạy ở khay hệ thống. Không cần quyền admin.
- **Cập nhật:** tải bản mới rồi chạy: app tự tắt bản cũ, thay file, chạy lại (giữ nguyên cài đặt).
- **Gỡ:** Settings > Apps > screen-hud.
- Exe chưa ký số nên lần đầu SmartScreen có thể cảnh báo: *More info* > *Run anyway*.

## Dùng

- Chỉ hiện thiết bị đang kết nối; không có thiết bị nào thì overlay ẩn hẳn.
- Icon con vịt ở khay hệ thống; rê chuột vào để xem % pin từng thiết bị. Bấm vào để mở menu: bật/tắt overlay, chọn góc dưới trái/phải, khởi động cùng Windows, tải bản mới, thoát.
- `screen-hud.exe --all`: hiện cả thiết bị chưa kết nối (chỉ để chẩn đoán).
- `screen-hud.exe --list`: in danh sách thiết bị (tên, %, trạng thái) rồi thoát.
- `screen-hud.exe --log`: ghi nhật ký sự kiện vào `%TEMP%\screen-hud.log` để chẩn đoán.

## Phát triển (từ WSL)

Cần trên Windows: rustup với toolchain `stable-x86_64-pc-windows-gnu` (không cần Visual Studio).

Phát hành: bump `version` trong `Cargo.toml`, commit, rồi `git tag screen-hud-vX.Y.Z && git push --tags`. GitHub Actions build (MSVC) và tạo Release kèm exe.

```bash
./dev.sh build        # rsync sang C:\Users\haihv\dev\screen-hud rồi cargo build --release
./dev.sh run          # tắt bản đang chạy, build, chạy bản mới với --portable (không cài)
./dev.sh run --all
./dev.sh list         # in danh sách thiết bị
./dev.sh mem          # RAM/CPU của bản đang chạy
./dev.sh stop
```
