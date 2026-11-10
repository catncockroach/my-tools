// Tính năng: đang có quảng cáo thì tắt tiếng và tua nhanh x3, hết quảng cáo thì trả lại như cũ.
// Quảng cáo và video chính dùng chung một thẻ <video>, nên phải nhớ tốc độ/âm thanh của video
// chính để khôi phục. Chỉ tin class "ad-showing" của player (không dùng cách đoán dự phòng
// của isAdShowing), để không bao giờ tua nhầm video chính.
// Khi đang xem live (kể cả dạng quảng cáo song song, live thu nhỏ chạy cạnh quảng cáo) thì chỉ
// tắt tiếng, không tua nhanh, vì tua nhanh làm live bị lag.
YTT.register("speedUpAd", (() => {
  "use strict";

  const AD_RATE = 3;             // x16 làm video giật, x3 vẫn mượt.
  const FIND_PLAYER_MS = 1000;   // Chờ player xuất hiện / bị thay (YouTube là SPA).
  const SETTLE_MS = 1000;        // YouTube có thể đặt lại x1 ngay trước khi gắn class quảng cáo.

  let player = null;
  let video = null;
  let classObserver = null;
  let findTimer = null;

  let inAd = false;
  let contentLive = false; // Video chính (đo lúc không có quảng cáo) có phải live không.
  let adIsLive = false;    // Quảng cáo hiện tại xuất hiện trong lúc xem live.
  // Lịch sử ngắn trạng thái video chính (khi không có quảng cáo): { rate, muted, at }.
  let history = [];
  // Trạng thái sẽ khôi phục khi hết quảng cáo.
  let saved = { rate: 1, muted: false };

  function adShowing() {
    return !!player && player.classList.contains("ad-showing");
  }

  // Các dấu hiệu cho thấy đang xem live. Trả về danh sách dấu hiệu khớp (rỗng = không phải live).
  function liveSignals() {
    if (!player) return [];
    const signals = [];
    if (player.classList.contains("ytp-live")) signals.push("player.ytp-live");
    if (player.querySelector(".ytp-time-display.ytp-live")) signals.push(".ytp-time-display.ytp-live");
    if (player.querySelectorAll("video").length > 1) signals.push("nhiều thẻ <video>");
    return signals;
  }

  // Ép tốc độ/tắt tiếng trong lúc quảng cáo (YouTube có thể tự đặt lại tốc độ).
  function enforce() {
    if (!video) return;
    if (!adIsLive && liveSignals().length) {
      adIsLive = true;
      YTT.log(`Đang quảng cáo, thấy dấu hiệu live (${liveSignals().join(", ")}): bỏ tua nhanh, chỉ tắt tiếng.`);
    }
    if (adIsLive) {
      if (video.playbackRate === AD_RATE) video.playbackRate = saved.rate;
    } else if (video.playbackRate !== AD_RATE) {
      video.playbackRate = AD_RATE;
    }
    if (!video.muted) video.muted = true;
  }

  function update() {
    if (adShowing()) {
      if (!inAd) {
        inAd = true;
        saved = pickSaved();
        const signals = liveSignals();
        adIsLive = contentLive || signals.length > 0;
        if (contentLive) signals.unshift("video chính là live");
        YTT.log(adIsLive
          ? `ad-showing bật, live (${signals.join(", ")}): chỉ tắt tiếng, giữ tốc độ. Đã lưu để khôi phục: x${saved.rate}, muted=${saved.muted}.`
          : `ad-showing bật: đặt tốc độ x${AD_RATE}, tắt tiếng. Đã lưu để khôi phục: x${saved.rate}, muted=${saved.muted}.`);
      }
      enforce();
    } else if (inAd) {
      inAd = false;
      adIsLive = false;
      restore();
      YTT.log(`ad-showing tắt: khôi phục tốc độ x${saved.rate}, muted=${saved.muted}.`);
    }
  }

  function restore() {
    if (!video) return;
    video.playbackRate = saved.rate;
    video.muted = saved.muted;
  }

  // Ghi nhớ tốc độ/âm thanh người dùng chọn cho video chính.
  function remember() {
    if (inAd || adShowing() || !video) return;
    const last = history[history.length - 1];
    if (last && last.rate === video.playbackRate && last.muted === video.muted) return;
    history.push({ rate: video.playbackRate, muted: video.muted, at: Date.now() });
    if (history.length > 10) history.shift();
  }

  // Trạng thái ổn định gần nhất: bỏ qua các thay đổi trong SETTLE_MS ngay trước quảng cáo.
  function pickSaved() {
    const cutoff = Date.now() - SETTLE_MS;
    const stable = [...history].reverse().find((h) => h.at < cutoff) || history[history.length - 1];
    return stable ? { rate: stable.rate, muted: stable.muted } : { rate: 1, muted: false };
  }

  function onVideoChange() {
    if (adShowing()) { update(); enforce(); } else remember();
  }

  function attachVideo(v) {
    if (v === video) return;
    detachVideo();
    video = v;
    if (!video) return;
    remember();
    video.addEventListener("ratechange", onVideoChange);
    video.addEventListener("volumechange", onVideoChange);
    video.addEventListener("loadedmetadata", onVideoChange);
    video.addEventListener("playing", onVideoChange);
  }

  function detachVideo() {
    if (!video) return;
    video.removeEventListener("ratechange", onVideoChange);
    video.removeEventListener("volumechange", onVideoChange);
    video.removeEventListener("loadedmetadata", onVideoChange);
    video.removeEventListener("playing", onVideoChange);
    video = null;
  }

  // Tìm player/video; theo dõi class của player để phản ứng ngay khi quảng cáo bắt đầu/kết thúc.
  function findPlayer() {
    const p = document.querySelector("#movie_player");
    if (p !== player) {
      if (classObserver) classObserver.disconnect();
      player = p;
      if (player) {
        classObserver = new MutationObserver(update);
        classObserver.observe(player, { attributes: true, attributeFilter: ["class"] });
      }
    }
    attachVideo(player ? player.querySelector("video") : null);
    if (player && !adShowing()) contentLive = liveSignals().length > 0;
    update();
  }

  return {
    start() {
      if (findTimer) return;
      findPlayer();
      findTimer = setInterval(findPlayer, FIND_PLAYER_MS);
    },
    stop() {
      clearInterval(findTimer);
      findTimer = null;
      if (classObserver) classObserver.disconnect();
      classObserver = null;
      if (inAd) restore(); // Tắt giữa quảng cáo -> trả lại ngay.
      inAd = false;
      adIsLive = false;
      contentLive = false;
      detachVideo();
      player = null;
    }
  };
})());
