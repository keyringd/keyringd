const canvas = document.querySelector('canvas');
const g = canvas.getContext('2d', { alpha: false });
const appIcon = new Image();
appIcon.src = '/logo.svg';
const personNormalImage = new Image();
personNormalImage.src = '/assets/person-normal-flat.png';
const personWorriedImage = new Image();
personWorriedImage.src = '/assets/person-worried-flat.png';
const attackerImage = new Image();
attackerImage.src = '/assets/attacker-hooded-flat.png';
const hardwareStart = 7.5;
const replacementStart = 14.5;
const caSetupStart = 22.5;
const setupOffset = 7.5;
const finalCaptionStartOffset = 10.55;
const titleSeconds = 2;
const storySeconds = 44;
const durationSeconds = titleSeconds + storySeconds;
const fullCutSeconds = 60;
const fullCutSpeed = durationSeconds / fullCutSeconds;
const finalHoldSeconds = fullCutSpeed;
const setupStart = storySeconds - finalCaptionStartOffset - finalHoldSeconds;
const keyringdReplacementStart = setupStart - 2.5;
const homepageSeconds = 29.25;

const c = {
  ink: '#102c3b', blue: '#315e73', muted: '#5f7885', accent: '#d3a354',
  status: '#315e73', deviceLine: '#91b4c5', deviceFill: '#edf3f5',
  paper: '#f7fafb', line: '#cad8de', steel: '#91b4c5', brassLight: '#f5dfb9',
  danger: '#9b4a46', navy: '#0d2f43', navy2: '#123b50', white: '#ffffff',
};
const clamp = n => Math.max(0, Math.min(1, n));
const ease = n => 1 - (1 - clamp(n)) ** 3;
const smooth = n => clamp(n) ** 2 * (3 - 2 * clamp(n));
const rise = (t, start, duration = .57) => ease((t - start) / duration);

function paintBackground() {
  g.fillStyle = c.paper;
  g.fillRect(0, 0, canvas.width, canvas.height);
  g.save();
  g.strokeStyle = 'rgba(49, 94, 115, .075)';
  g.lineWidth = 1;
  for (let x = 0; x <= canvas.width; x += 64) {
    g.beginPath(); g.moveTo(x, 0); g.lineTo(x, canvas.height); g.stroke();
  }
  for (let y = 0; y <= canvas.height; y += 64) {
    g.beginPath(); g.moveTo(0, y); g.lineTo(canvas.width, y); g.stroke();
  }
  g.strokeStyle = 'rgba(49, 94, 115, .11)';
  g.beginPath(); g.arc(1120, 10, 390, 0, 2 * Math.PI); g.stroke();
  g.restore();
}

function rect(x, y, w, h, r, fill, stroke, width = 1.5) {
  g.beginPath();
  g.roundRect(x, y, w, h, r);
  if (fill) { g.fillStyle = fill; g.fill(); }
  if (stroke) { g.strokeStyle = stroke; g.lineWidth = width; g.stroke(); }
}

function text(label, x, y, size, { color = c.ink, weight = 600, mono = false, align = 'left', spacing = 0 } = {}) {
  g.fillStyle = color;
  g.font = `${weight} ${size}px ${mono ? 'Consolas, monospace' : 'Segoe UI, sans-serif'}`;
  g.textAlign = align;
  g.textBaseline = 'alphabetic';
  g.letterSpacing = spacing + 'px';
  g.fillText(label, x, y);
  g.letterSpacing = '0px';
}

function captionText(label) {
  const lines = label.split('\n');
  const lineHeight = 29;
  const firstBaseline = 641 - (lines.length - 1) * lineHeight / 2;
  lines.forEach((line, index) => {
    text(line, 640, firstBaseline + index * lineHeight, 26, {
      align: 'center', color: c.muted, weight: 550,
    });
  });
}

function stroke(points, color, width = 3) {
  g.beginPath();
  points.forEach(([x, y], i) => i ? g.lineTo(x, y) : g.moveTo(x, y));
  g.strokeStyle = color;
  g.lineWidth = width;
  g.lineCap = 'round';
  g.lineJoin = 'round';
  g.stroke();
}

function dot(x, y, r, fill) {
  g.beginPath();
  g.arc(x, y, r, 0, 2 * Math.PI);
  g.fillStyle = fill;
  g.fill();
}

function float(p, draw, lift = 22) {
  if (p <= 0) return;
  g.save();
  g.globalAlpha *= p;
  g.translate(0, (1 - p) * lift);
  draw();
  g.restore();
}

