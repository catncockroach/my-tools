// Tính năng: nút Skip vừa xuất hiện là tự bấm luôn.
// Quảng cáo có thể nối tiếp nhau (quảng cáo 1 -> quảng cáo 2 -> trang ảnh tĩnh có nút Skip),
// nên mỗi lần nút Skip biến mất thì coi như xong một quảng cáo và đếm lại từ đầu.
YTT.register("autoSkipAd", (() => {
  "use strict";

  const CHECK_INTERVAL_MS = 300;
  const RETRY_AFTER_MS = 1500;  // Sau khi một lượt skipAd kết thúc, chờ thêm rồi mới thử lượt mới.
  const MAX_ROUNDS = 6;         // Số lượt skipAd liên tiếp mà nút vẫn còn, quá thì tạm dừng để khỏi bấm mãi.
  const REPORT_AFTER_MS = 8000; // Quảng cáo kéo dài mà chưa thấy nút Skip -> in chẩn đoán.

  let timer = null;
  let lastSkip = 0;
  let adStartedAt = 0;
  let reported = false;
  let rounds = 0;       // Số lượt skipAd liên tiếp trong khi nút Skip vẫn còn.
  let gaveUp = false;

  function check() {
    const button = YTT.findSkipButton();

    if (!button) {
      // Nút đã biến mất -> lượt sau (quảng cáo kế tiếp, trang tĩnh) được thử lại từ đầu.
      rounds = 0;
      gaveUp = false;
      if (!YTT.isAdShowing()) adStartedAt = 0;
    }
    if (!button && !YTT.isAdShowing()) return;

    if (!adStartedAt) {
      adStartedAt = Date.now();
      reported = false;
      YTT.log("Phát hiện quảng cáo, đang chờ nút Skip...");
    }

    if (button) {
      if (YTT.isSkipping()) { lastSkip = Date.now(); return; } // Lượt trước vẫn đang bấm lại/tua.
      if (Date.now() - lastSkip < RETRY_AFTER_MS) return;
      if (rounds >= MAX_ROUNDS) {
        if (!gaveUp) {
          gaveUp = true;
          YTT.log(`Đã thử ${MAX_ROUNDS} lượt mà nút Skip vẫn còn, tạm dừng. Chẩn đoán:`, YTT.describeAdButtons());
        }
        return;
      }
      rounds++;
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
