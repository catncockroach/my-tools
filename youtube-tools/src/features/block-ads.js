// Tính năng: chặn quảng cáo video từ gốc. Phần xử lý nằm ở src/page/block-ads.js (chạy trong
// trang, không đọc được chrome.storage). Module này chỉ ghi cờ bật/tắt vào localStorage của
// youtube.com để script trong trang đọc. Có hiệu lực từ video tải sau đó.
YTT.register("blockAds", (() => {
  "use strict";

  const FLAG_KEY = "ytt-block-ads"; // Trùng với src/page/block-ads.js.

  function setFlag(on) {
    try { localStorage.setItem(FLAG_KEY, on ? "1" : "0"); } catch (e) { YTT.log("Không ghi được localStorage:", e); }
  }

  return {
    start() { setFlag(true); },
    stop() { setFlag(false); }
  };
})());
