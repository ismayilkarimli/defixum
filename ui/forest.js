// Procedural conifer tree lines and the sky for the hour, shared by the pause screen and the
// settings window.

window.Forest = (() => {
  // Small seeded generator (mulberry32), so a forest stays the same for a whole day.
  function rng(seed) {
    let a = seed >>> 0;
    return () => {
      a = (a + 0x6d2b79f5) >>> 0;
      let t = a;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  const daySeed = (salt = 0) => {
    const d = new Date();
    return (d.getFullYear() * 400 + d.getMonth() * 32 + d.getDate()) * 97 + salt;
  };

  // One spruce: drooping tiers that narrow to a spire. Tiers alternate an outer branch tip with
  // an inner notch, so the silhouette reads as a conifer rather than a triangle.
  function spruce(x, base, w, h, r) {
    const tiers = 5 + Math.floor(r() * 4);
    const lean = (r() - 0.5) * w * 0.08;
    const left = [];
    for (let i = 0; i < tiers; i++) {
      const t = i / tiers;
      const y = base - h * t * 0.94;
      const half = (w / 2) * Math.pow(1 - t, 0.85) * (0.9 + r() * 0.2);
      const notchY = y - (h * 0.94) / tiers * (0.55 + r() * 0.2);
      left.push([x - half + lean * t, y], [x - half * 0.32 + lean * t, notchY]);
    }
    const apex = [x + lean, base - h];
    const right = left.map(([px, py]) => [2 * (x + lean * ((base - py) / h)) - px, py]).reverse();
    const pts = [[x - w * 0.05, base + 2], ...left, apex, ...right, [x + w * 0.05, base + 2]];
    return "M" + pts.map(([px, py]) => `${px.toFixed(1)} ${py.toFixed(1)}`).join("L") + "Z";
  }

  const smooth = (t) => t * t * (3 - 2 * t);

  /**
   * A band of overlapping spruces standing on `base`, filled solid down to `height`.
   * `clearing` lowers the trees around a centre point so text there stays readable.
   */
  function treeline({ width, height, base, minH, maxH, minW, maxW, seed, clearing, hills = 0, scrub = 0, spacing = [0.32, 0.72] }) {
    const r = rng(seed);
    // the ground rolls: two slow waves, so far ridges read as hills rather than a ruler line
    const f1 = (2 + r() * 2) / width, f2 = (5 + r() * 4) / width, p1 = r() * 7, p2 = r() * 7;
    const ground = (x) => base - hills * (0.6 * Math.sin(x * f1 * Math.PI + p1) + 0.4 * Math.sin(x * f2 * Math.PI + p2));
    let d = `M0 ${height}`;
    if (scrub) {
      // undergrowth: low rounded tufts along the ground
      for (let x = 0; x <= width; ) {
        const w = 6 + r() * 14, top = ground(x + w / 2) - scrub * (0.25 + r() * 0.75);
        d += `L${x.toFixed(1)} ${ground(x).toFixed(1)}Q${(x + w / 2).toFixed(1)} ${(2 * top - ground(x)).toFixed(1)} ${(x + w).toFixed(1)} ${ground(x + w).toFixed(1)}`;
        x += w;
      }
    } else {
      for (let x = 0; x <= width; x += width / 48) d += `L${x.toFixed(1)} ${ground(x).toFixed(1)}`;
    }
    d += `L${width} ${ground(width).toFixed(1)}L${width} ${height}Z`;
    let x = -maxW * r();
    while (x < width + maxW) {
      const w = minW + (maxW - minW) * r();
      let h = minH + (maxH - minH) * Math.pow(r(), 1.6);
      if (clearing) {
        const dist = Math.min(Math.abs(x - clearing.center) / clearing.half, 1);
        h *= clearing.floor + (1 - clearing.floor) * smooth(dist);
      }
      if (h > 1) d += spruce(x, ground(x) + r() * minH * 0.15, w, h, r);
      x += w * (spacing[0] + r() * (spacing[1] - spacing[0]));
    }
    return d;
  }

  // ---------- colour ----------

  const hex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
  const toHex = (c) => "#" + c.map((v) => Math.round(v).toString(16).padStart(2, "0")).join("");
  const mix = (a, b, t) => toHex(hex(a).map((v, i) => v + (hex(b)[i] - v) * t));
  function luminance(h) {
    const [r, g, b] = hex(h).map((v) => {
      v /= 255;
      return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  }

  // The sky over the trees through the day: [hour, top, horizon]. Interpolated, so the light
  // drifts with the clock instead of switching between themes.
  const SKY = [
    [0, "#07131a", "#22403c"],
    [4.5, "#0b1b22", "#2a4542"],
    [6.5, "#8aa3a9", "#d6dccf"],
    [9.5, "#a5c0bf", "#dfe8de"],
    [16, "#a0bcb9", "#dce5d8"],
    [17.4, "#a9b7a4", "#e3d6a6"],
    [18.5, "#c4ad80", "#e8c48a"],
    [20.25, "#20364a", "#5c6e69"],
    [22, "#07131a", "#22403c"],
    [24, "#07131a", "#22403c"],
  ];

  // A grey of the same brightness, for skies under cloud.
  const grey = (h) => {
    const [r, g, b] = hex(h), v = 0.3 * r + 0.55 * g + 0.15 * b;
    return toHex([v * 0.98, v, v * 1.01].map((c) => Math.min(c, 255)));
  };

  function sky(hour, wx = CALM) {
    let i = 0;
    while (i < SKY.length - 2 && hour >= SKY[i + 1][0]) i++;
    const [h0, t0, b0] = SKY[i], [h1, t1, b1] = SKY[i + 1];
    const t = Math.min(Math.max((hour - h0) / (h1 - h0), 0), 1);
    let top = mix(t0, t1, t), horizon = mix(b0, b1, t);
    // weather: cloud drains the colour and dims the light; fog flattens the sky into one pale wash
    if (wx.gloom) {
      // storm light is dark and heavy; a snowy sky goes pale instead
      const dim = wx.snow ? "#9aa3a3" : "#26302e", lift = wx.gloom * 0.32 + (wx.storm ? 0.3 : 0);
      top = mix(mix(top, grey(top), wx.gloom), dim, lift);
      horizon = mix(mix(horizon, grey(horizon), wx.gloom), wx.snow ? "#b4bcbb" : "#3a4442", lift * 0.6);
    }
    if (wx.fog) top = mix(top, horizon, wx.fog * 0.65);
    const light = luminance(mix(top, horizon, 0.5));
    // nearest trees: near-black spruce, a touch greener in daylight
    const near = mix("#020806", "#0a1c15", Math.min(light / 0.4, 1));
    // Words sit about 60% of the way down the sky. Dark ink and light ink read equally well at a
    // luminance near 0.2, so that is where they swap; close to the swap (dawn, dusk) neither has
    // much contrast, so a haze behind the words thickens to push the sky away from the ink.
    const behind = luminance(mix(top, horizon, 0.6));
    const haze = 0.08 + 0.4 * Math.max(0, 1 - Math.abs(behind - 0.2) / 0.18);
    const dark = Math.min(Math.max((0.08 - light) / 0.05, 0), 1);
    // cloud hides the stars; a thin veil still lets most through
    return { top, horizon, near, day: behind > 0.2, haze, stars: dark * (1 - Math.pow(wx.cover, 1.6)) };
  }

  // ---------- weather ----------

  const CALM = { kind: "clear", cover: 0, gloom: 0, fog: 0, rain: 0, snow: 0, storm: false, wind: 0 };

  // Rain, snow and fog strengths (0–1) for each WMO weather code Open-Meteo reports.
  const CODES = {
    45: { kind: "fog", fog: 1 }, 48: { kind: "fog", fog: 1 },
    51: { kind: "drizzle", rain: 0.2 }, 53: { kind: "drizzle", rain: 0.3 }, 55: { kind: "drizzle", rain: 0.4 },
    56: { kind: "drizzle", rain: 0.3 }, 57: { kind: "drizzle", rain: 0.45 },
    61: { kind: "rain", rain: 0.45 }, 63: { kind: "rain", rain: 0.7 }, 65: { kind: "rain", rain: 1 },
    66: { kind: "rain", rain: 0.5 }, 67: { kind: "rain", rain: 0.9 },
    80: { kind: "rain", rain: 0.5 }, 81: { kind: "rain", rain: 0.75 }, 82: { kind: "rain", rain: 1 },
    71: { kind: "snow", snow: 0.35 }, 73: { kind: "snow", snow: 0.65 }, 75: { kind: "snow", snow: 1 },
    77: { kind: "snow", snow: 0.25 }, 85: { kind: "snow", snow: 0.6 }, 86: { kind: "snow", snow: 1 },
    95: { kind: "storm", rain: 0.85, storm: true }, 96: { kind: "storm", rain: 1, storm: true }, 99: { kind: "storm", rain: 1, storm: true },
  };

  /** What the sky should do for a reading `{code, cloud, wind}` (null means fair weather). */
  function weather(w) {
    if (!w) return CALM;
    const c = { ...CALM, ...(CODES[w.code] || {}) };
    if (!CODES[w.code]) c.kind = w.code >= 3 ? "overcast" : w.code === 2 ? "cloudy" : "clear";
    // falling weather comes from a full sky, whatever the cloud figure says
    const wet = Math.max(c.rain, c.snow);
    c.cover = Math.max(Math.min((w.cloud ?? 0) / 100, 1), c.kind === "overcast" ? 0.9 : 0, wet ? 0.9 : 0, c.fog);
    c.gloom = Math.min(Math.max((c.cover - 0.55) / 0.45, 0) * 0.5 + c.rain * 0.25 + c.snow * 0.1 + (c.storm ? 0.2 : 0) + c.fog * 0.3, 0.9);
    c.wind = Math.min((w.wind ?? 0) / 45, 1);
    return c;
  }

  // Where the clock hour falls on the sky's own day, whose sun rises at 6.6 and sets at 18.6, so
  // the real sunrise and sunset (unix seconds) land on dawn and dusk whatever the season.
  function solarHour(hour, w) {
    if (!w || !w.sunrise || !w.sunset) return hour;
    const at = (t) => { const d = new Date(t * 1000); return d.getHours() + d.getMinutes() / 60; };
    const rise = at(w.sunrise), set = at(w.sunset), day = set - rise;
    if (!(day > 1 && day < 23)) return hour;
    if (hour >= rise && hour <= set) return 6.6 + ((hour - rise) / day) * 12;
    return (18.6 + (((hour - set + 24) % 24) / (24 - day)) * 12) % 24;
  }

  // The hour on this Mac's clock, in its time zone (`?hour=` overrides it for design work).
  const nowHour = () => {
    const forced = new URLSearchParams(location.search).get("hour");
    if (forced !== null && forced !== "") return +forced;
    const d = new Date();
    return d.getHours() + d.getMinutes() / 60;
  };

  // How strongly golden-hour light is in the air: 1 at sunset or sunrise, 0 away from them.
  const golden = (hour) => Math.max(0, 1 - Math.abs(hour - 18.6) / 1.3, 1 - Math.abs(hour - 6.6) / 1.1);

  return { rng, daySeed, treeline, sky, mix, nowHour, golden, weather, solarHour, CALM };
})();
