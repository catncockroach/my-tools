// Tính năng: nút Skip vừa xuất hiện là tự bấm luôn.
YTT.register("autoSkipAd", (() => {
  "use strict";

  const CHECK_INTERVAL_MS = 300;
  const RETRY_AFTER_MS = 1500;  // Sau khi một lượt skipAd kết thúc, chờ thêm rồi mới thử lượt mới.
  const MAX_TRIES_PER_BUTTON = 5; // Tránh bấm mãi một nút không phản hồi.
  const REPORT_AFTER_MS = 8000; // Quảng cáo kéo dài mà chưa thấy nút Skip -> in chẩn đoán.

  let timer = null;
  let lastSkip = 0;
  let adStartedAt = 0;
  let reported = false;
  const tries = new WeakMap(); // nút -> số lần đã bấm

  function check() {
    const button = YTT.findSkipButton();
    if (!button && !YTT.isAdShowing()) {
      adStartedAt = 0;
      return;
    }
    if (!adStartedAt) {
      adStartedAt = Date.now();
      reported = false;
      YTT.log("Phát hiện quảng cáo, đang chờ nút Skip...");
    }

    if (button) {
      if (YTT.isSkipping()) { lastSkip = Date.now(); return; } // Lượt skip trước vẫn đang bấm lại/tua.
      if (Date.now() - lastSkip < RETRY_AFTER_MS) return;
      const n = tries.get(button) || 0;
      if (n >= MAX_TRIES_PER_BUTTON) return;
      if (n === MAX_TRIES_PER_BUTTON - 1) YTT.log("Lần thử cuối cho nút Skip này.");
      tries.set(button, n + 1);
      lastSkip = Date.now();
      YTT.skipAd(button);
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
