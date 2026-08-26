const PREFIX = "com.flowingspdg.roland.v160hd.";

function addOption(select, value, label) {
  const opt = document.createElement("option");
  opt.value = value;
  opt.textContent = label;
  select.appendChild(opt);
}

function fillVideoSources(select) {
  for (let i = 1; i <= 8; i++) addOption(select, `hdmi:${i}`, `HDMI ${i}`);
  for (let i = 1; i <= 8; i++) addOption(select, `sdi:${i}`, `SDI ${i}`);
  for (let i = 1; i <= 16; i++) addOption(select, `still:${i}`, `Still ${i}`);
  for (let i = 1; i <= 20; i++) addOption(select, `input:${i}`, `Input ${i}`);
}

function fillSelects() {
  document.querySelectorAll("select.video-source").forEach((el) => {
    if (el.options.length === 0) fillVideoSources(el);
  });

  const sw = document.getElementById("switch");
  if (sw && sw.options.length === 0) {
    [
      ["pgm_a_1", "PGM/A 1"], ["pgm_a_2", "PGM/A 2"], ["pgm_a_3", "PGM/A 3"], ["pgm_a_4", "PGM/A 4"],
      ["pgm_a_5", "PGM/A 5"], ["pgm_a_6", "PGM/A 6"], ["pgm_a_7", "PGM/A 7"], ["pgm_a_8", "PGM/A 8"],
      ["pgm_a_9", "PGM/A 9"], ["pgm_a_10", "PGM/A 10"],
      ["pst_b_1", "PRV/B 1"], ["pst_b_2", "PRV/B 2"], ["pst_b_3", "PRV/B 3"], ["pst_b_4", "PRV/B 4"],
      ["pst_b_5", "PRV/B 5"], ["pst_b_6", "PRV/B 6"], ["pst_b_7", "PRV/B 7"], ["pst_b_8", "PRV/B 8"],
      ["pst_b_9", "PRV/B 9"], ["pst_b_10", "PRV/B 10"],
      ["aux_1", "AUX 1"], ["aux_2", "AUX 2"], ["aux_3", "AUX 3"], ["aux_4", "AUX 4"], ["aux_5", "AUX 5"],
      ["aux_6", "AUX 6"], ["aux_7", "AUX 7"], ["aux_8", "AUX 8"], ["aux_9", "AUX 9"], ["aux_10", "AUX 10"],
      ["cut", "CUT"], ["auto", "AUTO"], ["transition", "TRANSITION"], ["mode", "MODE"],
      ["input_assign", "INPUT ASSIGN"], ["pgm_center", "PGM encoder"], ["pst_center", "PRV encoder"],
      ["split_a", "SPLIT A"], ["split_b", "SPLIT B"], ["auto_mixing", "AUTO MIXING"], ["capture", "CAPTURE"],
      ["user_1", "USER 1"], ["user_2", "USER 2"], ["user_3", "USER 3"], ["user_4", "USER 4"],
      ["pinp1_pgm", "PinP1 PGM"], ["pinp1_pvw", "PinP1 PRV"], ["pinp2_pgm", "PinP2 PGM"], ["pinp2_pvw", "PinP2 PRV"],
      ["pinp3_pgm", "PinP3 PGM"], ["pinp3_pvw", "PinP3 PRV"], ["pinp4_pgm", "PinP4 PGM"], ["pinp4_pvw", "PinP4 PRV"],
      ["dsk1_pgm", "DSK1 PGM"], ["dsk1_pvw", "DSK1 PRV"], ["dsk2_pgm", "DSK2 PGM"], ["dsk2_pvw", "DSK2 PRV"],
      ["menu", "MENU"], ["exit", "EXIT"], ["enter", "ENTER"], ["output_fade", "OUTPUT FADE"],
      ["sequencer_on", "SEQUENCER ON"], ["sequencer_auto", "SEQUENCER AUTO"],
      ["sequencer_prev", "SEQUENCER PREV"], ["sequencer_next", "SEQUENCER NEXT"]
    ].forEach(([v, l]) => addOption(sw, v, l));
  }

  const ch = document.getElementById("channel");
  if (ch && ch.options.length === 0) {
    for (let i = 1; i <= 10; i++) addOption(ch, String(i), `Input ${i}`);
  }
  const assign = document.getElementById("input_assign");
  if (assign && assign.options.length === 0) {
    for (let i = 1; i <= 8; i++) addOption(assign, `hdmi${i}`, `HDMI ${i}`);
    for (let i = 1; i <= 8; i++) addOption(assign, `sdi${i}`, `SDI ${i}`);
    for (let i = 1; i <= 16; i++) addOption(assign, `still${i}`, `Still ${i}`);
  }
  const cam = document.getElementById("camera_id");
  if (cam && cam.options.length === 0) {
    for (let i = 1; i <= 16; i++) addOption(cam, String(i), `Camera ${i}`);
  }
  const slot = document.getElementById("slot");
  if (slot && slot.options.length === 0) {
    for (let i = 1; i <= 30; i++) addOption(slot, String(i), String(i));
  }
  const freezeIn = document.getElementById("freeze_input");
  if (freezeIn && freezeIn.options.length === 0) {
    for (let i = 1; i <= 8; i++) addOption(freezeIn, `hdmi:${i}`, `HDMI ${i}`);
    for (let i = 1; i <= 8; i++) addOption(freezeIn, `sdi:${i}`, `SDI ${i}`);
  }
}

