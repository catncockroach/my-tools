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

  // Nút có thực sự nằm trên cùng (không bị lớp khác che) tại điểm giữa của nó không.
  function isOnTop(el) {
    const r = el.getBoundingClientRect();
    const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    return !!hit && (hit === el || el.contains(hit) || hit.contains(el));
  }

  // Mọi nút Skip đang hiện, nút nằm trên cùng xếp trước.
  // Không phụ thuộc isAdShowing(), vì ở trang ảnh tĩnh cuối quảng cáo YouTube có thể đã gỡ
  // class "ad-showing" trong khi nút Skip vẫn còn. Có thể có nhiều nút cùng lúc (nút cũ của
  // quảng cáo trước còn sót lại, bị che), nên không chỉ lấy nút đầu tiên.
  function findSkipButtons() {
    const found = new Set();
    for (const el of document.querySelectorAll(SKIP_SELECTORS)) {
      if (isVisible(el)) found.add(el);
    }
    // Dò rộng theo nhãn, chỉ trong player cho nhẹ.
    const player = getPlayer();
    if (player) {
      for (const el of player.querySelectorAll(AD_AREA_BUTTONS)) {
        if (/skip|bỏ qua|bo qua/i.test(labelOf(el)) && isVisible(el)) found.add(el);
      }
    }
    // Giữ phần tử trong cùng (click sẽ nổi bọt lên phần tử cha), bỏ phần tử bọc ngoài.
    const list = [...found].filter((el) => ![...found].some((o) => o !== el && el.contains(o)));
    return list.sort((a, b) => isOnTop(b) - isOnTop(a));
  }

  function findSkipButton() {
    return findSkipButtons()[0] || null;
  }

  // In ra các nút đang có trong vùng quảng cáo, để biết cần sửa selector thế nào.
  function describeAdButtons() {
    const player = getPlayer();
    const video = getVideo();
    const describe = (el) => {
      const r = el.getBoundingClientRect();
      const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      return {
        tag: el.tagName.toLowerCase(),
        label: labelOf(el).slice(0, 150),
        visible: isVisible(el),
        onTop: isVisible(el) && isOnTop(el),
        coveredBy: hit && !isOnTop(el) ? labelOf(hit).slice(0, 100) : undefined
      };
    };
    return {
      playerClasses: player ? player.className : "(không thấy #movie_player)",
      video: video ? { currentTime: video.currentTime, duration: video.duration, ended: video.ended, paused: video.paused } : null,
      skipButtons: [...document.querySelectorAll(SKIP_SELECTORS)].map(describe),
      adButtons: [...document.querySelectorAll(AD_AREA_BUTTONS)].map(describe)
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

  const CLICK_EVERY_MS = 250;     // Nút Skip còn thì cứ 250 ms bấm lại, không nghỉ giữa chừng.
  const SEEK_AFTER_CLICKS = 2;    // Bấm 2 lần chưa được mà quảng cáo còn đang chạy -> tua tới cuối luôn.
  const GIVE_UP_AFTER_MS = 10000; // Bấm liên tục 10 giây không được thì dừng (tránh bấm mãi).
  let skipping = false;           // Đang trong một lượt skip (tránh auto-skip và phím bấm chồng nhau).

  function isSkipping() { return skipping; }

  // Video quảng cáo còn đang chạy và tua được (không phải màn hình kết thúc).
  function seekableAd() {
    const video = getVideo();
    return video && isFinite(video.duration) && video.duration > 0 &&
      !video.ended && video.duration - video.currentTime > 0.5 ? video : null;
  }

  // Bấm mọi nút Skip đang hiện (nút trên cùng trước), lặp lại mỗi CLICK_EVERY_MS cho tới khi
  // nút biến mất. Lần bấm đầu đôi khi chỉ dừng quảng cáo / chuyển sang màn hình kết thúc,
  // nên phải bấm tiếp. Nếu YouTube bỏ qua click giả lập khi quảng cáo còn chạy, tua tới cuối.
  // Thành công = nút Skip biến mất. onEnd(ok) được gọi khi kết thúc.
  // Trả về false nếu đang có một lượt skip khác chạy.
  function skipAd(onEnd) {
    if (skipping) return false;
    skipping = true;
    const t0 = performance.now();
    const ms = () => Math.round(performance.now() - t0);
    let clicks = 0;
    let seekedAt = -1; // Mốc (ms) đã tua video tới cuối, -1 = chưa tua.

    const finish = (ok, msg, extra) => {
      skipping = false;
      extra === undefined ? log(msg) : log(msg, extra);
      if (ok) toast("⏭ Đã bỏ qua quảng cáo");
      if (typeof onEnd === "function") onEnd(ok);
    };

    const tick = () => {
      const buttons = findSkipButtons();
      if (!buttons.length) {
        return finish(true, `+${ms()}ms: không còn nút Skip. Đã bấm ${clicks} lần${seekedAt >= 0 ? `, đã tua video lúc +${seekedAt}ms` : ", không tua"}.`);
      }
      if (ms() > GIVE_UP_AFTER_MS) {
        return finish(false, `+${ms()}ms: quá ${GIVE_UP_AFTER_MS / 1000}s, dừng bấm. Đã bấm ${clicks} lần, nút Skip vẫn còn. Trạng thái:`, describeAdButtons());
      }

      clicks++;
      if (clicks <= 3 || clicks % 4 === 0) { // Đủ để chẩn đoán, không spam console.
        log(`+${ms()}ms: bấm lần ${clicks} vào ${buttons.length} nút Skip:`,
          buttons.map((b) => labelOf(b).slice(0, 80) + (isOnTop(b) ? "" : " [bị che]")).join(" | "),
          "| ad-showing:", isAdShowing());
      }
      for (const b of buttons) simulateClick(b);

      const video = seekedAt < 0 && clicks > SEEK_AFTER_CLICKS && seekableAd();
      if (video) {
        seekedAt = ms();
        log(`+${ms()}ms: nút Skip vẫn còn sau ${clicks - 1} lần bấm, tua video ${video.currentTime.toFixed(1)}s -> ${video.duration.toFixed(1)}s.`);
        video.currentTime = video.duration;
      }
      setTimeout(tick, CLICK_EVERY_MS);
    };

    tick();
    return true;
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

  return { modules, register, log, isVisible, getPlayer, getVideo, isAdShowing, isTyping, findSkipButton, findSkipButtons, describeAdButtons, skipAd, isSkipping, toast };
})();
