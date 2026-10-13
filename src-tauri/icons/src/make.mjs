// Draws the app icon and the menu-bar glyph as SVG: a low sun over spruce bands, the same
// spruce silhouette the pause screen draws (ui/forest.js). Run: node make.mjs <outdir>
import { writeFileSync } from "node:fs";

const out = process.argv[2] ?? ".";
function rng(seed) { return () => ((seed = (seed * 16807) % 2147483647) / 2147483647); }

function spruce(x, base, w, h, r, tiers = 5 + Math.floor(r() * 4)) {
  const left = [];
  for (let i = 0; i < tiers; i++) {
    const t = i / tiers;
    const y = base - h * t * 0.94;
    const half = (w / 2) * Math.pow(1 - t, 0.85) * (0.9 + r() * 0.2);
    const notchY = y - (h * 0.94) / tiers * (0.55 + r() * 0.2);
    left.push([x - half, y], [x - half * 0.32, notchY]);
  }
  const right = left.map(([px, py]) => [2 * x - px, py]).reverse();
  const pts = [[x - w * 0.05, base + 2], ...left, [x, base - h], ...right, [x + w * 0.05, base + 2]];
  return "M" + pts.map(([px, py]) => `${px.toFixed(1)} ${py.toFixed(1)}`).join("L") + "Z";
}

// a band of spruces, lower towards the middle so the sun sits in a clearing
function band({ seed, base, minH, maxH, w, step, clear, x0 = 60, x1 = 964 }) {
  const r = rng(seed);
  let d = "";
  for (let x = x0; x <= x1; x += step * (0.7 + r() * 0.6)) {
    const off = Math.min(1, Math.abs(x - 512) / 380);
    const h = (minH + r() * (maxH - minH)) * (1 - clear * (1 - off * off));
    d += spruce(x, base + r() * 8, w * (0.8 + r() * 0.4), h, r);
  }
  return d + `M0 ${base}H1024V1024H0Z`;
}

const icon = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<defs>
  <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#a99a72"/><stop offset=".45" stop-color="#cdb184"/><stop offset=".75" stop-color="#e6c78f"/>
  </linearGradient>
  <radialGradient id="glow" cx="512" cy="430" r="300" gradientUnits="userSpaceOnUse">
    <stop offset="0" stop-color="#fff6dc" stop-opacity=".85"/><stop offset=".38" stop-color="#fbe8bd" stop-opacity=".35"/><stop offset="1" stop-color="#f3d9a4" stop-opacity="0"/>
  </radialGradient>
  <radialGradient id="sun" cx=".42" cy=".38" r=".7">
    <stop offset="0" stop-color="#fffef4"/><stop offset=".7" stop-color="#f8f3df"/><stop offset="1" stop-color="#efe5c4"/>
  </radialGradient>
  <linearGradient id="mist" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color="#e9cf9a" stop-opacity="0"/><stop offset=".5" stop-color="#ead2a0" stop-opacity=".55"/><stop offset="1" stop-color="#e9cf9a" stop-opacity="0"/>
  </linearGradient>
  <clipPath id="sq"><rect x="100" y="100" width="824" height="824" rx="185"/></clipPath>
  <filter id="shadow" x="-10%" y="-10%" width="120%" height="125%"><feDropShadow dx="0" dy="10" stdDeviation="12" flood-color="#000" flood-opacity=".28"/></filter>
</defs>
<g filter="url(#shadow)"><rect x="100" y="100" width="824" height="824" rx="185" fill="#cdb184"/></g>
<g clip-path="url(#sq)">
  <rect width="1024" height="1024" fill="url(#sky)"/>
  <circle cx="512" cy="430" r="300" fill="url(#glow)"/>
  <circle cx="512" cy="430" r="118" fill="url(#sun)"/>
  <path d="${band({ seed: 7, base: 690, minH: 120, maxH: 200, w: 70, step: 34, clear: 0.35 })}" fill="#a39368" opacity=".75"/>
  <rect x="0" y="600" width="1024" height="150" fill="url(#mist)"/>
  <path d="${band({ seed: 19, base: 770, minH: 150, maxH: 240, w: 96, step: 52, clear: 0.45 })}" fill="#5f6447"/>
  <path d="${band({ seed: 3, base: 860, minH: 120, maxH: 200, w: 120, step: 78, clear: 0.6 })}" fill="#2a3426"/>
  <path d="${spruce(220, 960, 170, 640, rng(11), 11)}${spruce(812, 970, 150, 540, rng(23), 10)}M0 905C260 880 760 880 1024 900V1024H0Z" fill="#0f1c15"/>
</g>
<rect x="100.5" y="100.5" width="823" height="823" rx="184.5" fill="none" stroke="#fff" stroke-opacity=".12"/>
</svg>`;
writeFileSync(`${out}/icon.svg`, icon);

// menu-bar glyph: a sun with a spruce in front of it, cut out so the two read apart at 16pt
const tree = "M41 5L33 20H37.5L29 34H34L24 50H38.5V59H43.5V50H58L48 34H53L44.5 20H49Z";
const tray = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">
<defs><mask id="m"><rect width="64" height="64" fill="#fff"/><path d="${tree}" fill="#000" stroke="#000" stroke-width="6" stroke-linejoin="round"/></mask></defs>
<circle cx="22" cy="25" r="16" fill="#000" mask="url(#m)"/>
<path d="${tree}" fill="#000"/>
</svg>`;
writeFileSync(`${out}/tray.svg`, tray);