function check(x, y, color = c.status, scale = 1) {
  stroke([[x - 8 * scale, y], [x - 2 * scale, y + 6 * scale], [x + 9 * scale, y - 7 * scale]], color, 3 * scale);
}

function key(x, y, color, scale = 1) {
  g.save();
  g.translate(x, y);
  g.scale(scale, scale);
  g.beginPath();
  g.arc(-8, -8, 8, 0, 2 * Math.PI);
  g.strokeStyle = color;
  g.lineWidth = 3;
  g.stroke();
  stroke([[-2, -2], [15, 15], [20, 10]], color);
  stroke([[8, 8], [13, 3]], color);
  g.restore();
}

function phone(t) {
  rect(112, 258, 143, 240, 24, c.deviceFill, c.deviceLine, 2);
  rect(122, 270, 123, 216, 16, '#ffffff', '#d9dfdc');
  rect(168, 278, 31, 4, 2, '#819aa7');
  stroke([[108, 327], [108, 348]], c.deviceLine, 2);
  stroke([[256, 331], [256, 357]], c.deviceLine, 2);
  key(183, 349, c.accent, 1.16);
  text('Passkey', 183, 404, 22, { align: 'center', color: '#6a501e' });
  if (t >= 8.5 && t < 12.6) {
    const signed = t >= 9.55;
    const press = smooth((t - 9.15) / .18) * (1 - smooth((t - 9.43) / .16));
    g.save();
    g.translate(183, 443);
    g.scale(1 - .07 * press, 1 - .09 * press);
    rect(-49, -16, 98, 32, 7, press > .65 ? '#d7e6f0' : '#edf4f8', '#b6cbd7');
    if (signed) check(-29, 0, c.accent, .53);
    text(signed ? 'Signed' : 'Approve', signed ? 8 : 0, 6, 17, { align: 'center', color: '#6a501e' });
    g.restore();
  }
}

function chip(x, y, color) {
  rect(x - 21, y - 21, 42, 42, 7, '#ffffff', color, 1.6);
  for (const offset of [-10, 0, 10]) {
    stroke([[x + offset, y - 21], [x + offset, y - 27]], color, 1.6);
    stroke([[x + offset, y + 21], [x + offset, y + 27]], color, 1.6);
    stroke([[x - 21, y + offset], [x - 27, y + offset]], color, 1.6);
    stroke([[x + 21, y + offset], [x + 27, y + offset]], color, 1.6);
  }
  key(x, y - 1, color, .58);
}

function certificate(x, y) {
  rect(x, y - 16, 23, 29, 4, c.brassLight, c.accent, 1.7);
  check(x + 11, y - 1, '#6a501e', .55);
  text('SSH certificate', x + 33, y + 6, 21, { color: '#6a501e' });
}

function laptop(t, stage, focus = 0, labelOpacity = 1) {
  const swap = stage === 'replacement' ? smooth((t - 1.4) / 1.2) : 0;
  const newComputer = stage === 'keyringd' || (stage === 'replacement' && t >= 2);
  const opacity = 1 - Math.sin(swap * Math.PI);
  const border = newComputer ? '#b87559' : '#9bb0b7';
  g.save();
  g.globalAlpha *= opacity;
  g.translate((newComputer ? 1 : -1) * (1 - opacity) * 34, 0);
  float(labelOpacity * opacity, () => {
    text(newComputer ? 'New computer' : 'Current computer', 640, 211, 22, { align: 'center' });
  }, 0);
  rect(447, 255, 386, 241, 15, '#ffffff', border, 2);
  dot(640, 265, 2.2, border);
  rect(460, 275, 360, 207, 8, '#ffffff', '#d9dfdc');
  switch (stage) {
    case 'threat': dependencyScreen(t); break;
    case 'hardware': hardwareScreen(t, false); break;
    case 'replacement': hardwareScreen(t, newComputer); break;
    case 'ca-setup':
    case 'keyringd':
      keyringdWindow(t, newComputer, focus);
      float(rise(t, 13, .35), () => sshTerminalWindow(t), 4);
      break;
  }
  g.beginPath();
  g.moveTo(430, 496);
  g.lineTo(850, 496);
  g.lineTo(842, 509);
  g.quadraticCurveTo(838, 515, 828, 515);
  g.lineTo(452, 515);
  g.quadraticCurveTo(442, 515, 438, 509);
  g.closePath();
  g.fillStyle = newComputer ? '#e8cbb8' : '#d7e1e1';
  g.fill();
  g.strokeStyle = border;
  g.lineWidth = 2;
  g.stroke();
  stroke([[613, 499], [667, 499]], border, 2);
  g.restore();
}

