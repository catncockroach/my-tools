// Tính năng: đang có quảng cáo thì tắt tiếng và tua nhanh x3, hết quảng cáo thì trả lại như cũ.
// Quảng cáo và video chính dùng chung một thẻ <video>, nên phải nhớ tốc độ/âm thanh của video
// chính để khôi phục. Chỉ tin class "ad-showing" của player (không dùng cách đoán dự phòng
// của isAdShowing), để không bao giờ tua nhầm video chính.
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
  // Lịch sử ngắn trạng thái video chính (khi không có quảng cáo): { rate, muted, at }.
  let history = [];
  // Trạng thái sẽ khôi phục khi hết quảng cáo.
  let saved = { rate: 1, muted: false };

  function adShowing() {
    return !!player && player.classList.contains("ad-showing");
  }

  // Ép tốc độ/tắt tiếng trong lúc quảng cáo (YouTube có thể tự đặt lại tốc độ).
  function enforce() {
    if (!video) return;
    if (video.playbackRate !== AD_RATE) video.playbackRate = AD_RATE;
    if (!video.muted) video.muted = true;
  }

  function update() {
    if (adShowing()) {
      if (!inAd) {
        inAd = true;
        saved = pickSaved();
        YTT.log(`ad-showing bật: đặt tốc độ x${AD_RATE}, tắt tiếng. Đã lưu để khôi phục: x${saved.rate}, muted=${saved.muted}.`);
      }
      enforce();
    } else if (inAd) {
      inAd = false;
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
      detachVideo();
      player = null;
    }
  };
})());
