<script lang="ts">
  import { ProgressBarStatus } from "@tauri-apps/api/window";
  import Button from "../lib/components/Button.svelte";
  import Modal from "../lib/components/Modal.svelte";
  let presets = $state<number[]>([120, 600, 1200]);
  let selectedSeconds = $state(120);
  let remainingSeconds = $state(selectedSeconds);
  let isRunning = $state(false);
  let intervalId: number | null = null;

  // Edit modal state
  let editOpen = $state(false);
  let editingIndex = $state<number | null>(null);
  let editHours = $state(0);
  let editMinutes = $state(2);
  let editSeconds = $state(0);
  let settingsOpen = $state(false);

  // Auto-messages & zones
  let autoMessagesEnabled = $state(true);
  // Thresholds: OK (>= criticalPercent), then CRITICAL (>= dangerPercent), then DANGER (closest to end)
  let criticalPercent = $state(50); // between criticalPercent and dangerPercent ⇒ CRITICAL
  let dangerPercent = $state(20); // <= dangerPercent ⇒ DANGER
  type UserMessage = { percent: number; text: string; fired?: boolean };
  type MessageSound = "beep" | "heartbeat" | "none" | "custom";
  type UserMessageExt = UserMessage & { sound?: MessageSound; custom?: string };
  let userMessages = $state<UserMessageExt[]>([
    { percent: 50, text: "Halfway there!" },
  ]);
  let prevZone: "OK" | "DANGER" | "CRITICAL" | null = null;
  // Sound settings
  let dangerSound: "heartbeat" | "beep" | "none" = $state("heartbeat");
  let criticalSound: "heartbeat" | "beep" | "none" = $state("none");
  type EndSound = "triple" | "beep" | "heartbeat" | "none" | "custom";
  let endSound: EndSound = $state("triple");
  let endSoundCustom: string | undefined = undefined;
  let hasFiredEndSound = false;
  let overlayOpen = $state(false);
  // End question settings
  let askOnEnd = $state(true);
  let endQuestionText = $state('Did you achieve your goal?');
  let showEndQuestion = $state(false);
  function isFreshStart() {
    return !isRunning && remainingSeconds === selectedSeconds;
  }

  // Simple persistent stats (localStorage)
  type Stats = { wins: number; losses: number };
  let wins = $state(0);
  let losses = $state(0);
  type RecordEntry = { ts: number; result: 'win' | 'loss' };
  function loadRecords(): RecordEntry[] { try { const s = localStorage.getItem('timerRecords'); if (s) return JSON.parse(s); } catch {} return []; }
  function saveRecords(recs: RecordEntry[]) { try { localStorage.setItem('timerRecords', JSON.stringify(recs)); } catch {} }
  function todayKey(d = new Date()) { return d.toISOString().slice(0,10); }
  function recomputeToday() {
    const recs = loadRecords();
    const key = todayKey();
    let w = 0, l = 0;
    for (const r of recs) {
      const rk = new Date(r.ts).toISOString().slice(0,10);
      if (rk === key) { if (r.result === 'win') w++; else l++; }
    }
    wins = w; losses = l;
  }
  function loadStats() {
    recomputeToday();
    try {
      const s = localStorage.getItem('endQuestionSettings');
      if (s) {
        const v = JSON.parse(s);
        if (typeof v.askOnEnd === 'boolean') askOnEnd = v.askOnEnd;
        if (typeof v.endQuestionText === 'string') endQuestionText = v.endQuestionText;
      }
    } catch {}
  }
  function saveStats() {
    // Derived from records; nothing to do
  }
  function persistEndQuestionSettings() {
    try { localStorage.setItem('endQuestionSettings', JSON.stringify({ askOnEnd, endQuestionText })); } catch {}
  }

  // Heatmap helpers (inside script)
  function startOfYear(d = new Date()) { const dt = new Date(d.getFullYear(), 0, 1); dt.setHours(0,0,0,0); return dt; }
  function sameDayKey(d: Date) {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }
  function countByDay(): Record<string, { wins: number; losses: number }> {
    const recs = loadRecords();
    const res: Record<string, { wins: number; losses: number }> = {};
    for (const r of recs) {
      const k = sameDayKey(new Date(r.ts));
      if (!res[k]) res[k] = { wins: 0, losses: 0 };
      if (r.result === 'win') res[k].wins++; else res[k].losses++;
    }
    return res;
  }
  function buildYearDays() {
    const map = countByDay();
    const days = [] as Array<{ date: Date; key: string; wins: number; losses: number }>;
    const now = new Date();
    // End at end-of-today so today is always included
    const end = new Date(now); end.setHours(23,59,59,999);
    let cur = startOfYear(now);
    while (cur <= end) {
      const key = sameDayKey(cur);
      const v = map[key] || { wins: 0, losses: 0 };
      days.push({ date: new Date(cur), key, wins: v.wins, losses: v.losses });
      cur = new Date(cur.getTime() + 24*60*60*1000);
    }
    return days;
  }
  function buildYearWeeks() {
    const days = buildYearDays();
    const weeks: Array<Array<{ date: Date; key: string; wins: number; losses: number }>> = [];
    let week: Array<{ date: Date; key: string; wins: number; losses: number }> = [];
    for (const d of days) {
      if (d.date.getDay() === 0 && week.length) { // Sunday starts a new column
        weeks.push(week);
        week = [];
      }
      week.push(d);
    }
    if (week.length) weeks.push(week);
    // Pad last week to 7 entries for grid alignment
    const last = weeks[weeks.length - 1];
    if (last && last.length < 7) {
      let cur = new Date(last[last.length - 1].date.getTime());
      while (last.length < 7) {
        cur = new Date(cur.getTime() + 24*60*60*1000);
        last.push({ date: cur, key: sameDayKey(cur), wins: 0, losses: 0 });
      }
    }
    return weeks;
  }
  function tooltipFor(key: string) {
    const recs = loadRecords().filter(r => sameDayKey(new Date(r.ts)) === key);
    const lines = recs
      .sort((a,b)=>a.ts-b.ts)
      .map(r => {
        const d = new Date(r.ts);
        const hh = String(d.getHours()).padStart(2,'0');
        const mm = String(d.getMinutes()).padStart(2,'0');
        return `${hh}:${mm} — ${r.result}`;
      });
    const summary = `Wins: ${recs.filter(r=>r.result==='win').length}, Losses: ${recs.filter(r=>r.result==='loss').length}`;
    return [key, summary, ...lines].join('\n');
  }
  function colorFor(w: number, l: number) {
    const total = w + l;
    if (total === 0) return '#1f1f1f';
    const ratio = w / total; // 1 => all wins, 0 => all losses
    const green = Math.round(100 + 155 * ratio);
    const red = Math.round(100 + 155 * (1 - ratio));
    return `rgb(${red},${green},120)`;
  }
  loadStats();
  let endRecorded = $state(false);
  async function handleTimerEnd() {
    if (endRecorded) return;
    endRecorded = true;
    showEndQuestion = askOnEnd;
  }

  function answerEnd(win: boolean) {
    if (endRecorded) return; // Already answered, prevent duplicate
    endRecorded = true;
    showEndQuestion = false;
    const now = Date.now();
    const recs = loadRecords();
    recs.push({ ts: now, result: win ? 'win' : 'loss' });
    saveRecords(recs);
    recomputeToday();
    // Let overlay hide its question too
    try { broadcast({ type: 'result', ts: now, result: win ? 'win' : 'loss' }); } catch {}
  }

  async function setTaskbarProgress(progress?: number) {
    const isTauri =
      typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
    if (!isTauri) return;
    const { getCurrentWebviewWindow } = await import(
      "@tauri-apps/api/webviewWindow"
    );
    const win = getCurrentWebviewWindow();
    try {
      if (progress === undefined) {
        await win.setProgressBar({ progress: undefined, status: ProgressBarStatus.Normal });
      } else {
        const p = Math.max(0, Math.min(1, progress));
        // Nudge macOS Dock to show immediately
        if (p === 0) await win.setProgressBar({ progress: 0.001, status: ProgressBarStatus.Normal });
        await win.setProgressBar({ progress: p, status: ProgressBarStatus.Normal }); // status helps on Windows
      }
    } catch {}
  }
  let audioCtx: AudioContext | null = null;
  let dangerSoundTimer: number | null = null;
  // Colors
  let dangerColor = $state("#ff4444");
  let criticalColor = $state("#ffa500");
  // Sync
  let startAtMs: number | null = null;
  let runDurationSeconds = $state(selectedSeconds);
  let lastStartSig: number | null = null;
  let bc: BroadcastChannel | null = null;
  let suppressBroadcast = false;

  function formatTime(totalSeconds: number) {
    const m = Math.floor(totalSeconds / 60)
      .toString()
      .padStart(2, "0");
    const s = Math.floor(totalSeconds % 60)
      .toString()
      .padStart(2, "0");
    return `${m}:${s}`;
  }

  function setPreset(seconds: number) {
    selectedSeconds = seconds;
    reset();
    broadcast({ type: "preset", duration: selectedSeconds });
  }

  function start() {
    if (isRunning) return;
    isRunning = true;
    if (remainingSeconds <= 0) remainingSeconds = selectedSeconds;
    runDurationSeconds = selectedSeconds;
    startAtMs = Date.now() - (runDurationSeconds - remainingSeconds) * 1000;
    lastStartSig = startAtMs;
    if (intervalId !== null) {
      clearInterval(intervalId);
      intervalId = null;
    }
    intervalId = window.setInterval(() => {
      if (!startAtMs) return;
      const elapsed = Math.floor((Date.now() - startAtMs) / 1000);
      const nextRemaining = Math.max(0, runDurationSeconds - elapsed);
      if (nextRemaining !== remainingSeconds) {
        remainingSeconds = nextRemaining;
        handleTick();
        // Update taskbar/dock progress from main window only
        setTaskbarProgress(
          (runDurationSeconds - remainingSeconds) /
            Math.max(1, runDurationSeconds)
        );
      }
      if (remainingSeconds === 0) {
        playEndSound();
        stopDangerSound();
        stopCriticalSound();
        handleTimerEnd();
        if (!suppressBroadcast) broadcast({ type: "end" });
        stop();
      }
    }, 1000);
    if (!suppressBroadcast)
      broadcast({
        type: "start",
        duration: runDurationSeconds,
        startAtMs,
        critical: criticalPercent,
        danger: dangerPercent,
        snd: dangerSound,
        csnd: criticalSound,
        colors: { dangerColor, criticalColor },
        messages: userMessages.map(({ percent, text }) => ({ percent, text })),
      });
  }

  function stop() {
    isRunning = false;
    if (intervalId !== null) {
      clearInterval(intervalId);
      intervalId = null;
    }
    startAtMs = null;
    stopDangerSound();
    stopCriticalSound();
    setTaskbarProgress(undefined);
    if (!suppressBroadcast)
      broadcast({
        type: "pause",
        remaining: remainingSeconds,
        duration: selectedSeconds,
      });
  }

  function reset() {
    stop();
    remainingSeconds = selectedSeconds;
    prevZone = null;
    userMessages = userMessages.map((m) => ({ ...m, fired: false }));
    startAtMs = null;
    hasFiredEndSound = false;
    endRecorded = false;
    setTaskbarProgress(undefined);
    if (!suppressBroadcast)
      broadcast({ type: "reset", duration: selectedSeconds });
  }

  function openEdit(index: number) {
    editingIndex = index;
    const total = presets[index];
    editHours = Math.floor(total / 3600);
    editMinutes = Math.floor((total % 3600) / 60);
    editSeconds = total % 60;
    editOpen = true;
  }

  function saveEdit() {
    if (editingIndex === null) return;
    const secs = Math.max(1, editHours * 3600 + editMinutes * 60 + editSeconds);
    const old = presets[editingIndex];
    const updated = [...presets];
    updated[editingIndex] = secs;
    presets = Array.from(new Set(updated)).sort((a, b) => a - b);
    if (selectedSeconds === old) setPreset(secs);
    closeEdit();
  }

  function closeEdit() {
    editOpen = false;
    editingIndex = null;
  }

  function addFromEdit() {
    const secs = Math.max(1, editHours * 3600 + editMinutes * 60 + editSeconds);
    presets = Array.from(new Set([...presets, secs])).sort((a, b) => a - b);
    closeEdit();
  }

  function removePresetAt(index: number) {
    const val = presets[index];
    presets = presets.filter((_, i) => i !== index);
    if (selectedSeconds === val) setPreset(presets[0] ?? 60);
  }

  function minutes(secs: number) {
    return Math.round(secs / 60);
  }

  function resetAll() {
    try {
      localStorage.clear();
    } catch {}
    // Defaults
    criticalPercent = 50;
    dangerPercent = 20;
    dangerSound = "heartbeat";
    criticalSound = "none";
    dangerColor = "#ff4444";
    criticalColor = "#ffa500";
    userMessages = [{ percent: 50, text: "Halfway there!", sound: "beep" }];
    selectedSeconds = 120;
    remainingSeconds = selectedSeconds;
    isRunning = false;
    startAtMs = null;
    broadcast({ type: "reset", duration: selectedSeconds });
  }

  async function openOverlayAndStart() {
    const isTauri =
      typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
    const startedAt = isRunning
      ? Date.now() - (selectedSeconds - remainingSeconds) * 1000
      : Date.now();
    const params = new URLSearchParams({
      duration: String(selectedSeconds),
      autostart: "0",
      critical: String(criticalPercent),
      danger: String(dangerPercent),
      msgs: encodeURIComponent(
        btoa(
          unescape(
            encodeURIComponent(
              JSON.stringify(
                userMessages.map(({ percent, text, sound, custom }) => ({
                  percent,
                  text,
                  sound,
                  custom,
                }))
              )
            )
          )
        )
      ),
      snd: dangerSound,
      csnd: criticalSound,
      startAt: String(Math.floor(startedAt)),
      dangerColor,
      criticalColor,
      ask: askOnEnd ? '1' : '0',
      q: encodeURIComponent(endQuestionText)
    });
    if (!isTauri) {
      location.href = `/overlay?${params.toString()}`;
      return;
    }
    const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const existing = await WebviewWindow.getByLabel("overlay");
    if (existing) {
      // Toggle: close overlay
      try {
        await existing.close();
      } catch {}
      overlayOpen = false;
      return;
    }
    const w = new WebviewWindow("overlay", {
      url: `/overlay?${params.toString()}`,
      title: "Timer Overlay",
      width: 260,
      height: 140,
      resizable: false,
      decorations: false,
      alwaysOnTop: true,
      focus: true,
      visible: true,
      shadow: true,
      skipTaskbar: true,
      x: 0,
      y: 0,
    });
    try {
      await new Promise<void>((resolve, reject) => {
        w.once("tauri://created", () => resolve());
        w.once("tauri://error", (e) => reject(e));
      });
      await w.setAlwaysOnTop(true);
      await w.setFocus();
      await w.show();
      // @ts-ignore center may not be typed
      await w.center?.();
      overlayOpen = true;
      // Track close to update state
      w.listen("tauri://destroyed", () => {
        overlayOpen = false;
      });
    } catch (e) {
      console.error("Failed to show overlay window", e);
    }
  }

  async function openPictureInPicture(totalSeconds: number) {
    // Create a canvas we can stream into PiP
    const width = 360,
      height = 200;
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("Canvas 2D context not available");

    // Create a video from the canvas stream
    const stream = (canvas as HTMLCanvasElement).captureStream?.(30);
    if (!stream) throw new Error("captureStream not supported");
    const video = document.createElement("video");
    video.muted = true; // required by autoplay policies
    (video as any).disablePictureInPicture = false;
    video.srcObject = stream;
    video.playsInline = true;
    video.style.position = "fixed";
    video.style.width = "1px";
    video.style.height = "1px";
    video.style.opacity = "0";
    video.style.pointerEvents = "none";
    document.body.appendChild(video);
    await video.play();

    // Start the countdown locally for the PiP view
    const startTs = Date.now();
    const endTs = startTs + totalSeconds * 1000;

    function formatTime(sec: number) {
      const m = Math.floor(sec / 60)
        .toString()
        .padStart(2, "0");
      const s = Math.max(0, Math.floor(sec % 60))
        .toString()
        .padStart(2, "0");
      return `${m}:${s}`;
    }

    let rafId = 0;
    const render = () => {
      const now = Date.now();
      const remainingMs = Math.max(0, endTs - now);
      const remainingSec = Math.ceil(remainingMs / 1000);
      const elapsed = totalSeconds - remainingSec;
      const progress = Math.min(1, Math.max(0, elapsed / totalSeconds));

      // background
      ctx.fillStyle = "#0b0b0b";
      ctx.fillRect(0, 0, width, height);

      // time text
      ctx.fillStyle = "#e8e8e8";
      ctx.font = "bold 42px Inter, system-ui, -apple-system, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(formatTime(remainingSec), width / 2, height / 2 - 16);

      // progress background
      const barW = width - 40,
        barH = 10,
        barX = 20,
        barY = height - 40;
      ctx.fillStyle = "#242424";
      ctx.fillRect(barX, barY, barW, barH);

      // progress fill (light green)
      ctx.fillStyle = "#8ef59b";
      ctx.fillRect(barX, barY, barW * progress, barH);

      if (remainingMs > 0) {
        rafId = requestAnimationFrame(render);
      }
    };
    render();

    // Enter PiP (standard + Safari fallback)
    let pipWindow: any = null;
    try {
      // @ts-ignore
      if (
        document.pictureInPictureEnabled &&
        (video as any).requestPictureInPicture
      ) {
        // @ts-ignore
        pipWindow = await (video as any).requestPictureInPicture();
      } else if ((video as any).webkitSetPresentationMode) {
        // Safari
        (video as any).webkitSetPresentationMode("picture-in-picture");
        pipWindow = true;
      } else {
        throw new Error("Picture-in-Picture not supported");
      }
    } catch (e) {
      // Clean up before rethrow to trigger fallback
      video.pause();
      (video.srcObject as MediaStream)?.getTracks().forEach((t) => t.stop());
      video.srcObject = null;
      video.remove();
      throw e;
    }

    // Clean up when PiP closes
    const onLeave = () => {
      cancelAnimationFrame(rafId);
      video.pause();
      (video.srcObject as MediaStream)?.getTracks().forEach((t) => t.stop());
      video.srcObject = null;
      video.remove();
      (document as any).removeEventListener("leavepictureinpicture", onLeave);
    };
    (document as any).addEventListener("leavepictureinpicture", onLeave, {
      once: true,
    });

    // Also start the main timer if not already running
    if (!isRunning) {
      remainingSeconds = totalSeconds;
      start();
    }

    return pipWindow;
  }

  function currentPercent() {
    return Math.max(
      0,
      Math.round((remainingSeconds / Math.max(1, selectedSeconds)) * 100)
    );
  }

  function zoneFromPercent(p: number): "OK" | "CRITICAL" | "DANGER" {
    if (p <= dangerPercent) return "DANGER";
    if (p <= criticalPercent) return "CRITICAL";
    return "OK";
  }

  function requestNotifyPermission() {
    if (
      typeof Notification !== "undefined" &&
      Notification.permission === "default"
    ) {
      Notification.requestPermission();
    }
  }

  function notifyDesktop(title: string, body: string) {
    try {
      if (
        typeof Notification !== "undefined" &&
        Notification.permission === "granted"
      )
        new Notification(title, { body });
    } catch {}
    // Tauri fallback via plugin-notification
    const isTauri =
      typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
    if (isTauri) {
      (async () => {
        try {
          const notif = await import("@tauri-apps/plugin-notification");
          const granted = await notif.isPermissionGranted();
          if (!granted) await notif.requestPermission();
          await notif.sendNotification({ title, body });
        } catch {}
      })();
    }
  }

  // Message sounds
  const audioCache: Record<string, HTMLAudioElement> = {};
  function playMessageSound(kind: MessageSound | undefined, custom?: string) {
    const k = kind ?? "beep";
    if (k === "none") return;
    if (k === "beep") {
      playBeep(200, 1000);
      return;
    }
    if (k === "heartbeat") {
      playHeartbeat();
      return;
    }
    if (k === "custom" && custom) {
      let el = audioCache[custom];
      if (!el) {
        el = new Audio(custom);
        audioCache[custom] = el;
      }
      try {
        el.currentTime = 0;
        el.play();
      } catch {}
    }
  }

  function handleTick() {
    if (!autoMessagesEnabled) return;
    const pct = currentPercent();
    const zone = zoneFromPercent(pct);
    if (zone !== prevZone) {
      prevZone = zone;
      if (zone === "DANGER" || zone === "CRITICAL")
        notifyDesktop("Timer", `Entered ${zone} zone`);
      if (zone === "DANGER") {
        startDangerSound();
        stopCriticalSound();
      } else if (zone === "CRITICAL") {
        stopDangerSound();
        startCriticalSound();
      } else {
        stopDangerSound();
        stopCriticalSound();
      }
    }
    for (const m of userMessages) {
      if (!m.fired && pct <= m.percent) {
        m.fired = true;
        notifyDesktop("Timer message", m.text);
        playMessageSound(m.sound, m.custom);
      }
    }
  }

  function ensureAudio() {
    if (!audioCtx) {
      try {
        audioCtx = new (window.AudioContext ||
          (window as any).webkitAudioContext)();
      } catch {}
    }
  }

  function playBeep(durationMs = 200, frequency = 880) {
    ensureAudio();
    if (!audioCtx) return;
    const osc = audioCtx.createOscillator();
    const gain = audioCtx.createGain();
    osc.type = "sine";
    osc.frequency.value = frequency;
    gain.gain.setValueAtTime(0.15, audioCtx.currentTime);
    osc.connect(gain).connect(audioCtx.destination);
    osc.start();
    gain.gain.exponentialRampToValueAtTime(
      0.0001,
      audioCtx.currentTime + durationMs / 1000
    );
    osc.stop(audioCtx.currentTime + durationMs / 1000 + 0.02);
  }

  function playHeartbeat() {
    ensureAudio();
    if (!audioCtx) return;
    // two short low-frequency beats
    playBeep(80, 150);
    setTimeout(() => playBeep(80, 150), 180);
  }

  function playTripleBeep() {
    playBeep(120, 1200);
    setTimeout(() => playBeep(120, 1200), 200);
    setTimeout(() => playBeep(120, 1200), 400);
  }

  const endAudioCache: Record<string, HTMLAudioElement> = {};
  function playEndSound() {
    if (hasFiredEndSound) return;
    hasFiredEndSound = true;
    if (endSound === "none") return;
    if (endSound === "triple") return playTripleBeep();
    if (endSound === "beep") return playBeep(200, 1000);
    if (endSound === "heartbeat") return playHeartbeat();
    if (endSound === "custom" && endSoundCustom) {
      let el = endAudioCache[endSoundCustom];
      if (!el) {
        el = new Audio(endSoundCustom);
        endAudioCache[endSoundCustom] = el;
      }
      try {
        el.currentTime = 0;
        el.play();
      } catch {}
    }
  }

  function startDangerSound() {
    stopDangerSound();
    if (dangerSound === "none") return;
    if (dangerSound === "beep") {
      dangerSoundTimer = window.setInterval(() => playBeep(150, 880), 1200);
    } else {
      dangerSoundTimer = window.setInterval(() => playHeartbeat(), 1000);
    }
  }

  function stopDangerSound() {
    if (dangerSoundTimer !== null) {
      clearInterval(dangerSoundTimer);
      dangerSoundTimer = null;
    }
  }

  let criticalSoundTimer: number | null = null;
  function startCriticalSound() {
    stopCriticalSound();
    if (criticalSound === "none") return;
    if (criticalSound === "beep")
      criticalSoundTimer = window.setInterval(() => playBeep(120, 1200), 1500);
    else criticalSoundTimer = window.setInterval(() => playHeartbeat(), 900);
  }
  function stopCriticalSound() {
    if (criticalSoundTimer !== null) {
      clearInterval(criticalSoundTimer);
      criticalSoundTimer = null;
    }
  }

  function broadcast(msg: any) {
    if (!bc && typeof window !== "undefined" && "BroadcastChannel" in window) {
      bc = new BroadcastChannel("timer-sync");
    }
    try {
      bc?.postMessage({ source: "main", ...msg });
    } catch {}
  }

  if (typeof window !== "undefined") {
    if ("BroadcastChannel" in window) {
      if (!bc) bc = new BroadcastChannel("timer-sync");
      bc.onmessage = (e) => {
        const data = e.data || {};
        if (data.source === "main") return; // ignore our own rebroadcasts
        if (data.source === "overlay") {
          // apply state from overlay without re-broadcasting
          suppressBroadcast = true;
        }
        if (data.type === "request_state") {
          broadcast({
            type: "state",
            duration: selectedSeconds,
            remaining: remainingSeconds,
            isRunning,
            startAtMs,
            critical: criticalPercent,
            danger: dangerPercent,
            snd: dangerSound,
            csnd: criticalSound,
            colors: { dangerColor, criticalColor },
            messages: userMessages.map(({ percent, text }) => ({
              percent,
              text,
            })),
          });
        } else if (data.type === "close") {
          overlayOpen = false;
        } else if (data.type === 'result') {
          // Overlay recorded a result; just hide our question if shown, don't record again
          if (endRecorded) return; // Already handled locally
          endRecorded = true;
          showEndQuestion = false;
        } else if (data.type === "start") {
          const incomingStart = data.startAtMs ?? Date.now();
          if (lastStartSig && Math.abs(incomingStart - lastStartSig) < 150) {
            suppressBroadcast = false;
            return;
          }
          selectedSeconds = data.duration ?? selectedSeconds;
          runDurationSeconds = data.duration ?? selectedSeconds;
          startAtMs = incomingStart;
          lastStartSig = startAtMs;
          isRunning = true;
          // restart interval loop to compute remaining from startAtMs
          if (intervalId !== null) {
            clearInterval(intervalId);
            intervalId = null;
          }
          intervalId = window.setInterval(() => {
            if (!startAtMs) return;
            const elapsed = Math.floor((Date.now() - startAtMs) / 1000);
            const nextRemaining = Math.max(0, runDurationSeconds - elapsed);
            if (nextRemaining !== remainingSeconds) {
              remainingSeconds = nextRemaining;
              handleTick();
              setTaskbarProgress(
                (runDurationSeconds - remainingSeconds) /
                  Math.max(1, runDurationSeconds)
              );
            }
            if (remainingSeconds === 0) {
              stopDangerSound();
              stopCriticalSound();
              if (!suppressBroadcast) broadcast({ type: "end" });
              handleTimerEnd();
              stop();
            }
          }, 1000);
        } else if (data.type === "pause") {
          if (intervalId !== null) {
            clearInterval(intervalId);
            intervalId = null;
          }
          isRunning = false;
          startAtMs = null;
          if (typeof data.remaining === "number")
            remainingSeconds = data.remaining;
        } else if (data.type === "reset") {
          selectedSeconds = data.duration ?? selectedSeconds;
          remainingSeconds = selectedSeconds;
          prevZone = null;
          userMessages = userMessages.map((m) => ({ ...m, fired: false }));
          startAtMs = null;
          isRunning = false;
        } else if (data.type === "preset") {
          selectedSeconds = data.duration ?? selectedSeconds;
          remainingSeconds = selectedSeconds;
        }
        suppressBroadcast = false;
      };
    }
  }
