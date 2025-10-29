<script lang="ts">
  import { onMount } from "svelte";

  let initialSeconds = $state(60);
  let remainingSeconds = $state(initialSeconds);
  let isRunning = $state(false);
  let intervalId: number | null = null;
  let criticalPercent = $state(50);
  let dangerPercent = $state(20);
  let startAtMs: number | null = null;
  let runDurationSeconds = $state(initialSeconds);
  let lastStartSig: number | null = null;
  type UserMessage = { percent: number; text: string; fired?: boolean };
  let userMessages = $state<UserMessage[]>([]);
  let prevZone: 'OK' | 'DANGER' | 'CRITICAL' | null = null;
  let now = $state(new Date());
  let dangerSound: 'heartbeat' | 'beep' | 'none' = $state('heartbeat');
  let dangerColor = $state('#ff4444');
  let criticalColor = $state('#ffa500');
  let bc: BroadcastChannel | null = null;
  let suppressBroadcast = false;
  let audioCtx: AudioContext | null = null;
  let dangerSoundTimer: number | null = null;

  function formatTime(totalSeconds: number) {
    const m = Math.floor(totalSeconds / 60)
      .toString()
      .padStart(2, "0");
    const s = Math.floor(totalSeconds % 60)
      .toString()
      .padStart(2, "0");
    return `${m}:${s}`;
  }

  function start() {
    if (isRunning) return;
    isRunning = true;
    if (remainingSeconds <= 0) remainingSeconds = initialSeconds;
    runDurationSeconds = initialSeconds;
    // Recompute start point from current remaining to avoid including paused time
    startAtMs = Date.now() - (runDurationSeconds - remainingSeconds) * 1000;
    lastStartSig = startAtMs;
    intervalId = window.setInterval(() => {
      if (startAtMs) {
        const elapsed = Math.floor((Date.now() - startAtMs) / 1000);
        remainingSeconds = Math.max(0, runDurationSeconds - elapsed);
      }
      if (remainingSeconds > 0) {
        // zone transitions
        const p = currentPercent();
        const z = zoneFromPercent(p);
        if (z !== prevZone) {
          prevZone = z;
          notifyDesktop('Timer zone changed', `Entered ${z} zone`);
          if (z === 'DANGER') startDangerSound(); else stopDangerSound();
        }
      } else {
        stop();
      }
    }, 1000);
    if (!suppressBroadcast) { try { bc?.postMessage({ source: 'overlay', type: 'start', duration: initialSeconds, startAtMs }); } catch {} }
  }

  function stop() {
    isRunning = false;
    if (intervalId !== null) {
      clearInterval(intervalId);
      intervalId = null;
    }
    // Clear start anchor to prevent drift while paused
    startAtMs = null;
    stopDangerSound();
    if (!suppressBroadcast) { try { bc?.postMessage({ source: 'overlay', type: 'pause', remaining: remainingSeconds, duration: initialSeconds }); } catch {} }
  }

  function reset() {
    stop();
    remainingSeconds = initialSeconds;
    startAtMs = null;
    if (!suppressBroadcast) { try { bc?.postMessage({ source: 'overlay', type: 'reset', duration: initialSeconds }); } catch {} }
  }

  function currentPercent() {
    return Math.max(0, Math.round((remainingSeconds / Math.max(1, initialSeconds)) * 100));
  }

  function zoneFromPercent(p: number): 'OK' | 'CRITICAL' | 'DANGER' {
    if (p <= dangerPercent) return 'DANGER';
    if (p <= criticalPercent) return 'CRITICAL';
    return 'OK';
  }

  function notifyDesktop(title: string, body: string) {
    try {
      if (typeof Notification !== 'undefined' && Notification.permission === 'granted') new Notification(title, { body });
    } catch {}
  }

  function ensureAudio() {
    if (!audioCtx) {
      try { audioCtx = new (window.AudioContext || (window as any).webkitAudioContext)(); } catch {}
    }
  }

  function playBeep(durationMs = 200, frequency = 880) {
    ensureAudio(); if (!audioCtx) return;
    const osc = audioCtx.createOscillator();
    const gain = audioCtx.createGain();
    osc.type = 'sine'; osc.frequency.value = frequency;
    gain.gain.setValueAtTime(0.15, audioCtx.currentTime);
    osc.connect(gain).connect(audioCtx.destination);
    osc.start();
    gain.gain.exponentialRampToValueAtTime(0.0001, audioCtx.currentTime + durationMs/1000);
    osc.stop(audioCtx.currentTime + durationMs/1000 + 0.02);
  }

  function playHeartbeat() { playBeep(80, 150); setTimeout(() => playBeep(80, 150), 180); }
  function startDangerSound() {
    stopDangerSound();
    if (dangerSound === 'none') return;
    if (dangerSound === 'beep') dangerSoundTimer = window.setInterval(() => playBeep(150, 880), 1200);
    else dangerSoundTimer = window.setInterval(() => playHeartbeat(), 1000);
  }
  function stopDangerSound() { if (dangerSoundTimer !== null) { clearInterval(dangerSoundTimer); dangerSoundTimer = null; } }

  onMount(async () => {
    // Parse query params for duration and autostart
    const url = new URL(window.location.href);
    const durationParam = url.searchParams.get("duration");
    if (durationParam) {
      const secs = Math.max(0, Number(durationParam) || 0);
      initialSeconds = secs;
      remainingSeconds = secs;
    }
    const autostart = url.searchParams.get("autostart");
    const startAt = url.searchParams.get('startAt');
    if (startAt) startAtMs = Number(startAt) || null;
    if (autostart === "1") start();

    const crit = url.searchParams.get('critical');
    const danger = url.searchParams.get('danger');
    if (crit) criticalPercent = Math.min(99, Math.max(1, Number(crit) || criticalPercent));
    if (danger) dangerPercent = Math.min(99, Math.max(1, Number(danger) || dangerPercent));
    const msgsStr = url.searchParams.get('msgs');
    if (msgsStr) {
      try {
        const decoded = JSON.parse(decodeURIComponent(atob(decodeURIComponent(msgsStr))));
        if (Array.isArray(decoded)) userMessages = decoded;
      } catch {}
    }
    const snd = url.searchParams.get('snd');
    if (snd === 'beep' || snd === 'heartbeat' || snd === 'none') dangerSound = snd;
    const csnd = url.searchParams.get('csnd');
    let criticalSound: 'heartbeat' | 'beep' | 'none' = 'none';
    if (csnd === 'heartbeat' || csnd === 'beep' || csnd === 'none') criticalSound = csnd;
    const dcol = url.searchParams.get('dangerColor');
    const ccol = url.searchParams.get('criticalColor');
    if (dcol) dangerColor = dcol;
    if (ccol) criticalColor = ccol;

    // Ensure always-on-top if Tauri is available
    const isTauri = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__;
    if (isTauri) {
      try {
        const { getCurrentWebviewWindow, WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
        const current = getCurrentWebviewWindow();
        await current.setAlwaysOnTop(true);
        // Ensure it floats across spaces/mission control if supported
        // @ts-expect-error older type defs may not include these options
        await current.setVisibleOnAllWorkspaces(true, { visibleOnFullScreen: true });
        const overlay = await WebviewWindow.getByLabel("overlay");
        await overlay?.show();
        await overlay?.setFocus();
      } catch {}
    }

    // Listen for navigation request from parent page to update this view
    if (isTauri) {
      const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      await WebviewWindow.getByLabel("overlay").then((parent) => parent?.listen<string>("overlay:navigate", (e: { payload: string }) => {
        const target = e.payload;
        if (typeof target === "string") {
          window.location.replace(target);
        }
      }));
    }

    // Clock updater
    setInterval(() => { now = new Date(); }, 1000);

    // Sync channel
    if ('BroadcastChannel' in window) {
      bc = new BroadcastChannel('timer-sync');
      bc.onmessage = (e) => {
        const data = e.data || {};
        if (data.source === 'overlay') return; // ignore our own
        if (data.type === 'start') {
          suppressBroadcast = true;
          const incomingStart = data.startAtMs ?? Date.now();
          if (lastStartSig && Math.abs(incomingStart - lastStartSig) < 150) return;
          initialSeconds = data.duration ?? initialSeconds;
          runDurationSeconds = data.duration ?? initialSeconds;
          startAtMs = incomingStart;
          lastStartSig = startAtMs;
          criticalPercent = data.critical ?? criticalPercent;
          dangerPercent = data.danger ?? dangerPercent;
          if (data.colors) { dangerColor = data.colors.dangerColor ?? dangerColor; criticalColor = data.colors.criticalColor ?? criticalColor; }
          if (Array.isArray(data.messages)) userMessages = data.messages;
          start();
          suppressBroadcast = false;
        } else if (data.type === 'pause') {
          suppressBroadcast = true;
          stop();
          startAtMs = null;
          if (typeof data.remaining === 'number') remainingSeconds = data.remaining;
          suppressBroadcast = false;
        } else if (data.type === 'reset') {
          suppressBroadcast = true;
          initialSeconds = data.duration ?? initialSeconds; reset();
          suppressBroadcast = false;
        } else if (data.type === 'preset') {
          initialSeconds = data.duration ?? initialSeconds; remainingSeconds = initialSeconds;
        }
      };
      try { bc.postMessage({ type: 'request_state' }); } catch {}
    }
  });
</script>

<main class="overlay {zoneFromPercent(currentPercent()).toLowerCase()} {remainingSeconds===0 ? 'ended' : ''}" data-tauri-drag-region style={`--danger:${dangerColor}; --critical:${criticalColor}` }>
  <div class="top">
    <div class="left">{now.toLocaleDateString()}</div>
    <div class="right">{now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</div>
  </div>
  <div class="time">{formatTime(remainingSeconds)}</div>
  <div class="bar">
    <div class="fill" style={`width: ${(1 - Math.max(0, remainingSeconds) / Math.max(1, initialSeconds)) * 100}%`}></div>
  </div>
  <div class="buttons">
    {#if !isRunning}
      <button onclick={start}>Start</button>
    {:else}
      <button onclick={stop}>Pause</button>
    {/if}
    <button onclick={reset}>Reset</button>
  </div>
</main>

<style>
.overlay {
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  -webkit-app-region: drag;
  padding: 8px;
}

.overlay.ok { background: transparent; }
.overlay.danger { background: color-mix(in srgb, var(--danger) 10%, transparent); }
.overlay.critical { background: color-mix(in srgb, var(--critical) 12%, transparent); }

.time {
  font-size: 40px;
  letter-spacing: 2px;
  animation: pulse 2s infinite ease-in-out;
}
.overlay.ended .time { animation: none; }
.overlay.danger .time { animation-duration: 1.25s; color: var(--danger); text-shadow: 0 0 12px color-mix(in srgb, var(--danger) 50%, transparent); }
.overlay.critical .time { animation-duration: 0.75s; color: var(--critical); text-shadow: 0 0 14px color-mix(in srgb, var(--critical) 55%, transparent); }
@keyframes pulse {
  0% { transform: scale(1); }
  50% { transform: scale(1.03); }
  100% { transform: scale(1); }
}

.bar { width: 90%; height: 8px; background: #242424; border-radius: 6px; overflow: hidden; -webkit-app-region: no-drag; }
.bar .fill { height: 100%; background: #8ef59b; transition: width .2s ease; }

.top { position: absolute; top: 6px; left: 8px; right: 8px; display: flex; justify-content: space-between; font-size: 12px; opacity: 0.9; -webkit-app-region: no-drag; }

.buttons { display: flex; gap: 6px; -webkit-app-region: no-drag; }

button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.4em 0.8em;
  font-size: 0.9em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
  cursor: pointer;
}

button:hover { border-color: #396cd8; }
button:active { background-color: #e8e8e8; }

@media (prefers-color-scheme: dark) {
  button { color: #ffffff; background-color: #0f0f0f98; }
  button:active { background-color: #0f0f0f69; }
}
</style>