function keyringdWindow(t, newComputer, focus) {
  rect(484, 285, 312, 120, 7, '#ffffff', '#c7d5d9');
  rect(485, 286, 310, 28, [6, 6, 0, 0], '#f2f5f5');
  g.drawImage(appIcon, 494, 290, 20, 20);
  text('Keyringd', 520, 306, 17, { weight: 650 });
  deviceKey(newComputer);
  float(rise(t, 11.85, .45) * (1 - focus), () => certificate(502, 385), 4);
}

function deviceKey(newComputer) {
  const color = newComputer ? c.accent : c.status;
  g.save();
  g.translate(514, 343);
  g.scale(.85, .85);
  chip(0, 0, color);
  g.restore();
  text('TPM / Secure Enclave', 548, 340, 19, { color: c.muted });
  text(newComputer ? 'New device private key' : 'Device private key', 548, 363, 21, { color });
}

function sshTerminalWindow(t) {
  const connected = t >= 18.05;
  const host = 'example.com';
  const command = 'ssh user@' + host;
  const typed = Math.floor(clamp((t - 13.2) / 1.75) * command.length);
  rect(484, 413, 312, 61, 7, '#163649');
  rect(484, 413, 312, 19, [7, 7, 0, 0], '#294d62');
  text('Terminal', 497, 427, 15, { color: '#ffffff' });
  text('$ ' + command.slice(0, typed), 497, 449, 17, { mono: true, color: '#ffffff' });
  if (connected) {
    text('Connected to ' + host, 497, 466, 17, { mono: true, color: '#bedbf1' });
  }
}

function dependencyScreen(t) {
  rect(489, 301, 302, 76, 8, '#f2f5f5', '#d9dfdc');
  text('Developer workstation', 503, 330, 19, { weight: 700 });
  text('Malicious process running…', 503, 357, 18, { color: c.danger });
  keyFile(514, 428, 1);
  text('SSH private key file', 542, 434, 20, { color: c.status });
}

function keyFile(x, y, scale) {
  g.save();
  g.translate(x, y);
  g.scale(scale, scale);
  paintPath('M-13-17h16l10 10v25h-26Z M3-17v10h10', '#ffffff', '#899fa9');
  key(-1, 4, c.status, .39);
  g.restore();
}

function stolenCopy(t) {
  if (t < 3.45) return;
  const move = smooth((t - 3.45) / 2.2);
  const x = 514 + 494 * move;
  const y = 428 + 61 * move + Math.sin(Math.PI * move) * 76;
  float(rise(t, 5.62, .42), () => {
    rect(983, 466, 178, 46, 9, '#fff7f5', '#d6aaa6', 1.4);
    text('Stolen copy', 1090, 495, 18, {
      align: 'center', color: '#7f3432', weight: 700,
    });
  }, 5);
  float(rise(t, 3.45, .4), () => keyFile(x, y, .87), 0);
}

function attacker(t) {
  float(rise(t, 3.45, .62), () => {
    text('Attacker', 1072, 211, 22, { align: 'center' });
    const bob = Math.sin(Math.PI * smooth((t - 3.45) / 1.2)) * 6;
    drawCharacterImage(attackerImage, [250, 70, 830, 1070], 965, 258 - bob, 215);
  }, 8);
}
function hardwareScreen(t, newComputer) {
  deviceKey(newComputer);
  const command = 'ssh user@example.com';
  const typed = newComputer ? Math.floor(clamp((t - 2.42) / .84) * command.length) : command.length;
  rect(484, 405, 312, 65, 6, '#163649');
  text('$ ' + command.slice(0, typed), 497, 429, 17, { mono: true, color: '#ffffff' });
  if (!newComputer) text('Connected to example.com', 497, 452, 17, { mono: true, color: '#bedbf1' });
  if (newComputer && t >= 3.42) {
    text('Permission denied (publickey).', 497, 452, 17, { mono: true, color: '#ead7c8' });
  }
}

function originalLaptop(t) {
  float(rise(t, 2.22, .55), () => {
    rect(266, 412, 132, 77, 7, '#ffffff', '#9bb0b7', 1.6);
    rect(257, 489, 150, 8, [0, 0, 4, 4], '#d7e1e1', '#9bb0b7');
    g.save();
    g.translate(332, 450);
    g.scale(.7, .7);
    chip(0, 0, c.status);
    g.restore();
    text('Private key stays in', 332, 518, 18, { align: 'center', color: c.muted });
    text('the old computer', 332, 540, 18, { align: 'center', color: c.muted });
  }, 10);
}