</script>

<main class="wrap">
  <h1>Timer</h1>

  <div class="presets">
    {#each presets as p, i}
      <div class="preset card">
        <button
          class="preset-btn bordered {selectedSeconds === p ? 'selected' : ''}"
          onclick={() => setPreset(p)}
        >
          <span class="time"
            >{Math.floor(p / 3600)
              ? `${Math.floor(p / 3600)}h `
              : ""}{Math.floor((p % 3600) / 60)}m{p % 60
              ? ` ${p % 60}s`
              : ""}</span
          >
          <span class="hint muted">click to select</span>
        </button>
        <div class="actions">
          <button
            class="icon edit"
            title="Edit"
            onclick={() => openEdit(i)}
            aria-label="Edit preset"
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25z"
                fill="currentColor"
              />
              <path
                d="M20.71 7.04a1.003 1.003 0 0 0 0-1.42l-2.34-2.34a1.003 1.003 0 0 0-1.42 0l-1.83 1.83 3.75 3.75 1.84-1.82z"
                fill="currentColor"
              />
            </svg>
          </button>
          <button
            class="icon remove"
            title="Remove"
            onclick={() => removePresetAt(i)}
            aria-label="Remove preset">×</button
          >
        </div>
      </div>
    {/each}
    <div class="preset add card">
      <button
        class="preset-btn bordered"
        onclick={() => {
          editHours = 0;
          editMinutes = 5;
          editSeconds = 0;
          editOpen = true;
        }}
      >
        + Add preset
      </button>
    </div>
  </div>

  <div class="display">{formatTime(remainingSeconds)}</div>
  {#if showEndQuestion && askOnEnd}
    <div class="end-question">
      <div class="q">{endQuestionText}</div>
      <div class="buttons">
        <button class="yes" onclick={() => answerEnd(true)} title="Yes">✓</button>
        <button class="no" onclick={() => answerEnd(false)} title="No">✖</button>
      </div>
    </div>
  {/if}
  <div class="stats">Today: {wins}/{losses}</div>

  <section class="heatmap-wrap">
    <div class="heatmap-grid">
      <!-- Month labels (row 1, columns start at 2) -->
      {#each buildYearWeeks() as wk, i}
        {#if wk[0] && wk[0].date.getDate() <= 7}
          <div class="hm-month" style={`grid-column:${i+2};`}>{wk[0].date.toLocaleString(undefined,{ month:'short'})}</div>
        {/if}
      {/each}
      <!-- Weekday labels (Mon, Wed, Fri at rows 3,5,7) -->
      <div class="hm-wlabel" style="grid-row:3;">Mon</div>
      <div class="hm-wlabel" style="grid-row:5;">Wed</div>
      <div class="hm-wlabel" style="grid-row:7;">Fri</div>
      <!-- Cells (rows 2..8, columns 2..n) -->
      {#each buildYearWeeks() as wk, i}
        {#each wk as day, j}
          <div class={`hm-cell ${day.key === sameDayKey(new Date()) ? 'today' : ''}`} style={`grid-column:${i+2}; grid-row:${j+2}; background:${colorFor(day.wins, day.losses)};`}
               title={tooltipFor(day.key)}></div>
        {/each}
      {/each}
    </div>
  </section>

  <div class="controls">
    {#if !isRunning}
      <button class="action primary" onclick={start}>▶ Start</button>
    {:else}
      <button class="action secondary" onclick={stop}>Pause</button>
    {/if}
    {#if !isFreshStart()}
      <button class="action ghost" onclick={reset}>Reset</button>
    {/if}
    <button
      class="action ghost {overlayOpen ? 'active' : ''}"
      onclick={openOverlayAndStart}
      title="Toggle overlay"
      >{overlayOpen ? "Overlay mode (on)" : "Overlay mode"}</button
    >
    <button
      class="icon gear tooltip"
      data-tip="Settings"
      onclick={() => {
        requestNotifyPermission();
        editOpen = false;
        settingsOpen = true;
      }}
      aria-label="Settings">⚙</button>
    <button
      class="icon chat {autoMessagesEnabled ? 'on' : ''} tooltip"
      data-tip="Auto-messages"
      onclick={() => {
        autoMessagesEnabled = !autoMessagesEnabled;
        requestNotifyPermission();
      }}
      aria-label="Auto messages">💬</button>
  </div>

  <Modal
    open={editOpen}
    title="Edit time"
    subtitle="Adjust hours, minutes and seconds"
    onclose={closeEdit}
  >
    <div class="grid selects">
      <label class="row">
        <span class="muted" style="width:90px">Hours</span>
        <select bind:value={editHours}>
          {#each Array.from({ length: 13 }, (_, n) => n) as h}
            <option value={h}>{h}</option>
          {/each}
        </select>
      </label>
      <label class="row">
        <span class="muted" style="width:90px">Minutes</span>
        <select bind:value={editMinutes}>
          {#each Array.from({ length: 60 }, (_, n) => n) as m}
            <option value={m}>{m}</option>
          {/each}
        </select>
      </label>
      <label class="row">
        <span class="muted" style="width:90px">Seconds</span>
        <select bind:value={editSeconds}>
          {#each Array.from({ length: 60 }, (_, n) => n) as s}
            <option value={s}>{s}</option>
          {/each}
        </select>
      </label>
    </div>
    <div class="row modal-actions" style="justify-content:flex-end; margin-top: 12px; gap:8px;">
      {#if editingIndex !== null}
        <button class="action ghost" onclick={closeEdit}>Cancel</button>
        <button class="action primary" onclick={saveEdit}>Save</button>
      {:else}
        <button class="action ghost" onclick={closeEdit}>Cancel</button>
        <button class="action primary" onclick={addFromEdit}>Add</button>
      {/if}
    </div>
  </Modal>

  {#if settingsOpen}
    <Modal
      open={settingsOpen}
      title="Timer settings"
      subtitle="Configure zones, messages and sounds"
      onclose={() => (settingsOpen = false)}
    >
      <div class="grid">
        <label class="row">
          <span class="muted" style="width:180px">Critical threshold (%)</span>
          <input type="range" min="1" max="99" bind:value={criticalPercent} />
          <span>{criticalPercent}%</span>
        </label>
        <label class="row">
          <span class="muted" style="width:180px">Danger threshold (%)</span>
          <input type="range" min="1" max="99" bind:value={dangerPercent} />
          <span>{dangerPercent}%</span>
        </label>
        <div class="muted" style="font-size:12px">
          Zone logic: Danger ≤ {dangerPercent}%, Critical between {dangerPercent}% and {criticalPercent}%, OK ≥ {criticalPercent}%.
        </div>

        <div style="margin-top:12px; font-weight:600;">Messages (trigger when remaining ≤ %)</div>
        {#each userMessages as msg, idx}
          <div class="row" style="gap:8px; align-items:center;">
            <input
              type="number"
              min="1"
              max="99"
              bind:value={msg.percent}
              style="width:80px"
            />
            <input
              type="text"
              bind:value={msg.text}
              placeholder="Message to show"
              style="flex:1"
            />
            <select bind:value={msg.sound}>
              <option value="beep">Beep</option>
              <option value="heartbeat">Heartbeat</option>
              <option value="none">None</option>
              <option value="custom">Custom</option>
            </select>
            {#if msg.sound === "custom"}
              <input
                type="file"
                accept="audio/*"
                onchange={async (e) => {
                  const file = (e.currentTarget as HTMLInputElement).files?.[0];
                  if (!file) return;
                  const reader = new FileReader();
                  reader.onload = () => { msg.custom = String(reader.result || ""); };
                  reader.readAsDataURL(file);
                }}
              />
            {/if}
            <button
              class="icon remove"
              title="Remove"
              onclick={() => (userMessages = userMessages.filter((_, i) => i !== idx))}>×</button>
          </div>
        {/each}
        <div class="row">
          <button
            class="icon"
            onclick={() => (userMessages = [...userMessages, { percent: 30, text: "Keep going!", sound: "beep" }])}
            title="Add message">＋</button>
        </div>

        <div style="margin-top:12px; font-weight:600;">End question</div>
        <label class="row" style="gap:8px">
          <input type="checkbox" bind:checked={askOnEnd} onchange={persistEndQuestionSettings} />
          <span class="muted">Ask when timer ends</span>
        </label>
        <label class="row">
          <span class="muted" style="width:180px">Question text</span>
          <input type="text" bind:value={endQuestionText} onblur={persistEndQuestionSettings} style="flex:1" />
        </label>

        <div style="margin-top:12px; font-weight:600;">Zone colors</div>
        <div class="row">
          <span class="muted" style="width:180px">Danger color</span>
          <input type="color" bind:value={dangerColor} />
        </div>
        <div class="row">
          <span class="muted" style="width:180px">Critical color</span>
          <input type="color" bind:value={criticalColor} />
        </div>
      </div>
      <div class="row" style="justify-content:flex-end; margin-top: 12px; gap:8px;">
        <button class="action ghost" onclick={() => { resetAll(); settingsOpen = false; }} title="Reset all data">Reset all</button>
        <button class="action" onclick={() => (settingsOpen = false)}>Close</button>
      </div>
    </Modal>
  {/if}
</main>

<style>
  .wrap {
    max-width: 640px;
    margin: 0 auto;
    padding: 32px 16px;
  display: flex;
  flex-direction: column;
    gap: 16px;
  }

  .presets {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 16px;
  }
  .preset {
    position: relative;
    padding: 8px;
  }
  .preset-btn {
    width: 100%;
    padding: 16px;
    text-align: left;
    background: transparent;
    color: var(--text);
    position: relative;
    z-index: 1;
  }
  .preset-btn.selected {
    border-color: var(--primary);
  }
  .preset .actions {
    position: absolute;
    top: 8px;
    right: 8px;
    display: inline-flex;
    gap: 6px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.15s ease;
    z-index: 2;
  }
  .preset:hover .actions,
  .preset:focus-within .actions {
    opacity: 1;
    pointer-events: auto;
  }
  .icon {
    background: rgba(0, 0, 0, 0.35);
    color: var(--text);
    border: 1px solid var(--card-border);
    border-radius: 8px;
    padding: 4px 8px;
    cursor: pointer;
  }
  .icon.edit:hover {
    color: var(--text);
  }
  .icon.remove:hover {
    color: #ff6b6b;
  }
  .time {
    font-size: 18px;
    font-weight: 600;
  }
  .hint {
    font-size: 12px;
  }

  .display {
    font-size: 64px;
  text-align: center;
    letter-spacing: 2px;
  }
  .stats { text-align: center; color: var(--muted); margin-top: -8px; }

  .controls {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
  .selects select {
    background: #0f0f0f;
    color: var(--text);
    border: 1px solid var(--card-border);
  border-radius: 8px;
    padding: 8px;
  }
  .icon {
    background: rgba(0, 0, 0, 0.35);
    color: var(--text);
    border: 1px solid var(--card-border);
    border-radius: 8px;
    padding: 6px 10px;
  cursor: pointer;
}
  .icon.gear {
  }
  .icon.chat.on {
    border-color: var(--primary);
  }
  .action {
    border-radius: 10px;
    border: 1px solid var(--card-border);
    padding: 0.6em 1em;
    font-weight: 600;
    color: var(--text);
    background: var(--card);
    box-shadow: var(--shadow);
    cursor: pointer;
  }
  .action.primary {
    background: var(--primary);
    color: #05210c;
    border-color: #72d480;
  }
  .action.secondary {
    background: #1b1b1b;
  }
  .action.ghost {
    background: transparent;
  }
  .action.active {
    border-color: var(--primary);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--primary) 35%, transparent);
  }
  .icon.gear,
  .icon.chat {
    font-size: 20px;
    line-height: 1;
  }
  .tooltip {
    position: relative;
  }
  .tooltip::after {
    content: attr(data-tip);
    position: absolute;
    top: -30px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(0, 0, 0, 0.8);
    color: #fff;
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 12px;
    white-space: nowrap;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.15s ease;
  }
  .tooltip:hover::after {
    opacity: 1;
  }

  .end-question { display:flex; flex-direction:column; align-items:center; gap:8px; margin-top:8px; }
  .end-question .q { font-weight:600; }
  .end-question .buttons { display:flex; gap:12px; }
  .end-question .buttons .yes, .end-question .buttons .no { width:36px; height:36px; border-radius:8px; border:1px solid var(--card-border); background:#111; color:#fff; }
  .end-question .buttons .yes { background: rgba(0,255,128,.20); }
  .end-question .buttons .no { background: rgba(255,64,64,.20); }

  /* heatmap styles (hard-scoped) */
  .heatmap-wrap .heatmap-grid {
    display:grid;
    grid-template-columns: 24px repeat(60, 12px);
    grid-template-rows: 16px repeat(7, 12px);
    gap: 2px;
    align-items: center;
    margin-top: 12px;
  }
  .heatmap-wrap .hm-month { grid-row:1; font-size: 10px; color: var(--muted); }
  .heatmap-wrap .hm-wlabel { grid-column:1; font-size: 10px; color: var(--muted); justify-self: end; }
  .heatmap-wrap .hm-cell { width:12px; height:12px; border-radius:2px; }
  .heatmap-wrap .hm-cell.today { outline: 1px solid var(--primary); outline-offset: 1px; }

  @media (prefers-color-scheme: dark) {
  }
</style>