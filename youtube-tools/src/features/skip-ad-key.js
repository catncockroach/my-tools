// Tính năng: khi có quảng cáo và nút Skip đã hiện, bấm phím bất kỳ để skip.
YTT.register("skipAdKey", (() => {
  "use strict";

  // Phím chỉ là modifier thì bỏ qua, để không chặn các tổ hợp như Ctrl+Tab, Alt+Tab.
  const IGNORED_KEYS = new Set(["Control", "Shift", "Alt", "Meta", "CapsLock", "Fn", "OS"]);

  function onKeyDown(e) {
    if (e.repeat || IGNORED_KEYS.has(e.key) || YTT.isTyping(e.target)) return;
    if (!YTT.isAdShowing()) return;
    const button = YTT.findSkipButton();
    if (!button) return; // Chưa có nút Skip -> để phím hoạt động bình thường.

    // Chặn phím, để nó không làm thao tác khác (vd: Space tạm dừng, F fullscreen).
    e.preventDefault();
    e.stopImmediatePropagation();
    YTT.skipAd(button);
  }

  return {
    // Capture ở window để chạy trước các phím tắt của YouTube.
    start() { window.addEventListener("keydown", onKeyDown, true); },
    stop() { window.removeEventListener("keydown", onKeyDown, true); }
  };
})());
