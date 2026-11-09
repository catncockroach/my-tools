// Tính năng: ẩn thẻ quảng cáo ("Sponsored") trong trang chủ, kết quả tìm kiếm, cột đề xuất, panel quảng cáo, banner.
// Chỉ chèn CSS (không vòng lặp, không theo dõi DOM): nhẹ, áp dụng luôn cho quảng cáo nạp thêm khi cuộn.
// Ẩn cả ô chứa quảng cáo (dùng :has) để lưới video không bị trống một ô.
// Không đụng tới quảng cáo trong player, để không ảnh hưởng tới việc tìm nút Skip.
YTT.register("hideFeedAds", (() => {
  "use strict";

  const STYLE_ID = "ytt-hide-feed-ads";

  // Khi YouTube đổi giao diện quảng cáo, thêm selector mới vào đây.
  const SELECTORS = [
    // Ô trong lưới trang chủ / kênh có chứa quảng cáo.
    "ytd-rich-item-renderer:has(ytd-ad-slot-renderer)",
    "ytd-rich-item-renderer:has(top-landscape-image-layout-view-model)",
    "ytd-rich-item-renderer:has(feed-ad-metadata-view-model)",
    "ytd-rich-section-renderer:has(ytd-ad-slot-renderer)",
    // Bản thân các khối quảng cáo (kết quả tìm kiếm, cột đề xuất bên phải...).
    "ytd-ad-slot-renderer",
    "top-landscape-image-layout-view-model",
    "ytd-in-feed-ad-layout-renderer",
    "ytd-display-ad-renderer",
    "ytd-promoted-sparkles-web-renderer",
    "ytd-promoted-video-renderer",
    "ytd-search-pyv-renderer",
    "ytd-companion-slot-renderer",
    "ytd-player-legacy-desktop-watch-ads-renderer",
    // Panel quảng cáo phía trên cột đề xuất ở trang xem video (target-id "engagement-panel-ads",
    // "engagement-panel-ads-business-agent-chat"...).
    "ytd-engagement-panel-section-list-renderer[target-id^='engagement-panel-ads']",
    "ytd-ads-engagement-panel-content-renderer",
    // Banner quảng cáo đầu trang chủ.
    "#masthead-ad",
    "ytd-banner-promo-renderer",
    "ytd-statement-banner-renderer"
  ];

  return {
    start() {
      if (document.getElementById(STYLE_ID)) return;
      const style = document.createElement("style");
      style.id = STYLE_ID;
      style.textContent = SELECTORS.map((s) => `${s} { display: none !important; }`).join("\n");
      (document.head || document.documentElement).appendChild(style);
    },
    stop() {
      document.getElementById(STYLE_ID)?.remove();
    }
  };
})());
