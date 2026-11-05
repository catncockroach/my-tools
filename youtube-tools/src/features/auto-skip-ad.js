// Tính năng: nút Skip vừa xuất hiện là tự bấm luôn.
// Quảng cáo có thể nối tiếp nhau (quảng cáo 1 -> quảng cáo 2 -> màn hình kết thúc có nút Skip).
// YTT.skipAd tự bấm liên tục tới khi nút biến mất; ở đây chỉ cần phát hiện nút thật nhanh.
YTT.register("autoSkipAd", (() => {
  "use strict";

  const CHECK_INTERVAL_MS = 100;
  const REPORT_AFTER_MS = 8000; // Quảng cáo kéo dài mà chưa thấy nút Skip -> in chẩn đoán.

  let timer = null;
  let adStartedAt = 0;
  let buttonSeenAt = 0;
  let reported = false;
  let gaveUp = false; // skipAd đã bó tay với nút đang hiện -> chờ nút biến mất rồi mới thử lại.

  function check() {
    const button = YTT.findSkipButton();

    if (!button) {
      buttonSeenAt = 0;
      gaveUp = false;
      if (!YTT.isAdShowing()) { adStartedAt = 0; return; }
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

  return {
    start() { if (!timer) timer = setInterval(check, CHECK_INTERVAL_MS); },
    stop() { clearInterval(timer); timer = null; }
  };
})());