function paintPath(data, fill, outline) {
  const shape = new Path2D(data);
  if (fill) { g.fillStyle = fill; g.fill(shape); }
  g.strokeStyle = outline;
  g.lineWidth = 2;
  g.lineJoin = 'round';
  g.stroke(shape);
}

function drawCharacterImage(image, [sourceX, sourceY, sourceWidth, sourceHeight], x, y, width) {
  const height = width * sourceHeight / sourceWidth;
  g.drawImage(image, sourceX, sourceY, sourceWidth, sourceHeight, x, y, width, height);
}

function person(worried, relieved) {
  g.save();
  const angle = -.025 * worried + .018 * relieved;
  g.translate(163, 405);
  g.rotate(angle);
  g.translate(-163, -405);
  if (worried < 1) {
    g.globalAlpha *= 1 - worried;
    drawCharacterImage(personNormalImage, [220, 80, 820, 1065], 73, 294, 180);
    g.globalAlpha /= Math.max(.001, 1 - worried);
  }
  if (worried > 0) {
    g.globalAlpha *= worried;
    drawCharacterImage(personWorriedImage, [220, 80, 820, 1065], 73, 294, 180);
  }
  g.restore();
  float(worried, () => {
    text('!', 263, 312, 32, { align: 'center', color: c.status, weight: 650 });
  }, 6);
}
function server(t, trustChange, replacement) {
  text('SSH Server', 1090, 211, 22, { align: 'center' });
  for (const y of [273, 314]) {
    rect(993, y, 194, 32, 6, '#ffffff', c.deviceLine, 1.8);
    dot(1009, y + 16, 3.1, t >= 18.05 ? '#41665c' : '#819aa7');
    stroke([[1024, y + 16], [1048, y + 16]], '#819aa7', 1.9);
    stroke([[1152, y + 16], [1172, y + 16]], '#819aa7', 1.9);
  }
  rect(986, 363, 208, 76, 9, c.deviceFill, '#bdcbcf', 1.5);
  float(1 - trustChange, () => {
    text('Original computer’s', 1090, 391, 19, { align: 'center' });
    text('public key', 1090, 419, 19, { align: 'center', color: c.status });
  }, -6);
  float(trustChange, () => {
    text('Passkey public key', 1090, 391, 19, { align: 'center' });
    text('as Trusted SSH CA', 1090, 419, 19, { align: 'center', color: c.status });
  }, 6);
  if (!replacement) {
    float(trustChange, () => {
      text('Configure once', 1090, 474, 19, {
        align: 'center', color: '#41665c', weight: 650,
      });
    }, 0);
  }
  if (replacement && t < 18.05) {
    text('No changes needed', 1090, 474, 19, {
      align: 'center', color: '#41665c', weight: 650,
    });
  }
  if (t >= 18.05) {
    float(rise(t, 18.05, .4), () => {
      check(1034, 467, '#41665c', .8);
      text('Verified', 1052, 474, 20, { color: '#41665c', weight: 700 });
    }, 5);
  }
}

function transfer(x, end, y, title, color, progress) {
  stroke([[x, y], [end, y]], color, 1.7);
  const direction = Math.sign(end - x);
  stroke([[end - 8 * direction, y - 6], [end, y], [end - 8 * direction, y + 6]], color, 1.7);
  if (progress >= 0 && progress <= 1) dot(x + (end - x) * progress, y, 4.5, color);
  text(title, (x + end) / 2, y - 14, 17, { align: 'center', color, weight: 650 });
}

const captions = [
  [0, 'If your computer is compromised, malware can copy an SSH private key stored as a file\nand send it to an attacker.'],
  [hardwareStart, 'One way to protect against private-key theft is to generate the key inside a TPM or Secure Enclave.\nThe hardware signs without exposing the private key.'],
  [replacementStart, 'But keeping the key in hardware creates a problem when you replace your computer.\nThe new computer needs a new key, so every server that trusts the old key directly must be updated.'],
  [caSetupStart, 'With Keyringd, servers trust the passkey’s public key as an SSH CA.\nEach computer uses its own hardware-backed device key for everyday SSH.'],
  [keyringdReplacementStart, 'On a new computer, Keyringd creates another hardware-backed device key.\nThe same passkey certifies it, so no server settings need to change.'],
  [setupStart + finalCaptionStartOffset, 'Everyday SSH uses the device key and certificate—without moving a private key\nor approving every connection with the passkey.'],
];

let titleCard;

