// Defixum settings window. Holds the config in memory, saves it shortly after each edit.

const tauri = window.__TAURI__ || mockTauri();
const { invoke } = tauri.core;
const { listen } = tauri.event;
const $ = (id) => document.getElementById(id);
const DAY_NAMES = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

let state = null;      // last get_state result
let config = null;     // working copy
let selected = null;   // rule id
let saveTimer = null;

// ---------- data ----------

async function load() {
  state = await invoke("get_state");
  if (!config) config = structuredClone(state.config);
  if (!selected || !config.rules.some((r) => r.id === selected)) selected = config.rules[0]?.id ?? null;
  renderAll();
}

function scheduleSave() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(save, 450);
}

// The local `config` stays the source of truth: the editor holds references into it, so it
// must never be swapped for the copy the backend sends back.
async function save() {
  try {
    await invoke("save_config", { config });
    flash("Saved");
    renderList();
    renderToday();
  } catch (e) {
    flash(`Couldn't save: ${e}`);
  }
}

function flash(text) {
  const el = $("saved");
  el.textContent = text;
  el.style.opacity = 1;
  clearTimeout(flash.t);
  flash.t = setTimeout(() => (el.style.opacity = 0), 1600);
}

const rule = () => config.rules.find((r) => r.id === selected);
const todayStats = (id) => (state.stats[state.today] || {})[id] || { intercepts: 0, opens: 0, skips: 0, seconds: 0 };

// ---------- tabs ----------

document.querySelectorAll(".tabs button").forEach((b) =>
  b.addEventListener("click", () => {
    document.querySelectorAll(".tabs button").forEach((x) => x.setAttribute("aria-selected", x === b));
    document.querySelectorAll(".tab").forEach((t) => (t.hidden = t.id !== `tab-${b.dataset.tab}`));
    if (b.dataset.tab !== "rules") load();
  })
);

function renderAll() {
  renderList();
  renderEditor();
  renderToday();
  renderActivity();
  renderSetup();
}

// ---------- rule list ----------

function ruleSummary(r) {
  const what = r.mode === "block" ? "Blocks" : `${r.pause_seconds}s pause`;
  const sites = r.sites.length ? r.sites.slice(0, 2).join(", ") + (r.sites.length > 2 ? ` +${r.sites.length - 2}` : "") : "No sites yet";
  return `${what}, ${sites}`;
}

function renderList() {
  const ul = $("rule-list");
  ul.replaceChildren(
    ...config.rules.map((r) => {
      const li = document.createElement("li");
      const b = document.createElement("button");
      b.className = r.enabled ? "" : "off";
      b.setAttribute("aria-current", r.id === selected);
      b.innerHTML = `<span class="state"></span><span class="rname"></span><span class="rsub"></span>`;
      b.querySelector(".rname").textContent = r.name || "Untitled rule";
      b.querySelector(".rsub").textContent = ruleSummary(r);
      b.addEventListener("click", () => { selected = r.id; renderList(); renderEditor(); });
      li.appendChild(b);
      return li;
    })
  );
}

$("add-rule").addEventListener("click", async () => {
  const r = {
    id: `r${Date.now()}`, name: "New rule", enabled: true, sites: [], mode: "pause", pause_seconds: 10,
    daily_opens: null, daily_minutes: null,
    schedule: { enabled: false, days: [true, true, true, true, true, false, false], start: "09:00", end: "17:00" },
  };
  config.rules.push(r);
  selected = r.id;
  clearTimeout(saveTimer);
  await save();
  renderList();
  renderEditor();
  const name = $("f-name");
  name.focus();
  name.select();
});

function renderToday() {
  let pauses = 0, back = 0;
  for (const r of config.rules) { const s = todayStats(r.id); pauses += s.intercepts; back += s.skips; }
  $("today-line").textContent =
    pauses === 0 ? "No pauses yet today." : `Today you paused ${times(pauses)} and turned back ${times(back)}.`;
}

