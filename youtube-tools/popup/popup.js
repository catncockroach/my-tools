const list = document.getElementById("list");
const defaults = {};
for (const f of YTT_FEATURES) defaults[f.id] = f.defaultEnabled;

chrome.storage.sync.get({ features: defaults }, ({ features }) => {
  for (const f of YTT_FEATURES) {
    const row = document.createElement("div");
    row.className = "feature";
    row.innerHTML = `<label><span></span><input type="checkbox"></label><p></p>`;
    row.querySelector("span").textContent = f.name;
    row.querySelector("p").textContent = f.description;
    const box = row.querySelector("input");
    box.checked = features[f.id] ?? f.defaultEnabled;
    box.addEventListener("change", () => {
      features[f.id] = box.checked;
      chrome.storage.sync.set({ features });
    });
    list.appendChild(row);
  }
});