function createTitleCard() {
  drawBookend(false);
  const card = document.createElement('canvas');
  card.width = canvas.width;
  card.height = canvas.height;
  const context = card.getContext('2d', { alpha: false });
  context.fillStyle = c.navy;
  context.fillRect(0, 0, card.width, card.height);
  context.strokeStyle = 'rgba(145, 180, 197, .12)';
  context.lineWidth = 1;
  for (let x = 0; x <= card.width; x += 64) {
    context.beginPath(); context.moveTo(x, 0); context.lineTo(x, card.height); context.stroke();
  }
  for (let y = 0; y <= card.height; y += 64) {
    context.beginPath(); context.moveTo(0, y); context.lineTo(card.width, y); context.stroke();
  }
  context.strokeStyle = 'rgba(145, 180, 197, .18)';
  context.beginPath(); context.arc(1070, 15, 430, 0, 2 * Math.PI); context.stroke();
  const name = 'Keyringd';
  const nameFont = '700 128px "Segoe UI", sans-serif';
  const subtitleFont = '650 102px "Segoe UI", sans-serif';
  context.font = nameFont;
  const nameMetrics = context.measureText(name);
  const nameHeight = nameMetrics.actualBoundingBoxAscent + nameMetrics.actualBoundingBoxDescent;
  const nameWidth = nameMetrics.actualBoundingBoxLeft + nameMetrics.actualBoundingBoxRight;
  const iconSize = 152;
  const iconGap = 30;
  const logoHeight = Math.max(iconSize, nameHeight);
  const logoWidth = iconSize + iconGap + nameWidth;
  const lineGap = 37;
  const subtitle = 'in 60 seconds';
  context.font = subtitleFont;
  context.letterSpacing = '-2px';
  const { actualBoundingBoxAscent: ascent, actualBoundingBoxDescent: descent } =
    context.measureText(subtitle);
  const blockHeight = logoHeight + lineGap + ascent + descent;
  const center = card.width / 2;
  const top = (card.height - blockHeight) / 2;
  const left = center - logoWidth / 2;
  context.drawImage(appIcon, left, top + (logoHeight - iconSize) / 2, iconSize, iconSize);
  context.fillStyle = c.white;
  context.font = nameFont;
  context.letterSpacing = '0px';
  context.fillText(name, left + iconSize + iconGap + nameMetrics.actualBoundingBoxLeft,
    top + (logoHeight - nameHeight) / 2 + nameMetrics.actualBoundingBoxAscent);
  context.textAlign = 'center';
  context.font = subtitleFont;
  context.letterSpacing = '-2px';
  context.fillStyle = c.steel;
  context.fillText(subtitle, center, top + logoHeight + lineGap + ascent);
  context.font = '800 18px "Segoe UI", sans-serif';
  context.letterSpacing = '2px';
  context.fillStyle = c.accent;
  context.fillText('HARDWARE-BACKED SSH KEYS', center, top - 50);
  return card;
}

function draw(t) {
  if (t < titleSeconds) {
    g.drawImage(titleCard, 0, 0);
    const fadeToPaper = smooth((t - 1.44) / .56);
    if (fadeToPaper > 0) {
      g.save();
      g.globalAlpha = fadeToPaper;
      g.fillStyle = c.paper;
      g.fillRect(0, 0, canvas.width, canvas.height);
      g.restore();
    }
    return;
  }
  const storyTime = t - titleSeconds;
  drawStory(storyTime);
  const reveal = smooth(storyTime / .48);
  if (reveal < 1) {
    g.save();
    g.globalAlpha = 1 - reveal;
    g.fillStyle = c.paper;
    g.fillRect(0, 0, canvas.width, canvas.height);
    g.restore();
  }
}

const homepageProblem = 'Each server still trusts the old computer’s public key.';
const failedReplacement = 4.25;

function drawHomepage(t) {
  const caRegistration = 7.4;
  const upgrade = 11.45;
  const passkeySetup = 13.75;
  if (t < 4.25) {
    drawHomepageOpening(t);
    return;
  }
  if (t < caRegistration) {
    drawStory(replacementStart + failedReplacement, homepageProblem);
    return;
  }
  if (t < upgrade) {
    drawStory(caSetupStart + t - caRegistration);
    return;
  }
  if (t < passkeySetup) {
    drawStory(keyringdReplacementStart + t - upgrade);
    return;
  }
  drawStory(setupStart + t - passkeySetup);
  float(smooth((t - 26.9) / .55), () => drawBookend(true), 0);
}

