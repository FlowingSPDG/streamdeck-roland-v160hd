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
      ["pst_b_1", "PST/B 1"], ["pst_b_2", "PST/B 2"], ["pst_b_3", "PST/B 3"], ["pst_b_4", "PST/B 4"],
      ["pst_b_5", "PST/B 5"], ["pst_b_6", "PST/B 6"], ["pst_b_7", "PST/B 7"], ["pst_b_8", "PST/B 8"],
      ["pst_b_9", "PST/B 9"], ["pst_b_10", "PST/B 10"],
      ["aux_1", "AUX 1"], ["aux_2", "AUX 2"], ["aux_3", "AUX 3"], ["aux_4", "AUX 4"], ["aux_5", "AUX 5"],
      ["aux_6", "AUX 6"], ["aux_7", "AUX 7"], ["aux_8", "AUX 8"], ["aux_9", "AUX 9"], ["aux_10", "AUX 10"],
      ["cut", "CUT"], ["auto", "AUTO"], ["transition", "TRANSITION"], ["mode", "MODE"],
      ["input_assign", "INPUT ASSIGN"], ["pgm_center", "PGM encoder"], ["pst_center", "PST encoder"],
      ["split_a", "SPLIT A"], ["split_b", "SPLIT B"], ["auto_mixing", "AUTO MIXING"], ["capture", "CAPTURE"],
      ["user_1", "USER 1"], ["user_2", "USER 2"], ["user_3", "USER 3"], ["user_4", "USER 4"],
      ["pinp1_pgm", "PinP1 PGM"], ["pinp1_pvw", "PinP1 PVW"], ["pinp2_pgm", "PinP2 PGM"], ["pinp2_pvw", "PinP2 PVW"],
      ["pinp3_pgm", "PinP3 PGM"], ["pinp3_pvw", "PinP3 PVW"], ["pinp4_pgm", "PinP4 PGM"], ["pinp4_pvw", "PinP4 PVW"],
      ["dsk1_pgm", "DSK1 PGM"], ["dsk1_pvw", "DSK1 PVW"], ["dsk2_pgm", "DSK2 PGM"], ["dsk2_pvw", "DSK2 PVW"],
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
}

function attachPiMessages() {
  if (!websocket) return;
  const original = websocket.onmessage;
  websocket.onmessage = function (evt) {
    const jsonObj = JSON.parse(evt.data);
    if (jsonObj.event === "sendToPropertyInspector" && jsonObj.payload && jsonObj.payload.status) {
      const el = document.getElementById("connectionStatus");
      if (el) el.textContent = jsonObj.payload.status;
      return;
    }
    if (original) original(evt);
  };
}

function testConnection() {
  sendPayloadToPlugin({ command: "test_connection" });
}

document.addEventListener("websocketCreate", () => {
  fillSelects();
  applyActionVisibility();
  attachPiMessages();
});
