// Hàm dùng chung cho các tính năng.
var YTT = (() => {
  "use strict";

  const modules = {}; // id -> { start(), stop() }

  function register(id, module) {
    modules[id] = module;
  }

  function isVisible(el) {
    if (!el) return false;
    const style = getComputedStyle(el);
    if (style.display === "none" || style.visibility === "hidden" || style.opacity === "0") return false;
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0;
  }

  function getPlayer() {
    return document.querySelector("#movie_player, .html5-video-player");
  }

  function getVideo() {
    return document.querySelector("#movie_player video, video.html5-main-video, video");
  }

  function isAdShowing() {
    const player = getPlayer();
    return !!player && player.classList.contains("ad-showing");
  }

  function isTyping(target) {
    if (!target) return false;
    if (target.isContentEditable) return true;
    const tag = target.tagName;
    return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
  }

  const SKIP_SELECTORS = [
    ".ytp-skip-ad-button",
    ".ytp-ad-skip-button-modern",
    ".ytp-ad-skip-button",
    ".ytp-ad-skip-button-container button",
    "button[id^='skip-button']",
    ".videoAdUiSkipButton"
  ].join(",");

  function findSkipButton() {
    for (const el of document.querySelectorAll(SKIP_SELECTORS)) {
      if (isVisible(el)) return el;
    }
    return null;
  }

  // Bấm nút Skip. Nếu YouTube bỏ qua click giả lập, tua quảng cáo (đã cho phép skip) tới cuối.
  function skipAd(button) {
    button.click();
    setTimeout(() => {
      if (!isAdShowing() || !findSkipButton()) return;
      const video = getVideo();
      if (video && isFinite(video.duration) && video.duration > 0) {
        video.currentTime = video.duration;
      }
    }, 300);
    toast("⏭ Đã bỏ qua quảng cáo");
  }

  function toast(text) {
    let el = document.getElementById("ytt-toast");
    if (!el) {
      el = document.createElement("div");
      el.id = "ytt-toast";
      Object.assign(el.style, {
        position: "fixed", top: "80px", left: "50%", transform: "translateX(-50%)",
        background: "rgba(0,0,0,.8)", color: "#fff", padding: "8px 16px",
        borderRadius: "6px", font: "500 14px Roboto, Arial, sans-serif",
        zIndex: 2147483647, pointerEvents: "none", transition: "opacity .3s"
      });
      document.documentElement.appendChild(el);
    }
    el.textContent = text;
    el.style.opacity = "1";
    clearTimeout(el._t);
    el._t = setTimeout(() => { el.style.opacity = "0"; }, 1200);
  }

  return { modules, register, isVisible, getPlayer, getVideo, isAdShowing, isTyping, findSkipButton, skipAd, toast };
})();