function applyActionVisibility() {
  const short = (actionInfo.action || "").replace(PREFIX, "");
  document.querySelectorAll("[data-actions]").forEach((el) => {
    const allowed = el.getAttribute("data-actions").split(/\s+/);
    const show = allowed.includes("*") || allowed.includes(short);
    el.style.display = show ? "" : "none";
  });
  const tally = document.getElementById("tally_check");
  if (tally && !tally.value) {
    if (short === "select.pgm") tally.value = "pgm";
    if (short === "select.pst") tally.value = "prv";
  }
  applyConnectionUi();
}

function endpointValue(endpoint) {
  return endpoint.host + "\t" + (endpoint.password || "0000");
}

function renderEndpoints(endpoints) {
  const pick = document.getElementById("connection_pick");
  if (!pick) return;
  const host = document.getElementById("host");
  const password = document.getElementById("password");
  const mode = document.getElementById("connection_mode");
  pick.innerHTML = "";
  addOption(pick, "manual", "Enter IP");
  const hostCounts = {};
  (endpoints || []).forEach((endpoint) => {
    hostCounts[endpoint.host] = (hostCounts[endpoint.host] || 0) + 1;
  });
  (endpoints || []).forEach((endpoint) => {
    const label = hostCounts[endpoint.host] > 1
      ? `${endpoint.host} (${endpoint.password}) · ${endpoint.status}`
      : `${endpoint.host} · ${endpoint.status}`;
    addOption(pick, endpointValue(endpoint), label);
  });
  const savedValue = host && host.value.trim()
    ? `${host.value.trim()}\t${(password && password.value) || "0000"}`
    : "";
  const hasMatch = savedValue && [...pick.options].some((opt) => opt.value === savedValue);
  const others = (endpoints || []).filter((endpoint) => endpointValue(endpoint) !== savedValue);
  if (mode && mode.value === "saved" && hasMatch) {
    pick.value = savedValue;
  } else if (mode && !mode.value && hasMatch && others.length > 0) {
    pick.value = savedValue;
    mode.value = "saved";
  } else {
    pick.value = "manual";
    if (mode && !mode.value) mode.value = "manual";
  }
  applyConnectionUi();
}

function applyConnectionUi() {
  const pick = document.getElementById("connection_pick");
  const manual = !pick || pick.value === "manual";
  const short = (actionInfo.action || "").replace(PREFIX, "");
  document.querySelectorAll("[data-manual-only]").forEach((el) => {
    const allowed = (el.getAttribute("data-actions") || "*").split(/\s+/);
    const actionShow = allowed.includes("*") || allowed.includes(short);
    el.style.display = actionShow && manual ? "" : "none";
  });
}

function onConnectionPick() {
  const pick = document.getElementById("connection_pick");
  const mode = document.getElementById("connection_mode");
  const host = document.getElementById("host");
  const password = document.getElementById("password");
  if (!pick || !mode) return;
  if (pick.value === "manual") {
    mode.value = "manual";
  } else {
    const parts = pick.value.split("\t");
    mode.value = "saved";
    if (host) host.value = parts[0] || "";
    if (password) password.value = parts[1] || "0000";
  }
  applyConnectionUi();
  setSettings();
}

function attachPiMessages() {
  if (!websocket) return;
  const original = websocket.onmessage;
  websocket.onmessage = function (evt) {
    const jsonObj = JSON.parse(evt.data);
    if (jsonObj.event === "sendToPropertyInspector" && jsonObj.payload) {
      if (jsonObj.payload.status) {
        const el = document.getElementById("connectionStatus");
        if (el) el.textContent = jsonObj.payload.status;
      }
      if (jsonObj.payload.endpoints) {
        renderEndpoints(jsonObj.payload.endpoints);
      }
      return;
    }
    if (original) original(evt);
    applyConnectionUi();
  };
}

function testConnection() {
  const pick = document.getElementById("connection_pick");
  if (pick && pick.value !== "manual") return;
  const status = document.getElementById("connectionStatus");
  if (status) status.textContent = "Testing…";
  sendPayloadToPlugin({
    command: "test_connection",
    host: (document.getElementById("host") || {}).value || "",
    password: (document.getElementById("password") || {}).value || "0000",
  });
}

document.addEventListener("websocketCreate", () => {
  fillSelects();
  applyActionVisibility();
  attachPiMessages();
});
