<script lang="ts">
  import { ProgressBarStatus } from "@tauri-apps/api/window";
  import Button from "../lib/components/Button.svelte";
  import Modal from "../lib/components/Modal.svelte";
  import Input from "../lib/components/Input.svelte";
  import Checkbox from "../lib/components/Checkbox.svelte";
  import Select from "../lib/components/Select.svelte";
  import Icon from "../lib/components/Icon.svelte";
  // Check if we're in dev mode
  const isDev = typeof import.meta !== 'undefined' && import.meta.env?.DEV === true;
  
  // Add 5-second preset in dev mode
  let presets = $state<number[]>(isDev ? [5, 120, 600, 1200] : [120, 600, 1200]);
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
  
  // Periodically check overlay state to keep it in sync
  $effect(() => {
    const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
    if (!isTauri) return;
    
    const checkOverlayState = async () => {
      try {
        const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
        const overlay = await WebviewWindow.getByLabel("overlay");
        if (overlay) {
          try {
            const isVisible = await overlay.isVisible();
            overlayOpen = isVisible;
          } catch {
            overlayOpen = false;
          }
        } else {
          overlayOpen = false;
        }
      } catch {
        overlayOpen = false;
      }
    };
    
    // Check immediately
    checkOverlayState();
    
    // Check periodically (every 2 seconds)
    const interval = setInterval(checkOverlayState, 2000);
    
    return () => clearInterval(interval);
  });
  // End question settings
  let askOnEnd = $state(true);
  let endQuestionText = $state('Win?');
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
      const z = localStorage.getItem('zoneSoundSettings');
      if (z) {
        const v = JSON.parse(z);
        if (['none','danger','critical','all'].includes(v.zoneSoundMode)) zoneSoundMode = v.zoneSoundMode;
        if (['none','danger','critical','all'].includes(v.zoneNotifyMode)) zoneNotifyMode = v.zoneNotifyMode;
      }
    } catch {}
  }
  function saveStats() {
    // Derived from records; nothing to do
  }
  function persistEndQuestionSettings() {
    try { localStorage.setItem('endQuestionSettings', JSON.stringify({ askOnEnd, endQuestionText })); } catch {}
  }
  function persistZoneSoundSettings() {
    try {
      localStorage.setItem('zoneSoundSettings', JSON.stringify({zoneSoundMode, zoneNotifyMode}));
    } catch {}
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
    // Detect theme: direct body class if present
    let isLight = false;
    if (typeof document !== 'undefined') {
      isLight = document.body.classList.contains('theme-light');
    }
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
    const recs = loadRecords();
    recs.push({ ts: Date.now(), result: win ? 'win' : 'loss' });
    saveRecords(recs);
    recomputeToday(); // Immediately updates heatmap
    heatmapKey += 1; // Force heatmap re-render
    showEndQuestion = false;
    // Let overlay hide its question too
    try { broadcast({ type: 'result', ts: Date.now(), result: win ? 'win' : 'loss' }); } catch {}
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
        await win.setProgressBar({ progress: p, status: ProgressBarStatus.Normal });
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
  let allMuted = $state(false); // global mute toggle
  let zoneSoundMode: 'none' | 'danger' | 'critical' | 'all' = $state('none'); // Default OFF
  let zoneNotifyMode: 'none' | 'danger' | 'critical' | 'all' = $state('none');
  let zoneNotifFired = new Set<string>(); // tracks zone notifications for this run
  let messageFireTimestamps: Record<number, number> = {};
  let heatmapKey = $state(0);

  // Licensing system (backend-controlled)
  let isLicensed = $state(false);
  let licenseModalOpen = $state(false);
  let licenseKeyInput = $state("");
  let licenseError = $state("");
  let helpMenuOpen = $state(false);

  async function checkLicense(): Promise<boolean> {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<boolean>("check_license");
      isLicensed = result;
      return result;
    } catch (error) {
      console.error("Failed to check license:", error);
      isLicensed = false;
      return false;
    }
  }

  async function activateLicense() {
    licenseError = "";
    const key = licenseKeyInput.trim();
    
    if (!key) {
      licenseError = "Please enter a license key";
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<boolean>("activate_license", { key });
      
      if (result) {
        isLicensed = true;
        licenseModalOpen = false;
        licenseKeyInput = "";
        helpMenuOpen = false;
      } else {
        licenseError = "Failed to activate license";
      }
    } catch (error: any) {
      // Backend returns error as string in Tauri
      licenseError = typeof error === 'string' ? error : error?.message || "Failed to activate license";
    }
  }

  function openChangeLicense() {
    licenseKeyInput = "";
    licenseError = "";
    licenseModalOpen = true;
    helpMenuOpen = false;
  }

  async function removeLicense() {
    if (!confirm('Are you sure you want to remove the license key? The app will require activation on next launch.')) {
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<boolean>("remove_license");
      
      if (result) {
        isLicensed = false;
        licenseModalOpen = true;
        helpMenuOpen = false;
        licenseKeyInput = "";
        licenseError = "";
      }
    } catch (error: any) {
      console.error("Failed to remove license:", error);
      alert("Failed to remove license. Please try again.");
    }
  }

  // Check license on mount
  $effect(() => {
    (async () => {
      const licensed = await checkLicense();
      if (!licensed) {
        licenseModalOpen = true;
      }
    })();
  });

  function resetZoneNotifFired() {
    zoneNotifFired = new Set<string>();
  }

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
    if (!isLicensed) return;
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
      autoMessageThisTickFired = false;
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
        // Ensure audio context is resumed before end sound
        try {
          // lazily create or resume context
          if (!audioCtx) {
            // @ts-ignore
            const Ctx = (window.AudioContext || (window as any).webkitAudioContext);
            if (Ctx) audioCtx = new Ctx();
          }
          audioCtx?.resume?.();
        } catch {}
        playEndSound();
        stopDangerSound();
        stopCriticalSound();
        const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
        if (overlayPopupOnEnd && isTauri) {
          (async () => {
            try {
              // Request user attention (dock/taskbar bounce)
              try {
                const appMod: any = await import("@tauri-apps/api/app");
                await appMod.getCurrent?.().requestUserAttention?.('Critical');
              } catch {}
              const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
              let overlay = await WebviewWindow.getByLabel("overlay");
              if (!overlay) {
                // Create overlay if it doesn't exist
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
                            userMessages.map(({ percent, text, sound, custom }) => ({ percent, text, sound, custom }))
                          )
                        )
                      )
                    )
                  ),
                  snd: dangerSound,
                  csnd: criticalSound,
                  startAt: String(Math.floor(Date.now())),
                  dangerColor,
                  criticalColor,
                  ask: askOnEnd ? '1' : '0',
                  q: encodeURIComponent(endQuestionText),
                  alwaysOnTop: overlayAlwaysOnTop ? '1' : '0',
                  popupOnEnd: '1',
                  overlayMode: overlayMode,
                  progressBarColor: overlayProgressBarColor,
                  progressBarFinishedColor: overlayProgressBarFinishedColor,
                  progressBarType: overlayProgressBarType,
                  overlayBackgroundImage: overlayBackgroundImage ? encodeURIComponent(overlayBackgroundImage) : '',
                });
                const w = new WebviewWindow("overlay", {
                  url: `/overlay?${params.toString()}`,
                  title: "Timer Overlay",
                  width: 260,
                  height: 140,
                  resizable: true,
                  decorations: true,
                  alwaysOnTop: overlayAlwaysOnTop,
                  focus: true,
                  visible: true,
                  shadow: true,
                  skipTaskbar: false,
                  x: 0,
                  y: 0,
                });
                await new Promise<void>((resolve, reject) => {
                  w.once("tauri://created", () => resolve());
                  w.once("tauri://error", (e) => reject(e));
                  setTimeout(() => reject(new Error("Timeout")), 5000);
                });
                overlay = w;
                overlayOpen = true;
              }
              // Always bring to front when timer ends (if overlay exists)
              if (overlay) {
              try { await overlay.unminimize?.(); } catch {}
              try { await overlay.setVisibleOnAllWorkspaces?.(true); } catch {}
                try { await overlay.setAlwaysOnTop(overlayAlwaysOnTop || true); } catch {}
              try { await overlay.show(); } catch {}
                try { await overlay.setFocus(); } catch {}
              }
              // Also broadcast as fallback
              try { broadcast({ type: 'overlay_popup' }); } catch {}
            } catch (e) {
              console.error("Failed to popup overlay on timer end:", e);
            }
          })();
        }
        handleTimerEnd();
        broadcast({ type: "end" });
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
        messages: userMessages.map(({ percent, text, fired, sound, custom }) => ({ percent, text, fired, sound, custom })),
      });
  }

  function stop() {
    if (!isLicensed) return;
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
    if (!isLicensed) return;
    stop();
    remainingSeconds = selectedSeconds;
    prevZone = null;
    userMessages = userMessages.map((m) => ({ ...m, fired: false }));
    startAtMs = null;
    hasFiredEndSound = false;
    endRecorded = false;
    setTaskbarProgress(undefined);
    resetZoneNotifFired();
    messageFireTimestamps = {};
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
    
    // First, check if overlay exists and sync state
    if (isTauri) {
      try {
        const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
        const existing = await WebviewWindow.getByLabel("overlay");
        if (existing) {
          try {
            const isVisible = await existing.isVisible();
            if (isVisible) {
              // Overlay exists and is visible, close it
              await existing.close();
              overlayOpen = false;
              return;
            } else {
              // Overlay exists but not visible, mark as closed
              overlayOpen = false;
            }
          } catch {
            overlayOpen = false;
          }
        } else {
          overlayOpen = false;
        }
      } catch {
        overlayOpen = false;
      }
    }
    
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
      q: encodeURIComponent(endQuestionText),
      alwaysOnTop: overlayAlwaysOnTop ? '1' : '0',
      popupOnEnd: overlayPopupOnEnd ? '1' : '0',
      overlayMode: overlayMode,
      progressBarColor: overlayProgressBarColor,
      progressBarFinishedColor: overlayProgressBarFinishedColor,
      progressBarType: overlayProgressBarType,
      overlayBackgroundImage: overlayBackgroundImage ? encodeURIComponent(overlayBackgroundImage) : '',
    });
    if (!isTauri) {
      location.href = `/overlay?${params.toString()}`;
      return;
    }
    // State sync already happened above, so if we get here and overlayOpen is false,
    // we know the overlay should be closed and we can proceed to create it
    const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const w = new WebviewWindow("overlay", {
      url: `/overlay?${params.toString()}`,
      title: "Timer Overlay",
      width: 260,
      height: 140,
      resizable: true,
      decorations: true,
      alwaysOnTop: overlayAlwaysOnTop,
      focus: true,
      visible: true,
      shadow: true,
      skipTaskbar: false,
      x: 0,
      y: 0,
    });
    try {
      await new Promise<void>((resolve, reject) => {
        w.once("tauri://created", () => resolve());
        w.once("tauri://error", (e) => reject(e));
        setTimeout(() => reject(new Error("Timeout")), 5000);
      });
      await w.setAlwaysOnTop(overlayAlwaysOnTop);
      await w.setFocus();
      await w.show();
      // @ts-ignore center may not be typed
      await w.center?.();
      // Set state immediately after window is shown
      overlayOpen = true;
      // Verify state by checking window visibility
      try {
        const isVisible = await w.isVisible();
        overlayOpen = isVisible;
      } catch {
        overlayOpen = true; // Default to true if check fails
      }
      // Track close to update state
      w.listen("tauri://destroyed", (_e: any) => {
        overlayOpen = false;
      });
      // Note: close-requested might not fire, so we also listen for destroyed
    } catch (e) {
      console.error("Failed to show overlay window", e);
      overlayOpen = false;
    }
  }

  async function openPictureInPicture(totalSeconds: number) {
    // Create a canvas we can stream into PiP
    const width = 360,
      height = 100;
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
    if (allMuted) return;
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

  let autoMessageThisTickFired = false;
  // Top-level:
  let lastTickTimestamp = 0;
  // Track messages that have been fired this tick to prevent duplicates within same tick
  let messagesFiredThisTick = new Set<number>();

  function handleTick() {
    // Only allow one tick event per second globally (per window)
    const now = Date.now();
    if (Math.abs(now - lastTickTimestamp) < 950) return;
    lastTickTimestamp = now;
    messagesFiredThisTick.clear(); // Reset per-tick tracking
    if (!autoMessagesEnabled) return;
    const pct = currentPercent();
    const zone = zoneFromPercent(pct);
    if (zone !== prevZone) {
      prevZone = zone;
      if (zone === 'DANGER' || zone === 'CRITICAL') {
        // Notification
        const notifyMap = {'DANGER': ['danger','all'], 'CRITICAL': ['critical','all']};
        if (zoneNotifyMode && notifyMap[zone]?.includes(zoneNotifyMode) && !zoneNotifFired.has(zone)) {
          notifyDesktop('Timer', `Entered ${zone} zone`);
          zoneNotifFired.add(zone);
        }
        // Sound
        const soundMap = {'DANGER': ['danger','all'], 'CRITICAL': ['critical','all']};
        if (zoneSoundMode && soundMap[zone]?.includes(zoneSoundMode)) {
          if (zone === 'DANGER') { startDangerSound(); stopCriticalSound(); }
          else if (zone === 'CRITICAL') { startCriticalSound(); stopDangerSound(); }
        } else {
          stopDangerSound();
          stopCriticalSound();
        }
      } else {
        // OK zone, turn off both
        stopDangerSound();
        stopCriticalSound();
      }
    }
    for (let i = 0; i < userMessages.length; ++i) {
      const m = userMessages[i];
      // Skip if already fired or fired this tick
      if (m.fired || messagesFiredThisTick.has(i)) continue;
      if (pct <= m.percent) {
        // Cooldown: do not fire if fired in last 2s
        if (messageFireTimestamps[i] && now - messageFireTimestamps[i] < 2000) continue;
        messageFireTimestamps[i] = now;
        messagesFiredThisTick.add(i); // Mark as fired this tick
        // Set fired immediately BEFORE broadcasting to prevent race conditions
        m.fired = true;
        // Broadcast BEFORE notification to minimize race condition window
        if (bc && !suppressBroadcast) {
          try {
            bc.postMessage({
              source: "main",
              type: "message_fired",
              messageIndex: i,
              timestamp: now
            });
          } catch {}
        }
        // Use queueMicrotask to ensure broadcast propagates before notification
        queueMicrotask(() => {
        notifyDesktop("Timer message", m.text);
        playMessageSound(m.sound, m.custom);
        });
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
    if (allMuted) return; // Respect mute
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
    if (allMuted) return;
    ensureAudio();
    if (!audioCtx) return;
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
    if (zoneSoundMode !== 'danger' && zoneSoundMode !== 'all') return; // disables in other modes
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
    if (zoneSoundMode !== 'critical' && zoneSoundMode !== 'all') return; // disables in other modes
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

  let overlayAlwaysOnTop = $state(true); // user option for overlay always-on-top
  let overlayPopupOnEnd = $state(false); // user option for popup on timer end
  let overlayMode = $state("normal"); // "normal" | "progressbar" | ...
  let overlayProgressBarColor = $state("#8ef59b");
  let overlayProgressBarFinishedColor = $state("#ff0000"); // Color when timer ends
  let overlayProgressBarType = $state("full");
  let appBackgroundImage = $state<string | null>(null);
  let overlayBackgroundImage = $state<string | null>(null);
  function persistOverlaySettings() {
    try {
      localStorage.setItem('overlaySettings', JSON.stringify({ overlayAlwaysOnTop, overlayPopupOnEnd, overlayMode, overlayProgressBarColor, overlayProgressBarFinishedColor, overlayProgressBarType }));
      if (bc) bc.postMessage({source:'main', type:'overlay_settings', alwaysOnTop: overlayAlwaysOnTop, overlayMode: overlayMode, progressBarColor: overlayProgressBarColor, progressBarFinishedColor: overlayProgressBarFinishedColor, progressBarType: overlayProgressBarType});
    } catch {}
  }
  function loadOverlaySettings() {
    try {
      const s = localStorage.getItem('overlaySettings');
      if (s) {
        const v = JSON.parse(s);
        if (typeof v.overlayAlwaysOnTop === 'boolean') overlayAlwaysOnTop = v.overlayAlwaysOnTop;
        if (typeof v.overlayPopupOnEnd === 'boolean') overlayPopupOnEnd = v.overlayPopupOnEnd;
        if (typeof v.overlayMode === 'string') overlayMode = v.overlayMode;
        if (typeof v.overlayProgressBarColor === 'string') overlayProgressBarColor = v.overlayProgressBarColor;
        if (typeof v.overlayProgressBarFinishedColor === 'string') overlayProgressBarFinishedColor = v.overlayProgressBarFinishedColor;
        if (typeof v.overlayProgressBarType === 'string') overlayProgressBarType = v.overlayProgressBarType;
      }
    } catch {}
  }
  loadOverlaySettings();

  function saveAppSettings() {
    try {
      const state = {
        dangerSound,
        criticalSound,
        endSound,
        endSoundCustom,
        autoMessagesEnabled,
        criticalPercent,
        dangerPercent,
        userMessages,
        askOnEnd,
        endQuestionText,
        presets,
        dangerColor,
        criticalColor,
        zoneSoundMode,
        zoneNotifyMode,
        overlayAlwaysOnTop,
        overlayPopupOnEnd,
        appBackgroundImage,
        overlayBackgroundImage,
      };
      localStorage.setItem('appSettings', JSON.stringify(state));
    } catch {}
  }
  function loadAppSettings() {
    try {
      const s = localStorage.getItem('appSettings');
      if (s) {
        const v = JSON.parse(s);
        if (typeof v.dangerSound === 'string') dangerSound = v.dangerSound;
        if (typeof v.criticalSound === 'string') criticalSound = v.criticalSound;
        if (typeof v.endSound === 'string') endSound = v.endSound;
        if (typeof v.endSoundCustom === 'string') endSoundCustom = v.endSoundCustom;
        if (typeof v.autoMessagesEnabled === 'boolean') autoMessagesEnabled = v.autoMessagesEnabled;
        if (typeof v.criticalPercent === 'number') criticalPercent = v.criticalPercent;
        if (typeof v.dangerPercent === 'number') dangerPercent = v.dangerPercent;
        if (Array.isArray(v.userMessages)) userMessages = v.userMessages;
        if (typeof v.askOnEnd === 'boolean') askOnEnd = v.askOnEnd;
        if (typeof v.endQuestionText === 'string') endQuestionText = v.endQuestionText;
        if (Array.isArray(v.presets)) {
          presets = v.presets;
          // In dev mode, ensure 5-second preset is included
          if (isDev && !presets.includes(5)) {
            presets = [5, ...presets].sort((a, b) => a - b);
          }
        }
        if (typeof v.dangerColor === 'string') dangerColor = v.dangerColor;
        if (typeof v.criticalColor === 'string') criticalColor = v.criticalColor;
        if (typeof v.zoneSoundMode === 'string') zoneSoundMode = v.zoneSoundMode;
        if (typeof v.zoneNotifyMode === 'string') zoneNotifyMode = v.zoneNotifyMode;
        if (typeof v.overlayAlwaysOnTop === 'boolean') overlayAlwaysOnTop = v.overlayAlwaysOnTop;
        if (typeof v.overlayPopupOnEnd === 'boolean') overlayPopupOnEnd = v.overlayPopupOnEnd;
        if (typeof v.appBackgroundImage === 'string') appBackgroundImage = v.appBackgroundImage;
        if (typeof v.overlayBackgroundImage === 'string') overlayBackgroundImage = v.overlayBackgroundImage;
      }
    } catch { }
  }
  // Call loadAppSettings() immediately on startup
  loadAppSettings();
  // For each setting user may change, call saveAppSettings() (e.g. after each setting is toggled/input)
  // effect(() => { saveAppSettings(dangerSound); }); // Removed
  // effect(() => { saveAppSettings(criticalSound); }); // Removed
  // effect(() => { saveAppSettings(endSound); }); // Removed
  // effect(() => { saveAppSettings(endSoundCustom); }); // Removed
  // effect(() => { saveAppSettings(autoMessagesEnabled); }); // Removed
  // effect(() => { saveAppSettings(criticalPercent); }); // Removed
  // effect(() => { saveAppSettings(dangerPercent); }); // Removed
  // effect(() => { saveAppSettings(userMessages); }); // Removed
  // effect(() => { saveAppSettings(askOnEnd); }); // Removed
  // effect(() => { saveAppSettings(endQuestionText); }); // Removed
  // effect(() => { saveAppSettings(presets); }); // Removed
  // effect(() => { saveAppSettings(dangerColor); }); // Removed
  // effect(() => { saveAppSettings(criticalColor); }); // Removed
  // effect(() => { saveAppSettings(zoneSoundMode); }); // Removed
  // effect(() => { saveAppSettings(zoneNotifyMode); }); // Removed
  // effect(() => { saveAppSettings(overlayAlwaysOnTop); }); // Removed
  // effect(() => { saveAppSettings(overlayPopupOnEnd); }); // Removed

  // Fix for buildMonthLabels to display each month only at the real transition week
  function buildMonthLabels(weeks: Array<Array<{ date: Date; key: string; wins: number; losses: number }>>): Array<{text:string, col:number}> {
    const labels = [];
    for (let i = 0; i < weeks.length; ++i) {
      const week = weeks[i];
      // Only print a label if this week contains the 1st (first day) of the month
      if (week.find(d => d.date.getDate() === 1)) {
        labels.push({text: week.find(d => d.date.getDate() === 1)!.date.toLocaleString(undefined, {month: 'short'}), col: i+2});
      }
      // Also print on very first week in the grid
      if (i === 0 && week[0]) {
        labels.push({text: week[0].date.toLocaleString(undefined, {month: 'short'}), col: i+2});
      }
    }
    // Remove duplicates based on month name and column position
    return labels.filter((v, idx, arr) => arr.findIndex(l => l.text === v.text && l.col === v.col) === idx);
  }
  const weekDays = ['Sun','Mon','Tue','Wed','Thu','Fri','Sat'];

  let colorTheme = $state(localStorage.getItem('colorTheme') || 'dark');
  $effect(() => {
    document.body.classList.toggle('theme-dark', colorTheme === 'dark');
    document.body.classList.toggle('theme-light', colorTheme === 'light');
    if (bc) bc.postMessage({ type: 'theme', theme: colorTheme });
  });
  function setTheme(t: string) {
    colorTheme = t;
    localStorage.setItem('colorTheme', t);
    if (bc) bc.postMessage({ type: 'theme', theme: colorTheme });
  }

  if (typeof window !== "undefined") {
    (async () => {
      const isTauri = (window as any).__TAURI_INTERNALS__;
      if (!isTauri) return;
      try {
        const { getCurrentWebviewWindow, WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
        const current = getCurrentWebviewWindow();
        await current.onCloseRequested(async (event) => {
          // Ask if timer is currently running
          if (isRunning) {
            const ok = typeof window !== 'undefined' ? window.confirm('The timer is running. Quit and stop the timer?') : true;
            if (!ok) {
              event.preventDefault();
              return;
            }
          }
          // Close overlay if present before the app exits
          try {
            const overlay = await WebviewWindow.getByLabel("overlay");
            await overlay?.close();
          } catch {}
        });
      } catch {}
    })();
    if ("BroadcastChannel" in window) {
      if (!bc) bc = new BroadcastChannel("timer-sync");
      bc.onmessage = async (e) => {
        const data = e.data || {};
        if (data.source === "main") return; // ignore our own rebroadcasts
        if (data.source === "overlay") {
          // apply state from overlay without re-broadcasting
          suppressBroadcast = true;
          // Handle overlay_opened message to immediately set state to true
          if (data.type === "overlay_opened") {
            overlayOpen = true;
          }
          // Sync overlayOpen state when receiving messages from overlay
          const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
          if (isTauri) {
            try {
              const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
              const overlay = await WebviewWindow.getByLabel("overlay");
              if (overlay) {
                try {
                  const isVisible = await overlay.isVisible();
                  overlayOpen = isVisible;
                } catch {
                  // If overlay exists but we can't check visibility, assume it's open
                  if (data.type === "overlay_opened") {
                    overlayOpen = true;
                  }
                }
              } else {
                overlayOpen = false;
              }
            } catch {
              // Keep current state if check fails
            }
          }
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
            messages: userMessages.map(({ percent, text, fired }) => ({
              percent,
              text,
              fired,
            })),
            askOnEnd,
            endQuestionText,
          });
        } else if (data.type === "close") {
          overlayOpen = false;
          // Verify the overlay is actually closed
          const isTauriCheck = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
          if (isTauriCheck) {
            (async () => {
              try {
                const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
                const overlay = await WebviewWindow.getByLabel("overlay");
                if (overlay) {
                  try {
                    const isVisible = await overlay.isVisible();
                    if (!isVisible) {
                      overlayOpen = false;
                    }
                  } catch {
                    overlayOpen = false;
                  }
                } else {
                  overlayOpen = false;
                }
              } catch {
                overlayOpen = false;
              }
            })();
          }
        } else if (data.type === 'result') {
          // Overlay recorded a result; persist and update today stats
          const recs = loadRecords();
          const result = data.result === 'win' ? 'win' : 'loss';
          const when = typeof data.ts === 'number' ? data.ts : Date.now();
          recs.push({ ts: when, result });
          saveRecords(recs);
          recomputeToday();
          heatmapKey += 1; // Force heatmap re-render on main page
          showEndQuestion = false;
          if (overlayPopupOnEnd) {
            const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
            if (isTauri) {
              (async () => {
                const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
                const overlay = await WebviewWindow.getByLabel("overlay");
                if (overlay) {
                  await overlay.unminimize?.();
                  await overlay.setVisibleOnAllWorkspaces?.(true);
                  await overlay.show();
                  await overlay.setAlwaysOnTop(true); // force top if option enabled
                  await overlay.setFocus();
                }
              })();
            }
          }
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
            autoMessageThisTickFired = false;
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
              // Popup overlay when timer ends (if enabled) - sync from overlay case
              const isTauriSync = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
              if (overlayPopupOnEnd && isTauriSync) {
                (async () => {
                  try {
                    const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
                    const overlay = await WebviewWindow.getByLabel("overlay");
                    if (overlay) {
                      try { await overlay.unminimize?.(); } catch {}
                      try { await overlay.setVisibleOnAllWorkspaces?.(true); } catch {}
                      try { await overlay.setAlwaysOnTop(overlayAlwaysOnTop || true); } catch {}
                      try { await overlay.show(); } catch {}
                      try { await overlay.setFocus(); } catch {}
                    }
                  } catch (e) {
                    console.error("Failed to popup overlay on timer end (sync):", e);
                  }
                })();
              }
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
        } else if (data.type === "overlay_settings") {
          overlayAlwaysOnTop = data.alwaysOnTop === 'true';
          persistOverlaySettings();
        } else if (data.type === "overlay_popup") {
          if (overlayPopupOnEnd) {
            const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
            if (isTauri) {
              (async () => {
                const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
                const overlay = await WebviewWindow.getByLabel("overlay");
                if (overlay) {
                  await overlay.unminimize?.();
                  await overlay.setVisibleOnAllWorkspaces?.(true);
                  await overlay.show();
                  await overlay.setAlwaysOnTop(true); // force top if option enabled
                  await overlay.setFocus();
                }
              })();
            }
          }
        }
        suppressBroadcast = false;
      };
    }
  }
</script>

<main class="max-w-[850px] mx-auto px-4 py-8 flex flex-col gap-4 min-h-screen" style={appBackgroundImage ? `background-image: url(${appBackgroundImage}); background-size: cover; background-position: center; background-repeat: no-repeat; background-attachment: fixed;` : ''}>
  <!-- Branding -->
  <div class="flex justify-between items-center mb-2">
    <div class="text-sm [color:var(--color-muted)]">
      <span class="font-semibold">Timer Gamified</span>
      <span class="ml-2 text-xs">Beta v0.1.0</span>
    </div>
  </div>
  {#if !isLicensed || licenseModalOpen}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <Modal open={licenseModalOpen} onclose={() => { if (isLicensed) { licenseModalOpen = false; } }}>
        <div class="p-6 flex flex-col gap-4">
          <h2 class="text-2xl font-bold [color:var(--color-foreground)]">License Activation Required</h2>
          <p class="[color:var(--color-muted)]">
            Timer Gamified Beta requires a valid license key to use. Please enter your license key to continue.
          </p>
          <div class="flex flex-col gap-2">
            <label class="text-sm font-semibold [color:var(--color-foreground)]" for="license-input">License Key</label>
            <Input
              id="license-input"
              type="text"
              bind:value={licenseKeyInput}
              placeholder="XXXXX-XXXXX-XXXXX-XXXXX-XXXXX"
              class="uppercase"
              onkeydown={(e) => {
                if (e.key === 'Enter') activateLicense();
              }}
              autofocus={true}
            />
            {#if licenseError}
              <p class="text-sm text-red-500">{licenseError}</p>
            {/if}
          </div>
          <div class="flex gap-2 justify-end">
            <button
              class="px-4 py-2 rounded-lg border [border-color:var(--color-card-border)] [background-color:var(--color-card)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors"
              onclick={() => {
                licenseKeyInput = "";
                licenseError = "";
                if (isLicensed) licenseModalOpen = false;
              }}
            >
              {isLicensed ? 'Cancel' : 'Clear'}
            </button>
            <button
              class="px-4 py-2 rounded-lg border [border-color:var(--color-primary)] [background-color:var(--color-primary)] text-white hover:opacity-90 transition-opacity"
              onclick={activateLicense}
            >
              Activate
            </button>
          </div>
          <div class="text-xs [color:var(--color-muted)] mt-2">
            <p>License key format: XXXXX-XXXXX-XXXXX-XXXXX-XXXXX</p>
            <p class="mt-1">License is validated and stored securely by the backend.</p>
          </div>
        </div>
      </Modal>
    </div>
  {/if}

  <h1 class="text-4xl font-bold game-font">Timer</h1>

  <div class="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-4">
    {#each presets.filter(p => isDev || p !== 5) as p}
      {@const originalIndex = presets.indexOf(p)}
      <div class="relative p-2 rounded-xl shadow-[0_8px_20px_rgba(0,0,0,0.35)] [background-color:var(--color-card)] border border-[var(--color-card-border)] group theme-light:shadow-[0_8px_32px_rgba(120,120,140,0.13)]">
        <button
          class="w-full p-4 text-left bg-transparent [color:var(--color-foreground)] relative z-[1] rounded-xl border border-[var(--color-card-border)] {selectedSeconds === p ? '![border-color:var(--color-primary)]' : ''}"
          onclick={() => setPreset(p)}
        >
          <span class="text-lg font-semibold block font-['Play',Inter,ui-sans-serif,system-ui,sans-serif]"
            >{Math.floor(p / 3600)
              ? `${Math.floor(p / 3600)}h `
              : ""}{Math.floor((p % 3600) / 60)}m{p % 60
              ? ` ${p % 60}s`
              : ""}</span
          >
          <span class="text-xs [color:var(--color-muted)]">click to select</span>
        </button>
        <div class="absolute top-2 right-2 inline-flex gap-1.5 opacity-0 pointer-events-none transition-opacity z-[2] group-hover:opacity-100 group-hover:pointer-events-auto group-focus-within:opacity-100 group-focus-within:pointer-events-auto">
          <button
            class="bg-black/35 [color:var(--color-foreground)] border [border-color:var(--color-card-border)] rounded-lg px-2 py-1 cursor-pointer hover:[color:var(--color-foreground)]"
            title="Edit"
            onclick={() => openEdit(originalIndex)}
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
            class="bg-black/35 [color:var(--color-foreground)] border [border-color:var(--color-card-border)] rounded-lg px-2 py-1 cursor-pointer hover:text-[#ff6b6b]"
            title="Remove"
            onclick={() => removePresetAt(originalIndex)}
            aria-label="Remove preset">×</button
          >
        </div>
      </div>
    {/each}
    <div class="relative p-2 rounded-xl shadow-[0_8px_20px_rgba(0,0,0,0.35)] [background-color:var(--color-card)] border border-[var(--color-card-border)] theme-light:shadow-[0_8px_32px_rgba(120,120,140,0.13)]">
      <button
        class="w-full p-4 text-left bg-transparent [color:var(--color-foreground)] rounded-xl border border-[var(--color-card-border)]"
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

  <div class="text-6xl text-center tracking-wider game-font {!isLicensed ? 'opacity-50 pointer-events-none' : ''}">{formatTime(remainingSeconds)}</div>
  {#if showEndQuestion && askOnEnd}
    <div class="flex flex-col items-center gap-4 mt-4">
      <div class="text-lg font-semibold [color:var(--color-foreground)] text-center px-4">{endQuestionText}</div>
      <div class="flex gap-4 justify-center items-center">
        <button 
          class="w-14 h-14 rounded-xl border-2 border-[rgba(0,255,128,0.6)] bg-[rgba(0,255,128,0.2)] hover:bg-[rgba(0,255,128,0.3)] hover:border-[rgba(0,255,128,0.8)] text-white grid place-items-center transition-all active:scale-95 shadow-[0_4px_12px_rgba(0,255,128,0.3)] group" 
          onclick={() => answerEnd(true)} 
          title="Yes"
          aria-label="Yes"
        >
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" class="group-hover:scale-110 transition-transform">
            <path d="M20.285 6.707a1 1 0 0 0-1.414-1.414L9.5 14.664l-3.371-3.37a1 1 0 1 0-1.415 1.413l4.078 4.079a1 1 0 0 0 1.415 0l10.078-10.079z" fill="currentColor" />
          </svg>
        </button>
        <button 
          class="w-14 h-14 rounded-xl border-2 border-[rgba(255,64,64,0.6)] bg-[rgba(255,64,64,0.2)] hover:bg-[rgba(255,64,64,0.3)] hover:border-[rgba(255,64,64,0.8)] text-white grid place-items-center transition-all active:scale-95 shadow-[0_4px_12px_rgba(255,64,64,0.3)] group" 
          onclick={() => answerEnd(false)} 
          title="No"
          aria-label="No"
        >
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" class="group-hover:scale-110 transition-transform">
            <path d="M18.3 5.71a1 1 0 0 0-1.41 0L12 10.59 7.11 5.7a1 1 0 1 0-1.41 1.42L10.59 12l-4.9 4.89a1 1 0 1 0 1.41 1.42L12 13.41l4.89 4.9a1 1 0 0 0 1.41-1.42L13.41 12l4.89-4.89a1 1 0 0 0 0-1.4z" fill="currentColor" />
          </svg>
        </button>
      </div>
    </div>
  {/if}
  <div class="text-center [color:var(--color-muted)] -mt-2">Today: {wins}/{losses}</div>

  <section class="mt-3 overflow-x-auto">
    {#key heatmapKey}
    <div class="grid grid-cols-[16px_repeat(60,5px)] grid-rows-[12px_repeat(7,5px)] gap-[1px] items-center mt-3 w-fit max-w-full mx-auto">
      {#each buildMonthLabels(buildYearWeeks()) as ml}
        <div class="row-start-1 text-[8px] [color:var(--color-muted)] leading-none" style={`grid-column:${ml.col};`}>{ml.text}</div>
      {/each}
      {#each weekDays as w, j}
        <div class="col-start-1 text-[8px] [color:var(--color-muted)] justify-self-end leading-none" style={`grid-row:${j+2};`}>{w}</div>
      {/each}
      {#each buildYearWeeks() as wk, i}
        {#each wk as day, j}
          <div class={`w-1.5 h-1.5 rounded-[0.5px] grid place-items-center ${day.key === sameDayKey(new Date()) ? '[outline:0.5px_solid_var(--color-primary)] outline-offset-0.5' : ''}`} style={`grid-column:${i+2}; grid-row:${j+2}; background:${colorFor(day.wins, day.losses)};`}
               title={tooltipFor(day.key)}></div>
        {/each}
      {/each}
    </div>
    {/key}
  </section>

  <div class="flex gap-2 flex-wrap items-center p-2 rounded-lg {!isLicensed ? 'opacity-50 pointer-events-none' : ''}">
    {#if !isRunning}
      <button class="h-[38px] flex items-center gap-1.5 rounded-[10px] border border-[#72d480] px-4 font-semibold text-[#05210c] [background-color:var(--color-primary)] shadow-[0_8px_20px_rgba(0,0,0,0.35)] cursor-pointer" onclick={start} disabled={!isLicensed}>
        <Icon variant="play" size="sm" />
        <span>{isFreshStart() ? 'Start' : 'Resume'}</span>
      </button>
    {:else}
      <button class="h-[38px] flex items-center gap-1.5 rounded-[10px] border [border-color:var(--color-card-border)] px-4 font-semibold [color:var(--color-foreground)] bg-[#1b1b1b] shadow-[0_8px_20px_rgba(0,0,0,0.35)] cursor-pointer" onclick={stop} disabled={!isLicensed}>
        <Icon variant="pause" size="sm" />
        <span>Pause</span>
      </button>
    {/if}
    {#if !isFreshStart()}
      <button class="h-[38px] flex items-center gap-1.5 rounded-[10px] border [border-color:var(--color-card-border)] px-4 font-semibold [color:var(--color-foreground)] bg-transparent cursor-pointer" onclick={reset} disabled={!isLicensed}>
        <Icon variant="reset" size="sm" />
        <span>Reset</span>
      </button>
    {/if}
    <button
      class="h-[38px] flex items-center gap-1.5 rounded-[10px] border px-4 font-semibold [color:var(--color-foreground)] bg-transparent cursor-pointer {overlayOpen ? 'overlay-active' : 'border-[var(--color-card-border)]'}"
      onclick={openOverlayAndStart}
      title="Toggle overlay"
    >
      <Icon variant="overlay" size="sm" />
      <span>{overlayOpen ? "Overlay mode (on)" : "Overlay mode"}</span>
    </button>
    <button
      class="h-[38px] w-[38px] flex items-center justify-center bg-black/35 [color:var(--color-foreground)] border [border-color:var(--color-card-border)] rounded-lg cursor-pointer relative group {settingsOpen ? '![border-color:var(--color-primary)]' : ''}"
      data-tip="Settings"
      onclick={() => {
        requestNotifyPermission();
        editOpen = false;
        settingsOpen = true;
      }}
      aria-label="Settings"
    >
      <Icon variant="settings" size="md" />
      <span class="absolute -top-[30px] left-1/2 -translate-x-1/2 bg-black/80 text-white rounded-md px-2 py-1 text-xs whitespace-nowrap opacity-0 pointer-events-none transition-opacity group-hover:opacity-100">Settings</span>
    </button>
    <button
      class="h-[38px] w-[38px] flex items-center justify-center bg-black/35 [color:var(--color-foreground)] border [border-color:var(--color-card-border)] rounded-lg cursor-pointer relative group {autoMessagesEnabled ? '![border-color:var(--color-primary)]' : ''}"
      data-tip="Auto-messages"
      onclick={() => {
        autoMessagesEnabled = !autoMessagesEnabled;
        requestNotifyPermission();
      }}
      aria-label="Auto messages"
    >
      <Icon variant="message" size="md" />
      <span class="absolute -top-[30px] left-1/2 -translate-x-1/2 bg-black/80 text-white rounded-md px-2 py-1 text-xs whitespace-nowrap opacity-0 pointer-events-none transition-opacity group-hover:opacity-100">Auto-messages</span>
    </button>
    <button
      class="h-[38px] w-[38px] flex items-center justify-center bg-black/35 [color:var(--color-foreground)] border [border-color:var(--color-card-border)] rounded-lg cursor-pointer relative group"
      aria-label="Toggle mute"
      onclick={() => (allMuted = !allMuted)}
    >
      <Icon variant={allMuted ? "bell-slash" : "bell"} size="md" />
      <span class="absolute -top-[30px] left-1/2 -translate-x-1/2 bg-black/80 text-white rounded-md px-2 py-1 text-xs whitespace-nowrap opacity-0 pointer-events-none transition-opacity group-hover:opacity-100">{allMuted ? "Unmute all notifications" : "Mute all notifications"}</span>
    </button>
    <div class="relative">
      <button
        class="h-[38px] flex items-center gap-1.5 bg-black/35 [color:var(--color-foreground)] border [border-color:var(--color-card-border)] rounded-lg px-2.5 cursor-pointer text-sm leading-none relative group {helpMenuOpen ? '![border-color:var(--color-primary)]' : ''}"
        data-tip="Help"
        onclick={(e) => {
          e.stopPropagation();
          helpMenuOpen = !helpMenuOpen;
        }}
        aria-label="Help"
      >
        <Icon variant="help" size="sm" />
        <span>Help</span>
        <span class="absolute -top-[30px] left-1/2 -translate-x-1/2 bg-black/80 text-white rounded-md px-2 py-1 text-xs whitespace-nowrap opacity-0 pointer-events-none transition-opacity group-hover:opacity-100">Help</span>
      </button>
      {#if helpMenuOpen}
        <div 
          class="absolute right-0 top-full mt-1 w-56 rounded-lg border [border-color:var(--color-card-border)] [background-color:var(--color-card)] shadow-lg z-50 py-1"
          onclick={(e) => e.stopPropagation()}
        >
          <button
            class="w-full text-left px-4 py-2 text-sm [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors"
            onclick={openChangeLicense}
          >
            Change License Key
          </button>
          {#if isLicensed}
            <button
              class="w-full text-left px-4 py-2 text-sm text-red-500 hover:text-red-600 hover:[background-color:var(--color-card-border)] transition-colors"
              onclick={removeLicense}
            >
              Remove License Key
            </button>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <Modal
    open={editOpen}
    title="Edit time"
    subtitle="Adjust hours, minutes and seconds"
    onclose={closeEdit}
  >
    <div class="grid gap-3">
      <label class="flex gap-3 items-center">
        <span class="[color:var(--color-muted)] w-[90px]">Hours</span>
        <Select bind:value={editHours} class="p-2">
          {#each Array.from({ length: 13 }, (_, n) => n) as h}
            <option value={h}>{h}</option>
          {/each}
        </Select>
      </label>
      <label class="flex gap-3 items-center">
        <span class="[color:var(--color-muted)] w-[90px]">Minutes</span>
        <Select bind:value={editMinutes} class="p-2">
          {#each Array.from({ length: 60 }, (_, n) => n) as m}
            <option value={m}>{m}</option>
          {/each}
        </Select>
      </label>
      <label class="flex gap-3 items-center">
        <span class="[color:var(--color-muted)] w-[90px]">Seconds</span>
        <Select bind:value={editSeconds} class="p-2">
          {#each Array.from({ length: 60 }, (_, n) => n) as s}
            <option value={s}>{s}</option>
          {/each}
        </Select>
      </label>
    </div>
    <div class="flex gap-2 justify-end mt-3">
      {#if editingIndex !== null}
        <button class="rounded-[10px] border [border-color:var(--color-card-border)] px-4 py-2.5 font-semibold [color:var(--color-foreground)] bg-transparent cursor-pointer" onclick={closeEdit}>Cancel</button>
        <button class="rounded-[10px] border border-[#72d480] px-4 py-2.5 font-semibold text-[#05210c] [background-color:var(--color-primary)] cursor-pointer" onclick={saveEdit}>Save</button>
      {:else}
        <button class="rounded-[10px] border [border-color:var(--color-card-border)] px-4 py-2.5 font-semibold [color:var(--color-foreground)] bg-transparent cursor-pointer" onclick={closeEdit}>Cancel</button>
        <button class="rounded-[10px] border border-[#72d480] px-4 py-2.5 font-semibold text-[#05210c] [background-color:var(--color-primary)] cursor-pointer" onclick={addFromEdit}>Add</button>
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
      <div class="grid gap-3">
        <label class="flex gap-3 items-center">
          <span class="[color:var(--color-muted)] w-[180px]">Critical threshold (%)</span>
          <Input type="range" min={0} max={99} bind:value={criticalPercent} onchange={_ => saveAppSettings()} class="flex-1" />
          <span>{criticalPercent}%</span>
        </label>
        <label class="flex gap-3 items-center">
          <span class="[color:var(--color-muted)] w-[180px]">Danger threshold (%)</span>
          <Input type="range" min={0} max={99} bind:value={dangerPercent} onchange={_ => saveAppSettings()} class="flex-1" />
          <span>{dangerPercent}%</span>
        </label>
        <div class="[color:var(--color-muted)] text-xs">
          Zone logic: Danger ≤ {dangerPercent}%, Critical between {dangerPercent}% and {criticalPercent}%, OK ≥ {criticalPercent}%.
        </div>

        <div class="mt-3 font-semibold">Messages (trigger when remaining ≤ %)</div>
        {#each userMessages as msg, idx}
          <div class="flex gap-2 items-center">
            <Input
              type="number"
              min={1}
              max={99}
              bind:value={msg.percent}
              class="w-20 p-2"
            />
            <Input
              type="text"
              bind:value={msg.text}
              placeholder="Message to show"
              class="flex-1 p-2"
            />
            <Select bind:value={msg.sound} class="p-2">
              <option value="beep">Beep</option>
              <option value="heartbeat">Heartbeat</option>
              <option value="none">None</option>
              <option value="custom">Custom</option>
            </Select>
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
                class="bg-[#0f0f0f] [color:var(--color-foreground)] border [border-color:var(--color-card-border)] rounded-lg p-1"
              />
            {/if}
            <button
              class="bg-black/35 [color:var(--color-foreground)] border [border-color:var(--color-card-border)] rounded-lg px-2 py-1 cursor-pointer hover:text-[#ff6b6b]"
              title="Remove"
              onclick={() => (userMessages = userMessages.filter((_, i) => i !== idx))}>×</button>
          </div>
        {/each}
        <div class="flex gap-3">
          <button
            class="bg-black/35 [color:var(--color-foreground)] border [border-color:var(--color-card-border)] rounded-lg px-2 py-1 cursor-pointer"
            onclick={() => (userMessages = [...userMessages, { percent: 30, text: "Keep going!", sound: "beep" }])}
            title="Add message">＋</button>
        </div>

        <div class="mt-3 font-semibold">End question</div>
        <Checkbox bind:checked={askOnEnd} onchange={_ => { persistEndQuestionSettings(); saveAppSettings(); }} label="Ask when timer ends" />
        <label class="flex gap-3 items-center">
          <span class="[color:var(--color-muted)] w-[180px]">Question text</span>
          <Input type="text" bind:value={endQuestionText} onblur={_ => { persistEndQuestionSettings(); saveAppSettings(); }} class="flex-1 p-2" />
        </label>

        <div class="mt-3 font-semibold">Zone colors</div>
        <div class="flex gap-3 items-center">
          <span class="[color:var(--color-muted)] w-[180px]">Danger color</span>
          <Input type="color" bind:value={dangerColor} onchange={_ => saveAppSettings()} />
        </div>
        <div class="flex gap-3 items-center">
          <span class="[color:var(--color-muted)] w-[180px]">Critical color</span>
          <Input type="color" bind:value={criticalColor} onchange={_ => saveAppSettings()} />
        </div>
        <label class="flex gap-3 items-center"><span class="[color:var(--color-muted)] w-[180px]">When to play zone sound</span>
          <Select bind:value={zoneSoundMode} onchange={_ => { persistZoneSoundSettings(); saveAppSettings(); }} class="flex-1 p-2">
            <option value="none">Never</option>
            <option value="danger">In Danger only</option>
            <option value="critical">In Critical only</option>
            <option value="all">Any phase change</option>
          </Select>
        </label>
        <label class="flex gap-3 items-center"><span class="[color:var(--color-muted)] w-[180px]">Show notification when entering zone</span>
          <Select bind:value={zoneNotifyMode} onchange={_ => { persistZoneSoundSettings(); saveAppSettings(); }} class="flex-1 p-2">
            <option value="none">Never</option>
            <option value="danger">In Danger only</option>
            <option value="critical">In Critical only</option>
            <option value="all">Any phase change</option>
          </Select>
        </label>
        <Checkbox bind:checked={overlayAlwaysOnTop} onchange={_ => { persistOverlaySettings(); saveAppSettings(); }} label="Overlay always stays on top" />
        <Checkbox bind:checked={overlayPopupOnEnd} onchange={_ => { persistOverlaySettings(); saveAppSettings(); }} label="Bring overlay to top when timer ends" />
        
        <div class="mt-3 font-semibold">Overlay Mode</div>
        <label class="flex gap-3 items-center">
          <span class="[color:var(--color-muted)] w-[180px]">Mode</span>
          <Select bind:value={overlayMode} onchange={_ => { persistOverlaySettings(); saveAppSettings(); }} class="flex-1 p-2">
            <option value="normal">Normal</option>
            <option value="progressbar">Progress Bar</option>
          </Select>
        </label>
        {#if overlayMode === "progressbar"}
          <label class="flex gap-3 items-center">
            <span class="[color:var(--color-muted)] w-[180px]">Progress bar color</span>
            <Input type="color" bind:value={overlayProgressBarColor} onchange={_ => { persistOverlaySettings(); saveAppSettings(); }} />
          </label>
          <label class="flex gap-3 items-center">
            <span class="[color:var(--color-muted)] w-[180px]">Progress bar finished color</span>
            <Input type="color" bind:value={overlayProgressBarFinishedColor} onchange={_ => { persistOverlaySettings(); saveAppSettings(); }} />
          </label>
          <label class="flex gap-3 items-center">
            <span class="[color:var(--color-muted)] w-[180px]">Progress bar type</span>
            <Select bind:value={overlayProgressBarType} onchange={_ => { persistOverlaySettings(); saveAppSettings(); }} class="flex-1 p-2">
              <option value="full">Full</option>
            </Select>
          </label>
        {/if}
        <label class="flex gap-3 items-center"><span class="[color:var(--color-muted)] w-[180px]">Color Theme</span>
          <Select bind:value={colorTheme} onchange={_ => setTheme(colorTheme)} class="flex-1 p-2">
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </Select>
        </label>

        <div class="mt-3 font-semibold">Background Images</div>
        <label class="flex flex-col gap-2">
          <span class="[color:var(--color-muted)] text-sm">App Background</span>
          <div class="flex gap-2 items-center">
            <input
              type="file"
              accept="image/*"
              class="flex-1 text-sm [color:var(--color-foreground)] [background-color:var(--color-card)] border [border-color:var(--color-card-border)] rounded-lg px-3 py-2 file:mr-4 file:py-1 file:px-3 file:rounded-lg file:border-0 file:text-sm file:font-semibold file:[background-color:var(--color-primary)] file:text-[#05210c] hover:file:opacity-90 cursor-pointer"
              onchange={async (e) => {
                const file = (e.currentTarget as HTMLInputElement).files?.[0];
                if (file) {
                  const reader = new FileReader();
                  reader.onload = () => {
                    appBackgroundImage = reader.result as string;
                    saveAppSettings();
                  };
                  reader.readAsDataURL(file);
                }
              }}
            />
            {#if appBackgroundImage}
              <button
                class="px-3 py-2 text-sm rounded-lg border [border-color:var(--color-card-border)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors"
                onclick={() => {
                  appBackgroundImage = null;
                  saveAppSettings();
                }}
              >
                Remove
              </button>
            {/if}
      </div>
        </label>
        <label class="flex flex-col gap-2">
          <span class="[color:var(--color-muted)] text-sm">Overlay Background</span>
          <div class="flex gap-2 items-center">
            <input
              type="file"
              accept="image/*"
              class="flex-1 text-sm [color:var(--color-foreground)] [background-color:var(--color-card)] border [border-color:var(--color-card-border)] rounded-lg px-3 py-2 file:mr-4 file:py-1 file:px-3 file:rounded-lg file:border-0 file:text-sm file:font-semibold file:[background-color:var(--color-primary)] file:text-[#05210c] hover:file:opacity-90 cursor-pointer"
              onchange={async (e) => {
                const file = (e.currentTarget as HTMLInputElement).files?.[0];
                if (file) {
                  const reader = new FileReader();
                  reader.onload = () => {
                    overlayBackgroundImage = reader.result as string;
                    saveAppSettings();
                    // Broadcast to overlay
                    if (bc) {
                      bc.postMessage({
                        source: 'main',
                        type: 'overlay_background',
                        backgroundImage: overlayBackgroundImage
                      });
                    }
                  };
                  reader.readAsDataURL(file);
                }
              }}
            />
            {#if overlayBackgroundImage}
              <button
                class="px-3 py-2 text-sm rounded-lg border [border-color:var(--color-card-border)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors"
                onclick={() => {
                  overlayBackgroundImage = null;
                  saveAppSettings();
                  if (bc) {
                    bc.postMessage({
                      source: 'main',
                      type: 'overlay_background',
                      backgroundImage: null
                    });
                  }
                }}
              >
                Remove
              </button>
            {/if}
          </div>
        </label>
      </div>
      <div class="flex justify-end gap-2 mt-3">
        <button class="rounded-[10px] border [border-color:var(--color-card-border)] px-4 py-2.5 font-semibold [color:var(--color-foreground)] bg-transparent cursor-pointer" onclick={() => { resetAll(); settingsOpen = false; }} title="Reset all data">Reset all</button>
        <button class="rounded-[10px] border [border-color:var(--color-card-border)] px-4 py-2.5 font-semibold [color:var(--color-foreground)] [background-color:var(--color-card)] shadow-[0_8px_20px_rgba(0,0,0,0.35)] cursor-pointer" onclick={() => (settingsOpen = false)}>Close</button>
      </div>
    </Modal>
  {/if}
</main>

<style>
  button.overlay-active {
    background-color: rgba(115, 212, 128, 0.15) !important;
    border-color: var(--color-primary) !important;
    box-shadow: 0 0 0 2px rgba(115, 212, 128, 0.35) !important;
  }
  :global(.theme-light) button.overlay-active {
    background-color: rgba(53, 204, 96, 0.15) !important;
    box-shadow: 0 0 0 2px rgba(53, 204, 96, 0.35) !important;
  }
</style>
