// Tính năng: nút Skip vừa xuất hiện là tự bấm luôn.
// Quảng cáo có thể nối tiếp nhau (quảng cáo 1 -> quảng cáo 2 -> màn hình kết thúc có nút Skip).
// YTT.skipAd tự bấm liên tục tới khi nút biến mất; ở đây chỉ cần phát hiện nút thật nhanh.
YTT.register("autoSkipAd", (() => {
  "use strict";

  // Kiểm tra theo 2 tốc độ: chậm khi không có quảng cáo (đỡ tốn CPU), nhanh khi có dấu hiệu
  // quảng cáo. Giữ tốc độ nhanh thêm một lúc sau quảng cáo, vì quảng cáo kế tiếp hoặc màn hình
  // kết thúc (có thể không còn class "ad-showing") hay xuất hiện ngay sau đó.
  const FAST_MS = 100;
  const SLOW_MS = 500;
  const STAY_FAST_MS = 5000;
  const REPORT_AFTER_MS = 8000; // Quảng cáo kéo dài mà chưa thấy nút Skip -> in chẩn đoán.

  let timer = null;
  let adStartedAt = 0;
  let buttonSeenAt = 0;
  let reported = false;
  let lastAdSignal = 0; // Lần cuối thấy quảng cáo hoặc nút Skip.
  let gaveUp = false; // skipAd đã bó tay với nút đang hiện -> chờ nút biến mất rồi mới thử lại.

  function check() {
    const button = YTT.findSkipButton();
    const adShowing = YTT.isAdShowing();
    if (button || adShowing) lastAdSignal = Date.now();

    if (!button) {
      buttonSeenAt = 0;
      gaveUp = false;
      if (!adShowing) { adStartedAt = 0; return; }
    }

    if (!adStartedAt) {
      adStartedAt = Date.now();
      reported = false;
      YTT.log("Phát hiện quảng cáo, đang chờ nút Skip...");
    }

    if (button) {
      if (!buttonSeenAt) {
        buttonSeenAt = Date.now();
        YTT.log(`Thấy nút Skip (quảng cáo đã chạy ${((buttonSeenAt - adStartedAt) / 1000).toFixed(1)}s).`);
      }
      if (gaveUp || YTT.isSkipping()) return;
      YTT.skipAd((ok) => { if (!ok) gaveUp = true; });
      return;
    }

    if (!reported && Date.now() - adStartedAt > REPORT_AFTER_MS) {
      reported = true;
      YTT.log("Quá 8 giây chưa thấy nút Skip (có thể quảng cáo không cho skip). Chẩn đoán:", YTT.describeAdButtons());
    }
  }

  function loop() {
    try { check(); } catch (e) { YTT.log("Lỗi khi kiểm tra quảng cáo:", e); }
    const fast = Date.now() - lastAdSignal < STAY_FAST_MS;
    timer = setTimeout(loop, fast ? FAST_MS : SLOW_MS);
  }

  return {
    start() { if (!timer) loop(); },
    stop() { clearTimeout(timer); timer = null; }
  };
})());