function drawHomepageOpening(t) {
  if (t <= .36) {
    drawBookend(false);
    return;
  }
  const push = smooth((t - .36) / 1.1);
  const pull = smooth((t - 2.38) / 1.8);
  const focus = push * (1 - pull);
  const failure = smooth((t - 1.02) / .48);
  const diagramY = 375 + 46 * failure;
  const terminalY = diagramY + 60;
  const zoom = 1 + .9 * focus;
  paintBackground();
  g.save();
  g.translate(648, terminalY + (380 - terminalY) * focus);
  g.scale(zoom, zoom);
  g.translate(-648, -terminalY);
  g.translate(648, diagramY);
  g.scale(1.05, 1.05);
  g.translate(-650, -380);
  float(1 - smooth((t - .42) / .76), () => {
    phone(0);
    server(12.7, 1, true);
    transfer(263, 442, 367, 'Passkey signature', c.status);
    transfer(838, 980, 404, 'SSH', c.status);
  }, 0);
  if (failure < 1) laptop(12.7, 'keyringd', focus, 1 - focus);
  float(failure, () => laptop(failedReplacement, 'replacement', 0, pull), 0);
  float(smooth((pull - .30) / .66), () => {
    person(1, 0);
    originalLaptop(failedReplacement);
    server(failedReplacement, 0, false);
  }, 0);
  g.restore();
  float(smooth((t - 1.25) / .45) * (1 - smooth((t - 3.15) / .47)), () => {
    text('New computer. Why won’t SSH connect?', 640, 644, 32, {
      align: 'center', color: c.muted, weight: 650,
    });
  }, 0);
  float(smooth((t - 3.5) / .62), () => {
    text('A new computer needs a new key.', 74, 136, 58, { weight: 650, spacing: -1.2 });
    text(homepageProblem, 640, 644, 27, { align: 'center', color: c.muted, weight: 550 });
  }, 0);
}

function drawBookend(connected) {
  paintBackground();
  g.save();
  g.translate(648, 375);
  g.scale(1.05, 1.05);
  g.translate(-650, -380);
  float(connected ? .28 : 1, () => {
    phone(0);
    transfer(263, 442, 367, 'Passkey signature', c.status);
  }, 0);
  const deviceTime = connected ? 19.3 : 12.7;
  laptop(deviceTime, 'keyringd');
  server(deviceTime, 1, true);
  transfer(838, 980, 404, 'SSH', c.status);
  g.restore();
}