const times = (n) => (n === 1 ? "once" : n === 2 ? "twice" : `${n} times`);

// ---------- editor ----------

function renderEditor() {
  const host = $("editor");
  const r = rule();
  if (!r) {
    host.innerHTML = `<p class="editor-empty">Add a rule to choose which sites make you pause.</p>`;
    return;
  }
  host.replaceChildren($("tpl-editor").content.cloneNode(true));

  const name = $("f-name");
  name.value = r.name;
  name.addEventListener("input", () => { r.name = name.value; renderList(); scheduleSave(); });

  const enabled = $("f-enabled");
  enabled.checked = r.enabled;
  $("f-enabled-text").textContent = r.enabled ? "On" : "Off";
  enabled.addEventListener("change", () => {
    r.enabled = enabled.checked;
    $("f-enabled-text").textContent = r.enabled ? "On" : "Off";
    renderList();
    scheduleSave();
  });

  // sites
  const chips = $("f-sites"), input = $("f-site-input");
  const drawChips = () => {
    chips.querySelectorAll(".chip").forEach((c) => c.remove());
    r.sites.forEach((s, i) => {
      const c = document.createElement("span");
      c.className = "chip";
      c.textContent = s;
      const x = document.createElement("button");
      x.textContent = "×";
      x.setAttribute("aria-label", `Remove ${s}`);
      x.addEventListener("click", (e) => { e.stopPropagation(); r.sites.splice(i, 1); drawChips(); renderList(); scheduleSave(); });
      c.appendChild(x);
      chips.insertBefore(c, input);
    });
  };
  const addSites = (text) => {
    const added = text.split(/[\s,]+/).map(cleanSite).filter((s) => s && !r.sites.includes(s));
    if (!added.length) return;
    r.sites.push(...added);
    drawChips();
    renderList();
    scheduleSave();
  };
  drawChips();
  chips.addEventListener("click", () => input.focus());
  input.addEventListener("keydown", (e) => {
    if ((e.key === "Enter" || e.key === "," || e.key === " ") && input.value.trim()) {
      e.preventDefault();
      addSites(input.value);
      input.value = "";
    } else if (e.key === "Backspace" && !input.value && r.sites.length) {
      r.sites.pop();
      drawChips();
      renderList();
      scheduleSave();
    }
  });
  input.addEventListener("blur", () => { if (input.value.trim()) { addSites(input.value); input.value = ""; } });
  input.addEventListener("paste", (e) => {
    e.preventDefault();
    addSites(e.clipboardData.getData("text"));
  });

  // mode + pause
  document.querySelectorAll('input[name="mode"]').forEach((radio) => {
    radio.checked = radio.value === r.mode;
    radio.addEventListener("change", () => { r.mode = radio.value; syncMode(); renderList(); scheduleSave(); });
  });
  const pause = $("f-pause"), out = $("f-pause-out");
  pause.value = r.pause_seconds;
  const drawPause = () => (out.textContent = `${pause.value} seconds`);
  drawPause();
  pause.addEventListener("input", () => { r.pause_seconds = +pause.value; drawPause(); renderList(); scheduleSave(); });
  const syncMode = () => {
    const block = r.mode === "block";
    $("pause-len").hidden = block;
    $("limits").hidden = block;
  };
  syncMode();

  // limits
  bindLimit("f-opens-on", "f-opens", "daily_opens", 5);
  bindLimit("f-mins-on", "f-mins", "daily_minutes", 30);

  // schedule
  const sOn = $("f-sched-on"), sched = $("sched");
  sOn.checked = r.schedule.enabled;
  const drawSched = () => sched.classList.toggle("off", !r.schedule.enabled);
  drawSched();
  sOn.addEventListener("change", () => { r.schedule.enabled = sOn.checked; drawSched(); scheduleSave(); });
  const days = $("f-days");
  DAY_NAMES.forEach((d, i) => {
    const b = document.createElement("button");
    b.textContent = d;
    b.setAttribute("aria-pressed", r.schedule.days[i]);
    b.addEventListener("click", () => {
      r.schedule.days[i] = !r.schedule.days[i];
      b.setAttribute("aria-pressed", r.schedule.days[i]);
      scheduleSave();
    });
    days.appendChild(b);
  });
  for (const [id, key] of [["f-start", "start"], ["f-end", "end"]]) {
    const el = $(id);
    el.value = r.schedule[key];
    el.addEventListener("change", () => { if (el.value) { r.schedule[key] = el.value; scheduleSave(); } });
  }

  // Two-step delete: window.confirm() is a silent no-op in the app's web view.
  const del = $("f-delete");
  del.addEventListener("click", () => {
    if (!del.classList.contains("armed")) {
      del.classList.add("armed");
      del.textContent = `Click again to delete “${r.name || "this rule"}”`;
      clearTimeout(del.t);
      del.t = setTimeout(() => { del.classList.remove("armed"); del.textContent = "Delete rule"; }, 4000);
      return;
    }
    clearTimeout(del.t);
    config.rules = config.rules.filter((x) => x !== r);
    selected = config.rules[0]?.id ?? null;
    renderList();
    renderEditor();
    scheduleSave();
  });

  function bindLimit(checkId, numId, key, fallback) {
    const check = $(checkId), num = $(numId);
    check.checked = r[key] != null;
    num.value = r[key] ?? fallback;
    num.disabled = !check.checked;
    check.addEventListener("change", () => {
      num.disabled = !check.checked;
      r[key] = check.checked ? Math.max(1, +num.value || fallback) : null;
      scheduleSave();
    });
    num.addEventListener("input", () => {
      if (check.checked && +num.value > 0) { r[key] = Math.round(+num.value); scheduleSave(); }
    });
  }
}

