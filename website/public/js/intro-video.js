const introVideo = document.querySelector('.intro-video-player');
const player = introVideo.closest('.intro-video');
const controls = player.querySelector('.intro-video-controls');
const play = controls.querySelector('.intro-video-play');
const seek = controls.querySelector('.intro-video-seek');
const time = controls.querySelector('.intro-video-time');
const fullscreen = controls.querySelector('.intro-video-fullscreen');

function formatTime(seconds) {
  const wholeSeconds = Math.floor(seconds);
  const minutes = Math.floor(wholeSeconds / 60);
  return `${minutes}:${String(wholeSeconds % 60).padStart(2, '0')}`;
}

function updateTimeline() {
  const duration = Number.isFinite(introVideo.duration) ? introVideo.duration : 0;
  const { currentTime, seekable } = introVideo;
  const canSeek = seekable.length > 0 && seekable.end(seekable.length - 1) > 0;
  seek.disabled = !canSeek;
  seek.max = duration;
  seek.value = currentTime;
  const position = `${formatTime(currentTime)} / ${formatTime(duration)}`;
  seek.setAttribute('aria-valuetext', position);
  time.textContent = position;
}

function togglePlayback() {
  if (introVideo.paused || introVideo.ended) {
    introVideo.play().catch(updatePlayback);
  } else {
    introVideo.pause();
  }
}

function updatePlayback() {
  const playing = !introVideo.paused && !introVideo.ended;
  play.setAttribute('aria-label', playing ? 'Pause' : 'Play');
  play.toggleAttribute('data-playing', playing);
}

play.addEventListener('click', togglePlayback);
introVideo.addEventListener('click', togglePlayback);
seek.addEventListener('input', () => {
  introVideo.currentTime = seek.valueAsNumber;
  updateTimeline();
});
for (const event of ['loadedmetadata', 'durationchange', 'progress', 'timeupdate', 'emptied']) {
  introVideo.addEventListener(event, updateTimeline);
}
for (const event of ['play', 'pause', 'ended', 'emptied']) {
  introVideo.addEventListener(event, updatePlayback);
}

if (document.fullscreenEnabled) {
  fullscreen.hidden = false;
  fullscreen.addEventListener('click', () => {
    const toggle = document.fullscreenElement === player ? document.exitFullscreen() : player.requestFullscreen();
    toggle.catch(() => {});
  });
  document.addEventListener('fullscreenchange', () => {
    fullscreen.setAttribute('aria-pressed', document.fullscreenElement === player);
  });
}

introVideo.controls = false;
controls.hidden = false;
updateTimeline();
updatePlayback();

async function enableIntroVideoSeeking() {
  const { seekable } = introVideo;
  if (seekable.length && seekable.end(seekable.length - 1) > 0) {
    return;
  }

  // Some static servers ignore byte-range requests. A local blob can still be seeked.
  let videoBlob;
  try {
    const response = await fetch(introVideo.currentSrc);
    if (!response.ok) {
      return;
    }
    videoBlob = await response.blob();
  } catch {
    return;
  }

  const { currentTime, paused, ended } = introVideo;
  introVideo.src = URL.createObjectURL(videoBlob);
  introVideo.currentTime = currentTime;
  if (!paused && !ended) {
    await introVideo.play().catch(() => {});
  }
}

if (introVideo.readyState >= HTMLMediaElement.HAVE_METADATA) {
  enableIntroVideoSeeking();
} else {
  introVideo.addEventListener('loadedmetadata', enableIntroVideoSeeking, { once: true });
}
