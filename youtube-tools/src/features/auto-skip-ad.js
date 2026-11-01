// Tính năng: nút Skip vừa xuất hiện là tự bấm luôn.
YTT.register("autoSkipAd", (() => {
  "use strict";

  const CHECK_INTERVAL_MS = 300;
  const RETRY_AFTER_MS = 1000; // Tránh bấm liên tục khi YouTube chưa kịp xử lý.

  let timer = null;
  let lastSkip = 0;

  function check() {
    if (!YTT.isAdShowing()) return;
    if (Date.now() - lastSkip < RETRY_AFTER_MS) return;
    const button = YTT.findSkipButton();
    if (!button) return;
    lastSkip = Date.now();
    YTT.skipAd(button);
  }

  return {
    start() { if (!timer) timer = setInterval(check, CHECK_INTERVAL_MS); },
    stop() { clearInterval(timer); timer = null; }
  };
})());
