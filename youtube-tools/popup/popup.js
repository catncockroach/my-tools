const list = document.getElementById("list");
const status = document.getElementById("status");
const defaults = {};
for (const f of YTT_FEATURES) defaults[f.id] = f.defaultEnabled;

// Ô version: bấm để tải lại extension, giống nút Reload trong chrome://extensions
// (nạp code mới vừa copy sang). Tab YouTube đang mở vẫn cần F5 để chạy code mới.
const versionChip = document.getElementById("version");
const loadedVersion = chrome.runtime.getManifest().version;
versionChip.textContent = "v" + loadedVersion;
versionChip.title = "Bấm để tải lại extension";
versionChip.addEventListener("click", () => {
  versionChip.disabled = true;
  versionChip.textContent = "Đang tải lại…";
  setTimeout(() => chrome.runtime.reload(), 150); // Popup sẽ tự đóng.
});

// Extension cài dạng unpacked đọc file trực tiếp từ đĩa, nên manifest.json trên đĩa có thể
// mới hơn bản đang chạy -> báo có bản mới.
fetch(chrome.runtime.getURL("manifest.json"), { cache: "no-store" })
  .then((r) => r.json())
  .then(({ version }) => {
    if (!version || version === loadedVersion || versionChip.disabled) return;
    versionChip.textContent = "↻ v" + version;
    versionChip.title = `Đang chạy v${loadedVersion}, có bản v${version}. Bấm để cập nhật.`;
    versionChip.classList.add("update");
  })
  .catch(() => {});

// Cập nhật dòng "x/y đang bật" và hiệu ứng ở logo.
function updateStatus(features) {
  const on = YTT_FEATURES.filter((f) => features[f.id]).length;
  document.getElementById("count").textContent =
    on ? `${on}/${YTT_FEATURES.length} tính năng đang bật` : "Đang tắt tất cả";
  document.body.classList.toggle("active", on > 0);
}

// Hiện "✓ Đã lưu" thoáng qua ở dòng trạng thái.
function flashSaved() {
  status.classList.add("flash");
  clearTimeout(flashSaved.t);
  flashSaved.t = setTimeout(() => status.classList.remove("flash"), 1200);
}

chrome.storage.sync.get({ features: defaults }, ({ features }) => {
  YTT_FEATURES.forEach((f, i) => {
    const row = document.createElement("label");
    row.className = "feature";
    row.style.setProperty("--i", i);
    row.innerHTML =
      `<span class="num"></span><span class="name"></span><input type="checkbox" class="switch">`;
    row.querySelector(".num").textContent = String(i + 1).padStart(2, "0");
    row.querySelector(".name").textContent = f.name;
    row.title = f.description; // Mô tả chỉ hiện khi rê chuột.

    const box = row.querySelector("input");
    box.checked = features[f.id] ?? f.defaultEnabled;
    features[f.id] = box.checked;
    row.classList.toggle("on", box.checked);

    box.addEventListener("change", () => {
      features[f.id] = box.checked;
      row.classList.toggle("on", box.checked);
      updateStatus(features);
      chrome.storage.sync.set({ features }, flashSaved);
    });
    list.appendChild(row);
  });
  updateStatus(features);
});