function drawStory(t, captionOverride) {
  const stage = t < hardwareStart ? 'threat' : t < replacementStart ? 'hardware' :
    t < caSetupStart ? 'replacement' : t < keyringdReplacementStart ? 'ca-setup' : 'keyringd';
  const local = stage === 'replacement' ? t - replacementStart :
    stage === 'keyringd' ? setupOffset + Math.max(0, t - setupStart) :
    stage === 'ca-setup' ? t - caSetupStart :
    stage === 'hardware' ? t - hardwareStart : t;
  const title = {
    threat: 'Private key files can be stolen.',
    hardware: 'Hardware keeps the private key safe.',
    replacement: 'But hardware-bound keys can’t move.',
    'ca-setup': 'Trust the passkey, not one computer.',
    keyringd: 'New computer. New device key. Same server setup.',
  }[stage];
  const stageStart = {
    threat: 0, hardware: hardwareStart, replacement: replacementStart,
    'ca-setup': caSetupStart, keyringd: keyringdReplacementStart,
  }[stage];
  paintBackground();
  const titleSize = stage === 'keyringd' ? 46 : stage === 'ca-setup' ? 54 : 58;
  float(stageStart === 0 ? 1 : rise(t, stageStart, .34), () => {
    text(title, 74, 136, titleSize, { weight: 650, spacing: -1.2 });
  }, 6);
  if (stage === 'ca-setup') {
    float(rise(t, caSetupStart, .34), () => {
      text('KEYRINGD’S TRUST MODEL', 74, 177, 16, {
        color: c.accent, weight: 800, spacing: 1.6,
      });
    }, 4);
  }
  g.save();
  g.translate(648, 421);
  g.scale(1.05, 1.05);
  g.translate(-650, -380);
  const replacementWorry = rise(t, replacementStart, .38) *
    (1 - rise(t, caSetupStart, 1.05));
  const threatWorry = stage === 'threat' ? rise(t, 3.45, .45) : 0;
  const worry = Math.max(replacementWorry, threatWorry);
  person(worry, rise(t, setupStart + finalCaptionStartOffset, .55));
  const laptopArrival = stage === 'keyringd' ? rise(t, keyringdReplacementStart, .55) : 1;
  const computerLabelOpacity = stage === 'threat' || stage === 'hardware' ? 0 : 1;
  float(laptopArrival, () => laptop(local, stage, 0, computerLabelOpacity), 14);
  if (stage !== 'threat') {
    server(local, smooth((t - caSetupStart - .55) / .75), stage === 'keyringd');
  }
  if (stage === 'threat') {
    attacker(local);
    stolenCopy(local);
  }
  if (stage === 'hardware' && local >= 3.7) {
    transfer(838, 980, 400, 'SSH signature', c.status, smooth((local - 3.7) / 2));
  }
  if (stage === 'replacement') originalLaptop(local);
  if (stage === 'ca-setup' || stage === 'keyringd') {
    float(rise(t, caSetupStart + .15, .55), () => {
      g.save();
      if (stage === 'keyringd') g.globalAlpha *= 1 - .72 * smooth((local - 13) / .72);
      g.translate(181, 167);
      g.scale(.67, .67);
      phone(stage === 'keyringd' ? local : 0);
      g.restore();
    }, 8);
  }
  if (stage === 'keyringd') {
    if (local >= 7.8 && local < 9.05) {
      transfer(443, 357, 403, 'Request', c.status, ease((local - 7.8) / 1.25));
    }
    if (local >= 9.55 && local < 11.85) {
      transfer(357, 443, 403, 'Signature', c.status, smooth((local - 9.55) / 2.3));
    }
    if (local >= 15.05 && local < 18.05) {
      transfer(838, 980, 404, 'SSH', c.status, smooth((local - 15.05) / 3));
    }
  }
  g.restore();
  const [start, detail] = captionOverride ? [0, captionOverride] :
    captions.findLast(([time]) => t >= time);
  float(start === 0 ? 1 : rise(t, start, .28), () => {
    captionText(detail);
  }, 5);
}
const u8 = (...values) => new Uint8Array(values);
const num = (bytes, value) => {
  const buffer = new Uint8Array(bytes);
  const view = new DataView(buffer.buffer);
  if (bytes === 2) view.setUint16(0, value);
  else view.setUint32(0, value);
  return buffer;
};
const u16 = value => num(2, value);
const u32 = value => num(4, value);
const ascii = value => new TextEncoder().encode(value);
const blank = length => new Uint8Array(length);
const join = (...chunks) => {
  const result = blank(chunks.reduce((length, chunk) => length + chunk.byteLength, 0));
  let offset = 0;
  for (const chunk of chunks) { result.set(chunk, offset); offset += chunk.byteLength; }
  return result;
};
const box = (type, ...chunks) => join(u32(8 + chunks.reduce((size, chunk) => size + chunk.byteLength, 0)), ascii(type), ...chunks);
const full = (type, flags, ...chunks) => box(type, u32(flags), ...chunks);
const matrix = join(u32(65536), blank(12), u32(65536), blank(12), u32(0x40000000));

function mp4(frames, config, fps) {
  const w = canvas.width;
  const h = canvas.height;
  const clock = 24000;
  const sampleDuration = clock / fps;
  const duration = frames.length * sampleDuration;
  const ftyp = box('ftyp', ascii('isom'), u32(512), ascii('isomiso2avc1mp41'));
  const mvhd = full('mvhd', 0, blank(8), u32(clock), u32(duration), u32(65536), u16(256), blank(10), matrix, blank(24), u32(2));
  const tkhd = full('tkhd', 7, blank(8), u32(1), blank(4), u32(duration), blank(16), matrix, u32(w * 65536), u32(h * 65536));
  const mdhd = full('mdhd', 0, blank(8), u32(clock), u32(duration), u16(0x55c4), u16(0));
  const hdlr = full('hdlr', 0, blank(4), ascii('vide'), blank(12), ascii('VideoHandler'), u8(0));
  const vmhd = full('vmhd', 1, blank(8));
  const dinf = box('dinf', full('dref', 0, u32(1), full('url ', 1)));
  const avc = box('avc1', blank(6), u16(1), blank(16), u16(w), u16(h), u32(0x480000), u32(0x480000), blank(4), u16(1), blank(32), u16(24), u16(65535), box('avcC', config));
  const stsd = full('stsd', 0, u32(1), avc);
  const stts = full('stts', 0, u32(1), u32(frames.length), u32(sampleDuration));
  const stsc = full('stsc', 0, u32(1), u32(1), u32(frames.length), u32(1));
  const stsz = full('stsz', 0, u32(0), u32(frames.length), ...frames.map(frame => u32(frame.data.length)));
  const keys = frames.flatMap((frame, i) => frame.key ? [u32(i + 1)] : []);
  const stss = full('stss', 0, u32(keys.length), ...keys);
  const movie = offset => {
    const stco = full('stco', 0, u32(1), u32(offset));
    const stbl = box('stbl', stsd, stts, stsc, stsz, stco, stss);
    return box('moov', mvhd, box('trak', tkhd, box('mdia', mdhd, hdlr, box('minf', vmhd, dinf, stbl))));
  };
  return new Blob([ftyp, movie(ftyp.length + movie(0).length + 8), box('mdat', ...frames.map(frame => frame.data))], { type: 'video/mp4' });
}

