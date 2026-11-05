// Bật/tắt từng tính năng theo cài đặt trong popup.
(() => {
  "use strict";

  const running = {};
  const defaults = {};
  for (const f of YTT_FEATURES) defaults[f.id] = f.defaultEnabled;

  function apply(id, on) {
    const mod = YTT.modules[id];
    // Lần đầu (running[id] chưa có) cũng gọi stop() cho tính năng đang tắt, để tính năng nào
    // cần ghi trạng thái "tắt" (vd blockAds ghi cờ localStorage) đều được ghi. stop() phải an toàn
    // khi gọi lúc chưa start.
    if (!mod || running[id] === on) return;
    on ? mod.start() : mod.stop();
    running[id] = on;
  }

  chrome.storage.sync.get({ features: defaults }, ({ features }) => {
    for (const id of Object.keys(defaults)) apply(id, features[id] ?? defaults[id]);
    YTT.log(`Đã chạy v${chrome.runtime.getManifest().version}. Tính năng đang bật:`, Object.keys(running).filter((id) => running[id]));
  });

  chrome.storage.onChanged.addListener((changes) => {
    if (!changes.features) return;
    const features = changes.features.newValue || {};
    for (const id of Object.keys(defaults)) apply(id, features[id] ?? defaults[id]);
  });
})();
