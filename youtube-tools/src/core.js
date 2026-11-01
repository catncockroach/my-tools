// Hàm dùng chung cho các tính năng.
var YTT = (() => {
  "use strict";

  const modules = {}; // id -> { start(), stop() }

  // Ghi log ra DevTools Console (lọc bằng "[YTT]") để chẩn đoán khi không hoạt động.
  function log(...args) {
    console.log("%c[YTT]", "color:#e62117;font-weight:bold", ...args);
  }

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
    if (player && (player.classList.contains("ad-showing") || player.classList.contains("ad-interrupting"))) return true;
    // Dự phòng khi YouTube đổi class: lớp phủ quảng cáo đang hiện.
    return [...document.querySelectorAll(".ytp-ad-player-overlay-layout, .ytp-ad-player-overlay, .video-ads .ytp-ad-module > *")]
      .some(isVisible);
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

  // Nút trong vùng quảng cáo (dùng để dò nút Skip khi YouTube đổi class).
  const AD_AREA_BUTTONS = [
    ".video-ads button", ".video-ads [role='button']",
    ".ytp-ad-module button",
    "[class*='ytp-ad'] button", "[class*='ytp-ad'][role='button']",
    "[class*='skip'] button", "button[class*='skip']", "[class*='skip'][role='button']"
  ].join(",");

  function labelOf(el) {
    return [el.className, el.id, el.getAttribute("aria-label"), el.textContent]
      .filter((x) => typeof x === "string").join(" ").replace(/\s+/g, " ").trim();
  }

  // Nút Skip đang hiện. Không phụ thuộc isAdShowing(), vì ở trang tĩnh cuối quảng cáo
  // YouTube có thể đã gỡ class "ad-showing" trong khi nút Skip vẫn còn.
  function findSkipButton() {
    for (const el of document.querySelectorAll(SKIP_SELECTORS)) {
      if (isVisible(el)) return el;
    }
    if (!isAdShowing()) return null; // Dò rộng (nặng hơn) chỉ khi chắc đang có quảng cáo.
    for (const el of document.querySelectorAll(AD_AREA_BUTTONS)) {
      if (/skip|bỏ qua|bo qua/i.test(labelOf(el)) && isVisible(el)) return el;
    }
    return null;
  }

  // In ra các nút đang có trong vùng quảng cáo, để biết cần sửa selector thế nào.
  function describeAdButtons() {
    const player = getPlayer();
    return {
      playerClasses: player ? player.className : "(không thấy #movie_player)",
      buttons: [...document.querySelectorAll(AD_AREA_BUTTONS)].map((el) => ({
        tag: el.tagName.toLowerCase(),
        label: labelOf(el).slice(0, 150),
        visible: isVisible(el)
      }))
    };
  }

  // Giả lập chuỗi sự kiện chuột như người bấm thật (một số nút chỉ nghe pointerdown/mouseup).
  function simulateClick(el) {
    const r = el.getBoundingClientRect();
    const opts = {
      bubbles: true, cancelable: true, composed: true, button: 0,
      clientX: r.left + r.width / 2, clientY: r.top + r.height / 2
    };
    const pointer = { ...opts, pointerId: 1, pointerType: "mouse", isPrimary: true };
    el.dispatchEvent(new PointerEvent("pointerover", pointer));
    el.dispatchEvent(new PointerEvent("pointerdown", pointer));
    el.dispatchEvent(new MouseEvent("mousedown", opts));
    el.dispatchEvent(new PointerEvent("pointerup", pointer));
    el.dispatchEvent(new MouseEvent("mouseup", opts));
    el.click();
  }

  // Bấm nút Skip. Nếu YouTube bỏ qua click giả lập, tua quảng cáo (đã cho phép skip) tới cuối.
  // Thành công = nút Skip biến mất (không dựa vào class "ad-showing").
  function skipAd(button) {
    log("Bấm nút Skip:", labelOf(button).slice(0, 100), "| ad-showing:", isAdShowing());
    simulateClick(button);
    setTimeout(() => {
      if (!findSkipButton()) { log("Đã skip xong (click có tác dụng)."); toast("⏭ Đã bỏ qua quảng cáo"); return; }
      const video = getVideo();
      const canSeek = video && isFinite(video.duration) && video.duration > 0 &&
        !video.ended && video.duration - video.currentTime > 0.5;
      if (!canSeek) {
        log("Click không có tác dụng, và không còn video để tua (trang tĩnh?).",
          video ? { currentTime: video.currentTime, duration: video.duration, ended: video.ended, paused: video.paused } : "không thấy video");
        return;
      }
      log(`Click không có tác dụng, tua quảng cáo tới cuối (${video.currentTime.toFixed(1)}s / ${video.duration.toFixed(1)}s).`);
      video.currentTime = video.duration;
      setTimeout(() => {
        if (findSkipButton()) {
          log("Vẫn còn nút Skip sau khi tua.");
        } else {
          log("Đã tua hết quảng cáo.");
          toast("⏭ Đã bỏ qua quảng cáo");
        }
      }, 1000);
    }, 400);
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

  return { modules, register, log, isVisible, getPlayer, getVideo, isAdShowing, isTyping, findSkipButton, describeAdButtons, skipAd, toast };
})();