function cleanSite(s) {
  s = s.trim().toLowerCase().replace(/^[a-z]+:\/\//, "").replace(/^www\./, "").replace(/\/+$/, "");
  return /^[a-z0-9-]+(\.[a-z0-9-]+)+(\/.*)?$/.test(s) ? s : "";
}

// ---------- activity ----------

function renderActivity() {
  const days = [];
  const base = new Date(state.today + "T12:00:00");
  for (let i = 6; i >= 0; i--) {
    const d = new Date(base);
    d.setDate(base.getDate() - i);
    const key = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    const rules = state.stats[key] || {};
    const sum = { intercepts: 0, opens: 0, skips: 0, seconds: 0 };
    for (const s of Object.values(rules)) for (const k in sum) sum[k] += s[k];
    days.push({ key, label: i === 0 ? "Today" : DAY_NAMES[(d.getDay() + 6) % 7], ...sum, today: i === 0 });
  }
  const t = days[6];
  const week = days.reduce((a, d) => ({ i: a.i + d.intercepts, s: a.s + d.skips, sec: a.sec + d.seconds }), { i: 0, s: 0, sec: 0 });
  $("activity-headline").textContent =
    t.intercepts === 0 ? "Nothing has needed a pause today." : `You paused ${times(t.intercepts)} today and turned back ${times(t.skips)}.`;
  $("activity-lede").textContent =
    week.i === 0
      ? "Once a rule catches a site, each pause shows up here."
      : `Over the last 7 days: ${week.i} pauses, ${Math.round((week.s / week.i) * 100)}% of them ended with the tab closed, and ${fmtMinutes(week.sec)} spent on paused sites.`;

  const max = Math.max(1, ...days.map((d) => d.skips + d.opens));
  $("week-chart").replaceChildren(
    ...days.map((d) => {
      const col = document.createElement("div");
      col.className = "col" + (d.today ? " is-today" : "");
      const total = d.skips + d.opens;
      col.innerHTML = `<span class="n">${total || ""}</span><div class="stack" style="height:${(total / max) * 140 + 2}px"><div class="back" style="flex:${d.skips}"></div><div class="open" style="flex:${d.opens}"></div></div><span class="d">${d.label}</span>`;
      col.title = `${d.key}: ${d.skips} turned back, ${d.opens} opened, ${fmtMinutes(d.seconds)}`;
      return col;
    })
  );

  const tbody = $("rule-table").querySelector("tbody");
  if (!config.rules.length) {
    tbody.innerHTML = `<tr><td colspan="5" class="empty">No rules yet.</td></tr>`;
    return;
  }
  tbody.replaceChildren(
    ...config.rules.map((r) => {
      const s = todayStats(r.id);
      const tr = document.createElement("tr");
      const mins = Math.floor(s.seconds / 60);
      const over = r.daily_minutes && mins >= r.daily_minutes;
      const opensOver = r.daily_opens && s.opens >= r.daily_opens;
      tr.innerHTML = `<td></td><td>${s.intercepts}</td><td>${s.skips}</td><td class="${opensOver ? "over" : ""}">${s.opens}${r.daily_opens ? ` of ${r.daily_opens}` : ""}</td><td class="${over ? "over" : ""}">${fmtMinutes(s.seconds)}${r.daily_minutes ? ` of ${r.daily_minutes} min` : ""}</td>`;
      tr.firstChild.textContent = r.name || "Untitled rule";
      return tr;
    })
  );
}

function fmtMinutes(sec) {
  const m = Math.round(sec / 60);
  if (m < 60) return `${m} min`;
  return `${Math.floor(m / 60)} h ${m % 60} min`;
}

// ---------- setup ----------

function renderSetup() {
  const needsAx = state.browsers.some((b) => b.kind === "ax");
  const ax = $("ax-row");
  ax.className = "row" + (needsAx && !state.accessibility ? " warn" : "");
  ax.innerHTML = state.accessibility
    ? `<span><strong>Accessibility is allowed</strong><small>Used to read the address in Zen, Firefox and similar browsers</small></span><span class="status ok">Allowed</span>`
    : `<span><strong>Allow Accessibility</strong><small>Needed for Zen, Firefox and similar browsers. Turn on Defixum in the list that opens</small></span><button class="btn" id="ax-open">Open System Settings</button>`;
  $("ax-open")?.addEventListener("click", () => invoke("open_privacy", { pane: "accessibility" }));

  const ul = $("browsers");
  if (!state.browsers.length) {
    ul.innerHTML = `<li><span class="bname">No supported browsers found</span></li>`;
  } else {
    ul.replaceChildren(...state.browsers.map(browserRow));
  }

  let problem = needsAx && !state.accessibility;
  for (const b of state.browsers) if (b.status === "denied") problem = true;
  $("setup-dot").hidden = !problem;

  const so = $("session-options");
  if (document.activeElement !== so) so.value = config.settings.session_options.join(", ");
  const idle = $("idle-seconds");
  if (document.activeElement !== idle) idle.value = config.settings.idle_seconds;
  $("autostart").checked = state.autostart;
}

function browserRow(b) {
  const li = document.createElement("li");
  const how = b.kind === "ax" ? "through Accessibility" : "through Apple Events";
  li.innerHTML = `<span class="bname"></span><span class="status"></span>`;
  li.querySelector(".bname").textContent = b.name;
  const small = document.createElement("small");
  small.textContent = how;
  li.querySelector(".bname").appendChild(small);
  const st = li.querySelector(".status");
  const ok = b.kind === "ax" ? state.accessibility : b.status === "ok";
  if (ok) {
    st.textContent = "Working";
    st.classList.add("ok");
  } else if (b.status === "denied") {
    st.textContent = "Not allowed";
    st.classList.add("bad");
    li.appendChild(button("Open System Settings", () => invoke("open_privacy", { pane: b.kind === "ax" ? "accessibility" : "automation" })));
  } else if (b.kind !== "ax") {
    st.textContent = b.running ? "Not checked yet" : `Open ${b.name} to check`;
    if (b.running) li.appendChild(button("Check", () => check(b)));
  } else {
    st.textContent = "Needs Accessibility";
    st.classList.add("bad");
  }
  return li;
}

async function check(b) {
  const r = await invoke("test_browser", { bundle: b.bundle });
  if (r === "not_running") flash(`Open ${b.name} first`);
  await load();
}

function button(text, fn) {
  const b = document.createElement("button");
  b.className = "btn";
  b.textContent = text;
  b.addEventListener("click", fn);
  return b;
}

$("session-options").addEventListener("change", (e) => {
  const list = e.target.value.split(/[\s,]+/).map(Number).filter((n) => Number.isInteger(n) && n >= 1 && n <= 240);
  if (list.length) config.settings.session_options = [...new Set(list)].sort((a, b) => a - b).slice(0, 4);
  e.target.value = config.settings.session_options.join(", ");
  scheduleSave();
});
$("idle-seconds").addEventListener("change", (e) => {
  const v = Math.min(900, Math.max(10, Math.round(+e.target.value || 60)));
  config.settings.idle_seconds = v;
  e.target.value = v;
  scheduleSave();
});
$("autostart").addEventListener("change", async (e) => {
  try {
    e.target.checked = await invoke("set_autostart", { enabled: e.target.checked });
  } catch (err) {
    flash(`Couldn't change: ${err}`);
    e.target.checked = !e.target.checked;
  }
});

// ---------- boot ----------

listen("refresh", () => load());
window.addEventListener("focus", () => { if (state) invoke("get_state").then((s) => { state = s; renderToday(); renderActivity(); renderSetup(); }); });
load();

// Lets the page render in a plain browser for design work (no Tauri).
function mockTauri() {
  const today = new Date();
  const key = (d) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  const stats = {};
  for (let i = 0; i < 7; i++) {
    const d = new Date(today);
    d.setDate(today.getDate() - i);
    stats[key(d)] = { starter: { intercepts: 3 + ((i * 5) % 7), opens: 1 + (i % 3), skips: 2 + ((i * 3) % 4), seconds: 600 + i * 420 } };
  }
  let cfg = {
    rules: [
      { id: "starter", name: "Social & video", enabled: true, sites: ["youtube.com", "reddit.com", "x.com", "instagram.com"], mode: "pause", pause_seconds: 10, daily_opens: 5, daily_minutes: 30, schedule: { enabled: false, days: [true, true, true, true, true, false, false], start: "09:00", end: "17:00" } },
      { id: "work", name: "Work hours", enabled: true, sites: ["news.ycombinator.com"], mode: "block", pause_seconds: 10, daily_opens: null, daily_minutes: null, schedule: { enabled: true, days: [true, true, true, true, true, false, false], start: "09:00", end: "17:00" } },
    ],
    settings: { session_options: [2, 5, 15], idle_seconds: 60 },
  };
  return {
    core: {
      invoke: async (cmd, args) => {
        if (cmd === "get_state")
          return { config: structuredClone(cfg), today: key(today), stats, accessibility: true, autostart: false, active_allowances: {},
            browsers: [
              { bundle: "app.zen-browser.zen", name: "Zen", kind: "ax", running: true, status: "ok" },
              { bundle: "com.google.Chrome", name: "Google Chrome", kind: "chromium", running: false, status: null },
              { bundle: "company.thebrowser.Browser", name: "Arc", kind: "chromium", running: true, status: "denied" },
              { bundle: "com.apple.Safari", name: "Safari", kind: "safari", running: true, status: null },
            ] };
        if (cmd === "save_config") { cfg = structuredClone(args.config); cfg.rules.forEach((r, i) => (r.id ||= "r" + Date.now() + i)); return structuredClone(cfg); }
        return null;
      },
    },
    event: { listen: async () => {} },
  };
}