async function render() {
  await Promise.all([
    appIcon.decode(), personNormalImage.decode(), personWorriedImage.decode(), attackerImage.decode(),
  ]);
  const homepage = new URLSearchParams(location.search).get('cut') === 'homepage';
  const seconds = homepage ? homepageSeconds : fullCutSeconds;
  const drawFrame = homepage ? drawHomepage : time => draw(time * fullCutSpeed);
  const fileName = homepage ? 'keyringd-homepage' : 'keyringd-in-60-seconds';
  if (!homepage) titleCard = createTitleCard();
  const fps = 24;
  const width = canvas.width;
  const height = canvas.height;
  const frames = [];
  let description;
  let failure;
  const options = {
    codec: 'avc1.420028', width, height, bitrate: 2000000, framerate: fps,
    latencyMode: 'realtime', hardwareAcceleration: 'prefer-software', avc: { format: 'avc' },
  };
  const supported = await VideoEncoder.isConfigSupported(options);
  if (!supported.supported) throw new Error('This browser cannot encode H.264.');
  const encoder = new VideoEncoder({
    output(chunk, metadata) {
      if (metadata.decoderConfig?.description) description = new Uint8Array(metadata.decoderConfig.description);
      const data = blank(chunk.byteLength);
      chunk.copyTo(data);
      frames.push({ data, key: chunk.type === 'key', time: chunk.timestamp });
    },
    error(error) { failure = error; },
  });
  encoder.configure(options);
  for (let i = 0; i < fps * seconds; i++) {
    drawFrame(i / fps);
    const frame = new VideoFrame(canvas, { timestamp: Math.round(i * 1e6 / fps), duration: Math.round(1e6 / fps) });
    encoder.encode(frame, { keyFrame: i % (2 * fps) === 0 });
    frame.close();
    if (encoder.encodeQueueSize > 12) await new Promise(resolve => encoder.addEventListener('dequeue', resolve, { once: true }));
    if (failure) throw failure;
  }
  await encoder.flush();
  encoder.close();
  if (frames.length !== fps * seconds || !description || frames.some((frame, i) => frame.time !== Math.round(i * 1e6 / fps))) {
    throw new Error('Video encoder returned incomplete or reordered frames.');
  }
  const video = mp4(frames, description, fps);
  const save = async (name, body) => {
    const response = await fetch('/save/' + name, { method: 'POST', body });
    if (!response.ok) throw new Error('Could not save ' + name);
  };
  await save(fileName + '.mp4', video);
  drawFrame(0);
  await save(fileName + '-poster.png', await new Promise(resolve => canvas.toBlob(resolve, 'image/png')));

  const player = document.createElement('video');
  const videoEvent = name => new Promise((resolve, reject) => {
    player.addEventListener(name, resolve, { once: true });
    player.addEventListener('error', () => reject(new Error(player.error.message)), { once: true });
  });
  const loaded = videoEvent('loadeddata');
  player.src = URL.createObjectURL(video);
  await loaded;
  const contact = document.createElement('canvas');
  const previewWidth = width / 2;
  const previewHeight = height / 2;
  contact.width = previewWidth * 2;
  const reviewTimes = homepage ?
    [0.05, 1.85, 3.15, 5.15, 9.25, 12.6, 15.5, 17.2, 19.2, 22.45, 25.9, 28.9] :
    [...[.1, 1.74, 2.05, 3.4, 5.9, 7.8, 10.6, 17.1, 21.4, 25.3, 29.8, 33.1, 35.6, 39.2]
      .map(time => time / fullCutSpeed), 54, seconds - 1 / fps];
  contact.height = previewHeight * Math.ceil(reviewTimes.length / 2);
  const page = contact.getContext('2d');
  for (const [index, seconds] of reviewTimes.entries()) {
    const seeked = videoEvent('seeked');
    player.currentTime = seconds;
    await seeked;
    page.drawImage(player, (index % 2) * previewWidth, Math.floor(index / 2) * previewHeight, previewWidth, previewHeight);
  }
  await save('contact-sheet.png', await new Promise(resolve => contact.toBlob(resolve, 'image/png')));
  await save('verification.json', JSON.stringify({ duration: player.duration, width: player.videoWidth, height: player.videoHeight, frames: frames.length }));
}

render().catch(error => fetch('/error', { method: 'POST', body: error.stack || String(error) }));
