// Check environment
const tauri = window.__TAURI__;
const invoke = tauri?.core?.invoke || tauri?.invoke || (async (cmd, args) => {
  console.log(`[Mock Invoke] ${cmd}`, args);
  if (cmd === 'get_idle_seconds') return 0; // Simulate activity
  if (cmd === 'start_drag') console.log("Simulating Window Drag");
  if (cmd === 'start_resize_drag') console.log("Simulating Window Resize Drag", args);
  return null;
});

document.addEventListener('DOMContentLoaded', () => {
  // --- Elements ---
  const appCircle = document.getElementById('app-circle');
  const boltIcon = document.querySelector('.pavlok-bolt');
  const modeBtn = document.getElementById('alert-mode-btn');
  const resizeHandles = document.querySelectorAll('.resize-handle');

  const workInput = document.getElementById('work-timer');
  const breakInput = document.getElementById('break-timer');
  const apiInput = document.getElementById('api-token');

  const fatigueDisplay = document.getElementById('fatigue-display');
  const fatigueValueNumber = document.getElementById('fatigue-value-number');
  const boltFillRect = document.getElementById('bolt-fill-rect');

  // Icons
  const icons = {
    beep: document.getElementById('icon-beep'),
    vibro: document.getElementById('icon-vibro'),
    zap: document.getElementById('icon-zap')
  };

  // --- State ---
  const modes = ['beep', 'vibro', 'zap'];
  let currentModeIndex = 0; // Start with beep
  let isMonitoring = false;

  // Logic State
  let fatigue = 0;
  let restStreak = 0;
  let activeSeconds = 0;
  let secondCounter = 0;
  let lastAlertTime = 0;
  let apiKeyInvalid = false;
  let lastTickAt = null;

  function setProgress(ringPercent, displayPercent = ringPercent) {
    const normalized = Math.max(0, Math.min(100, ringPercent));
    if (boltFillRect) {
      const boltFill = normalized / 100;
      const height = 24 * boltFill;
      const y = 24 - height;
      boltFillRect.setAttribute("y", `${y}`);
      boltFillRect.setAttribute("height", `${height}`);
    }
    if (fatigueValueNumber) {
      fatigueValueNumber.textContent = `${Math.round(displayPercent)}`;
    } else if (fatigueDisplay) {
      fatigueDisplay.textContent = `${Math.round(displayPercent)}%`;
    }
  }

  function getWorkLimit() {
    return Math.max(1, parseInt(workInput.value) || 45);
  }

  function getBreakLimit() {
    return Math.max(1, parseInt(breakInput.value) || 5);
  }

  function refreshFatigueUI() {
    const workLimit = getWorkLimit();
    const fatiguePercent = (fatigue / workLimit) * 100;
    setProgress(fatiguePercent, fatiguePercent);
    updateApiWarningState();
  }

  function applyMinute() {
    const workLimit = getWorkLimit();
    const breakLimit = getBreakLimit();
    const minuteActiveSeconds = activeSeconds;
    const wasAtLimit = fatigue >= workLimit;

    if (minuteActiveSeconds >= 10) {
      fatigue++;
      restStreak = 0;
    } else {
      if (fatigue > 0) fatigue--;
      restStreak++;
    }

    activeSeconds = 0;
    secondCounter = 0;

    if (restStreak >= breakLimit) {
      fatigue = 0;
    }

    const isAtLimit = fatigue >= workLimit;
    if (isAtLimit) {
      const now = Date.now();
      const crossedLimitNow = !wasAtLimit;
      const fullyActiveMinute = minuteActiveSeconds >= 60;
      const canRepeatAtLimit = wasAtLimit && fullyActiveMinute;

      // Safety rule:
      // - first alert: immediately when crossing to 100%
      // - repeat alerts at/over 100%: only after a fully active minute
      if ((crossedLimitNow || canRepeatAtLimit) && now - lastAlertTime > 60000) {
        sendAlert();
        lastAlertTime = now;
      }
    }

    refreshFatigueUI();
  }

  function applySampleSeconds(activeSecs, inactiveSecs) {
    const totalSeconds = Math.max(0, activeSecs + inactiveSecs);
    if (totalSeconds === 0) return;

    if (activeSecs > 0) {
      activeSeconds += activeSecs;
    }

    let remaining = totalSeconds;
    while (remaining > 0) {
      const toMinuteBoundary = 60 - secondCounter;
      const step = Math.min(remaining, toMinuteBoundary);
      secondCounter += step;
      remaining -= step;

      if (secondCounter >= 60) {
        applyMinute();
      }
    }
  }

  function isAtLimit() {
    return fatigue >= getWorkLimit();
  }

  function updateApiWarningState() {
    appCircle.classList.toggle('api-key-invalid', isAtLimit() && apiKeyInvalid);
    updateBoltTooltip();
  }

  // --- Persistence & Initialization ---
  if (localStorage.getItem('workTime')) workInput.value = localStorage.getItem('workTime');
  if (localStorage.getItem('breakTime')) breakInput.value = localStorage.getItem('breakTime');
  if (localStorage.getItem('apiToken')) apiInput.value = localStorage.getItem('apiToken');

  // Tray action: reset fatigue from native menu
  if (tauri?.event?.listen) {
    tauri.event.listen("reset-fatigue", () => {
      resetFatigue();
    });
  }

  updateModeUI();
  updateBoltTooltip();
  setProgress(0, 0);

  function getResizeDirectionFromPointerEvent(e) {
    const rect = appCircle.getBoundingClientRect();
    const hit = 16;
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const nearLeft = x <= hit;
    const nearRight = x >= rect.width - hit;
    const nearTop = y <= hit;
    const nearBottom = y >= rect.height - hit;

    if (nearTop && nearLeft) return 'nw';
    if (nearTop && nearRight) return 'ne';
    if (nearBottom && nearLeft) return 'sw';
    if (nearBottom && nearRight) return 'se';
    if (nearTop) return 'n';
    if (nearBottom) return 's';
    if (nearLeft) return 'w';
    if (nearRight) return 'e';
    return null;
  }

  // --- DRAG FUNCTIONALITY (Agenda Style) ---
  appCircle.addEventListener('mousedown', async (e) => {
    const edgeDirection = getResizeDirectionFromPointerEvent(e);
    if (edgeDirection) {
      await invoke('start_resize_drag', { direction: edgeDirection });
      return;
    }

    if (e.target.closest('.control-btn') ||
      e.target.closest('.timers-container') ||
      e.target.closest('.api-container') ||
      e.target.closest('.pavlok-bolt') ||
      e.target.closest('.resize-handle')) {
      return;
    }

    await invoke('start_drag');
  });

  resizeHandles.forEach(handle => {
    handle.addEventListener('pointerdown', async (e) => {
      e.preventDefault();
      e.stopPropagation();
      const direction = handle.dataset.dir;
      if (!direction) return;
      try {
        await invoke('start_resize_drag', { direction });
      } catch (err) {
        console.error("Resize drag failed:", err);
      }
    });
  });

  // --- Event Listeners ---

  // 1. Bolt Click (Toggle Monitor)
  boltIcon.addEventListener('click', (e) => {
    e.stopPropagation();
    isMonitoring = !isMonitoring;
    updateMonitoringState();
    triggerHapticVisual(boltIcon);
    triggerChargeBurst();
  });

  // 2. Mode Toggle
  modeBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    currentModeIndex = (currentModeIndex + 1) % modes.length;
    updateModeUI();
    triggerHapticVisual(modeBtn);
  });

  // 5. API Token
  if (apiInput) {
    apiInput.addEventListener('change', () => {
      localStorage.setItem('apiToken', apiInput.value);
      apiKeyInvalid = false;
      updateApiWarningState();
    });
    apiInput.addEventListener('input', () => {
      apiKeyInvalid = false;
      updateApiWarningState();
    });
    apiInput.addEventListener('mousedown', (e) => e.stopPropagation());
  }

  // 6. Timer Inputs
  [workInput, breakInput].forEach(input => {
    input.addEventListener('change', () => validateAndSave(input));
    input.addEventListener('keyup', () => validateAndSave(input, false));
    input.addEventListener('blur', () => validateAndSave(input, true));
    input.addEventListener('wheel', (e) => {
      e.preventDefault();
      let val = parseInt(input.value) || 0;
      if (e.deltaY < 0) val++; else val--;
      if (val < 1) val = 1;
      if (val > 99) val = 99;
      input.value = val;
      validateAndSave(input);
    });
    input.addEventListener('mousedown', (e) => e.stopPropagation());
  });

  // --- Monitoring Loop (1s Tick) ---
  setInterval(async () => {
    if (!isMonitoring) return;

    try {
      const now = Date.now();
      if (lastTickAt == null) {
        lastTickAt = now;
      }
      const elapsedSeconds = Math.max(1, Math.floor((now - lastTickAt) / 1000));
      lastTickAt = now;

      const idleSeconds = await invoke('get_idle_seconds');
      const activeNow = idleSeconds < 2.0;
      const missedSeconds = Math.max(0, elapsedSeconds - 1);
      applySampleSeconds(activeNow ? 1 : 0, (activeNow ? 0 : 1) + missedSeconds);

    } catch (e) {
      console.error("Invoke Error:", e);
    }

  }, 1000);

  async function sendAlert() {
    const token = apiInput.value;
    const type = modes[currentModeIndex];

    if (!token || !token.trim()) {
      console.warn("Alert Triggered but NO TOKEN set.");
      apiKeyInvalid = true;
      updateApiWarningState();
      return;
    }

    console.log(`Sending Alert: ${type}`);
    try {
      const res = await invoke('send_pavlok_alert', {
        token: token,
        stimulusType: type
      });
      console.log("Alert Result:", res);
      if (res === "Sent") {
        apiKeyInvalid = false;
      } else if (typeof res === "string" && /^Error:\s*(401|403)\b/.test(res)) {
        apiKeyInvalid = true;
      }
      updateApiWarningState();
    } catch (e) {
      console.error("Alert Failed:", e);
    }
  }

  // --- Helpers ---
  function validateAndSave(input, forceClamp = true) {
    let val = parseInt(input.value);
    if (isNaN(val)) val = 0;
    if (forceClamp) {
      if (val < 1) val = 1;
      if (val > 99) val = 99;
      input.value = val;
    }
    if (input.id === 'work-timer') localStorage.setItem('workTime', val);
    if (input.id === 'break-timer') localStorage.setItem('breakTime', val);

    refreshFatigueUI();
  }

  function updateModeUI() {
    const mode = modes[currentModeIndex];
    Object.values(icons).forEach(icon => icon.classList.remove('active'));
    if (icons[mode]) icons[mode].classList.add('active');
    modeBtn.setAttribute('data-mode', mode);
    modeBtn.title = capitalize(mode);
  }

  function updateMonitoringState() {
    refreshFatigueUI();

    if (isMonitoring) {
      // New run starts a fresh minute window; fatigue value is preserved.
      activeSeconds = 0;
      secondCounter = 0;
      lastTickAt = Date.now();
      appCircle.classList.add('monitoring');
    } else {
      lastTickAt = null;
      appCircle.classList.remove('monitoring');
    }
  }

  function resetFatigue() {
    fatigue = 0;
    restStreak = 0;
    activeSeconds = 0;
    secondCounter = 0;
    lastAlertTime = 0;
    lastTickAt = Date.now();
    setProgress(0, 0);
  }

  function triggerHapticVisual(element) {
    element.style.transform = "scale(0.9)";
    setTimeout(() => {
      element.style.transform = "";
    }, 100);
  }

  function updateBoltTooltip() {
    const tooltip = (isAtLimit() && apiKeyInvalid)
      ? "API key invalid: alert not sent"
      : (isMonitoring ? "Stop" : "Start");
    boltIcon.setAttribute("title", tooltip);
    boltIcon.setAttribute("aria-label", tooltip);
  }

  function triggerChargeBurst() {
    appCircle.classList.remove('charge-burst-active');
    void appCircle.offsetWidth;
    appCircle.classList.add('charge-burst-active');
    setTimeout(() => {
      appCircle.classList.remove('charge-burst-active');
    }, 650);
  }

  function capitalize(str) {
    return str.charAt(0).toUpperCase() + str.slice(1);
  }
});
