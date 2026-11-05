// Chạy trong trang (MAIN world, document_start), trước mọi script của YouTube.
// Xóa danh sách quảng cáo (adPlacements, adSlots, playerAds) khỏi dữ liệu player trước khi
// player đọc, để player không phát quảng cáo. Cùng cách với uBlock Origin (json-prune).
// Không gọi được chrome.* ở đây; bật/tắt qua localStorage do src/features/block-ads.js ghi.
(() => {
  "use strict";

  const FLAG_KEY = "ytt-block-ads"; // "0" = tắt; thiếu hoặc khác "0" = bật (mặc định bật).
  const AD_KEYS = ["adPlacements", "adSlots", "playerAds"];
  const MAX_ARRAY_ITEMS = 10; // Dữ liệu dạng mảng chỉ xét vài phần tử đầu, tránh tốn CPU với mảng lớn.

  function enabled() {
    try { return localStorage.getItem(FLAG_KEY) !== "0"; } catch { return true; }
  }

  function log(...args) {
    console.log("%c[YTT]", "color:#e62117;font-weight:bold", ...args);
  }

  function pruneObject(obj, prefix, removed) {
    if (!obj || typeof obj !== "object") return;
    for (const key of AD_KEYS) {
      if (!(key in obj)) continue;
      const value = obj[key];
      removed.push(`${prefix}${key}(${Array.isArray(value) ? value.length : 1})`);
      delete obj[key];
    }
  }

  // Xóa các khóa quảng cáo ở cấp ngoài cùng và trong playerResponse. Trả về chính data.
  function prune(data, source) {
    if (!data || typeof data !== "object" || !enabled()) return data;
    const items = Array.isArray(data) ? data.slice(0, MAX_ARRAY_ITEMS) : [data];
    const removed = [];
    for (const item of items) {
      if (!item || typeof item !== "object") continue;
      pruneObject(item, "", removed);
      pruneObject(item.playerResponse, "playerResponse.", removed);
    }
    if (removed.length) log(`Đã xóa khỏi ${source}: ${removed.join(", ")}.`);
    return data;
  }

  // 1. Dữ liệu lúc mở trang: `var ytInitialPlayerResponse = {...}` trong HTML.
  //    Định nghĩa sẵn setter trên window để lọc ngay khi YouTube gán.
  let initialPlayerResponse;
  try {
    Object.defineProperty(window, "ytInitialPlayerResponse", {
      configurable: true,
      get() { return initialPlayerResponse; },
      set(value) { initialPlayerResponse = prune(value, "ytInitialPlayerResponse"); }
    });
  } catch (e) {
    log("Không gắn được setter ytInitialPlayerResponse:", e);
  }

  // 2. Dữ liệu khi chuyển video (fetch /youtubei/v1/player...) đọc bằng Response.json().
  //    Dùng Proxy để hàm vẫn trông như hàm gốc (toString trả "[native code]").
  const shortUrl = (url) => String(url || "").replace(/^https?:\/\/[^/]+/, "").split("?")[0];
  Response.prototype.json = new Proxy(Response.prototype.json, {
    apply(target, thisArg, args) {
      return Reflect.apply(target, thisArg, args)
        .then((data) => prune(data, `fetch ${shortUrl(thisArg && thisArg.url)}`));
    }
  });

  // 3. Dữ liệu đọc dạng chữ rồi JSON.parse (XHR, response.text()).
  JSON.parse = new Proxy(JSON.parse, {
    apply(target, thisArg, args) {
      return prune(Reflect.apply(target, thisArg, args), "JSON.parse");
    }
  });

  log(`Chặn quảng cáo từ gốc: đã cài, trạng thái ${enabled() ? "bật" : "tắt"}.`);
})();
