#!/usr/bin/env bash
# Build và chạy screen-hud trên Windows từ WSL.
# Source được rsync sang ổ Windows rồi build bằng cargo.exe phía Windows
# (build thẳng từ \\wsl$ rất chậm).
#
# Dùng: ./dev.sh build          build release
#       ./dev.sh run [args...]  build, tắt bản đang chạy, chạy nền (vd: ./dev.sh run --all)
#       ./dev.sh list           build rồi in danh sách thiết bị
#       ./dev.sh stop           tắt bản đang chạy
#       ./dev.sh mem            RAM/CPU của bản đang chạy
set -euo pipefail

SRC="$(cd "$(dirname "$0")" && pwd)"
WIN_DIR="${SCREEN_HUD_WIN_DIR:-/mnt/c/Users/haihv/dev/screen-hud}"
EXE="$(wslpath -w "$WIN_DIR")\\target\\release\\screen-hud.exe"

# Chạy lệnh Windows qua WSL interop. Interop đôi khi lỗi ngay lúc khởi động process
# ("UtilAcceptVsock: accept4 failed 110"), khi đó thử lại tối đa 3 lần.
interop() {
  local out rc attempt
  for attempt in 1 2 3; do
    rc=0
    out=$(cd "$WIN_DIR" && "$@" 2>&1) || rc=$?
    [[ "$out" == *UtilAcceptVsock* ]] || break
    echo "(WSL interop lỗi, thử lại lần $attempt)" >&2
  done
  printf '%s\n' "$out" | tr -d '\r'
  return $rc
}

ps() { interop powershell.exe -NoProfile -Command "$1"; }

build() {
  mkdir -p "$WIN_DIR"
  rsync -a --delete --exclude target/ "$SRC/" "$WIN_DIR/"
  interop cmd.exe /c '%USERPROFILE%\.cargo\bin\cargo.exe build --release'
  # Giữ Cargo.lock trong repo (lần rsync sau mang nó sang lại).
  cp "$WIN_DIR/Cargo.lock" "$SRC/Cargo.lock"
}

stop() { ps 'Stop-Process -Name screen-hud -Force -ErrorAction SilentlyContinue; exit 0'; }

case "${1:-build}" in
  build) build ;;
  run)
    shift
    stop   # exe đang chạy thì không ghi đè được
    build
    args=""
    [ $# -gt 0 ] && args="-ArgumentList '$*'"
    ps "Start-Process -FilePath '$EXE' $args"
    echo "Đã chạy $EXE $*" ;;
  list) build; ps "& '$EXE' --list" ;;
  stop) stop ;;
  mem)
    ps '
      $p = Get-Process screen-hud -ErrorAction SilentlyContinue
      if (-not $p) { "screen-hud không chạy"; exit }
      $c = (Get-Counter "\Process(screen-hud)\Working Set - Private").CounterSamples[0].CookedValue
      "Private working set (Task Manager): {0:N1} MB" -f ($c/1MB)
      "Working set:                        {0:N1} MB" -f ($p.WorkingSet64/1MB)
      "Private bytes (commit):             {0:N1} MB" -f ($p.PrivateMemorySize64/1MB)
      "Threads: {0}   Handles: {1}   CPU time: {2:N2}s" -f $p.Threads.Count, $p.HandleCount, $p.CPU
    ' ;;
  *) echo "Lệnh không hợp lệ: $1" >&2; exit 1 ;;
esac
