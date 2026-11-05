const list = document.getElementById("list");
const footer = document.getElementById("footer");
const defaults = {};
for (const f of YTT_FEATURES) defaults[f.id] = f.defaultEnabled;

document.getElementById("version").textContent = "v" + chrome.runtime.getManifest().version;

// Cập nhật dòng "x/y đang bật" và hiệu ứng ở logo.
function updateStatus(features) {
  const on = YTT_FEATURES.filter((f) => features[f.id]).length;
  document.getElementById("count").textContent =
    on ? `${on}/${YTT_FEATURES.length} tính năng đang bật` : "Đang tắt tất cả";
  document.body.classList.toggle("active", on > 0);
}

// Hiện "✓ Đã lưu" thoáng qua ở footer.
function flashSaved() {
  footer.classList.add("flash");
  clearTimeout(flashSaved.t);
  flashSaved.t = setTimeout(() => footer.classList.remove("flash"), 1400);
}

chrome.storage.sync.get({ features: defaults }, ({ features }) => {
  YTT_FEATURES.forEach((f, i) => {
    const row = document.createElement("label");
    row.className = "feature";
    row.style.setProperty("--i", i);
    row.innerHTML =
      `<span class="num"></span><span class="name"></span>` +
      `<input type="checkbox" class="switch"><p class="desc"></p>`;
    row.querySelector(".num").textContent = String(i + 1).padStart(2, "0");
    row.querySelector(".name").textContent = f.name;
    row.querySelector(".desc").textContent = f.description;

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
