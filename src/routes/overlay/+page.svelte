<script lang="ts">
  import { onMount } from "svelte";
  import { LogicalPosition, LogicalSize } from "@tauri-apps/api/dpi";

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
  type MessageSound = "beep" | "heartbeat" | "none" | "custom";
  type UserMessageExt = UserMessage & { sound?: MessageSound; custom?: string };
  let userMessages = $state<UserMessageExt[]>([]);
  let prevZone: "OK" | "DANGER" | "CRITICAL" | null = null;
  let now = $state(new Date());
  let dangerSound: "heartbeat" | "beep" | "none" = $state("heartbeat");
  let dangerColor = $state("#ff4444");
  let criticalColor = $state("#ffa500");
  let bc: BroadcastChannel | null = null;
  let suppressBroadcast = false;
  const zoneSoundsEnabled = false;
  let audioCtx: AudioContext | null = null;
  let dangerSoundTimer: number | null = null;
  let hasEnded = $state(false);
  let hasStarted = $state(false);
  let resultRecorded = $state(false);
  const notifyEnabled = false;
  let askOnEnd = $state(true);
  let endQuestionText = $state("Did you achieve your goal?");
  let editingTime = $state(false);
  let editTimeStr = $state("");
  let editError = $state("");
  let editInputEl: HTMLInputElement | null = null;
  let overlayAlwaysOnTop = false;
  let colorTheme = "dark";
  let bringingToFront = false;
  let overlayMode = $state("normal");
  let progressBarColor = $state("#8ef59b");
  let progressBarType = $state("full");
  function updateOverlayTheme(newTheme: string) {
    colorTheme = newTheme;
    document.body.classList.toggle("theme-dark", colorTheme === "dark");
    document.body.classList.toggle("theme-light", colorTheme === "light");
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

  function start() {
    if (isRunning) return;
    isRunning = true;
    hasStarted = true;
    if (remainingSeconds <= 0) remainingSeconds = initialSeconds;
    runDurationSeconds = initialSeconds;
    startAtMs = Date.now() - (runDurationSeconds - remainingSeconds) * 1000;
    lastStartSig = startAtMs;
    intervalId = window.setInterval(() => {
      if (startAtMs) {
        const elapsed = Math.floor((Date.now() - startAtMs) / 1000);
        remainingSeconds = Math.max(0, runDurationSeconds - elapsed);
      }
      if (remainingSeconds > 0) {
        const p = currentPercent();
        const z = zoneFromPercent(p);
        if (z !== prevZone) {
          prevZone = z;
          notifyDesktop("Timer", `Entered ${z} zone`);
          if (zoneSoundsEnabled) {
            if (z === "DANGER") startDangerSound();
            else stopDangerSound();
          }
        }
        // Auto-messages are handled by the main window to prevent duplicates
        // Only process zone changes here
      } else {
        stopDangerSound();
        pause();
        hasEnded = true;
      }
    }, 1000);
    if (!suppressBroadcast) {
      try {
        bc?.postMessage({
          source: "overlay",
          type: "start",
          duration: initialSeconds,
          startAtMs,
        });
      } catch {}
    }
  }

  function pause() {
    isRunning = false;
    if (intervalId !== null) {
      clearInterval(intervalId);
      intervalId = null;
    }
    startAtMs = null;
    stopDangerSound();
    if (!suppressBroadcast) {
      try {
        bc?.postMessage({
          source: "overlay",
          type: "pause",
          remaining: remainingSeconds,
          duration: initialSeconds,
        });
      } catch {}
    }
  }

  function reset() {
    pause();
    remainingSeconds = initialSeconds;
    startAtMs = null;
    if (!suppressBroadcast) {
      try {
        bc?.postMessage({
          source: "overlay",
          type: "reset",
          duration: initialSeconds,
        });
      } catch {}
    }
    hasEnded = false;
    resultRecorded = false;
  }

  function currentPercent() {
    return Math.max(
      0,
      Math.round((remainingSeconds / Math.max(1, initialSeconds)) * 100)
    );
  }
  
  function progressPercent() {
    if (initialSeconds <= 0) return 0;
    const elapsed = initialSeconds - remainingSeconds;
    const percent = (elapsed / initialSeconds) * 100;
    return Math.max(0, Math.min(100, percent));
  }

  function zoneFromPercent(p: number): "OK" | "CRITICAL" | "DANGER" {
    if (p <= dangerPercent) return "DANGER";
    if (p <= criticalPercent) return "CRITICAL";
    return "OK";
  }

  function notifyDesktop(title: string, body: string) {
    if (!notifyEnabled) return;
    try {
      if (
        typeof Notification !== "undefined" &&
        Notification.permission === "granted"
      )
        new Notification(title, { body });
    } catch {}
  }

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
    playBeep(80, 150);
    setTimeout(() => playBeep(80, 150), 180);
  }
  function startDangerSound() {
    stopDangerSound();
    if (dangerSound === "none") return;
    if (dangerSound === "beep")
      dangerSoundTimer = window.setInterval(() => playBeep(150, 880), 1200);
    else dangerSoundTimer = window.setInterval(() => playHeartbeat(), 1000);
  }
  function stopDangerSound() {
    if (dangerSoundTimer !== null) {
      clearInterval(dangerSoundTimer);
      dangerSoundTimer = null;
    }
  }

  function openTimeEditor() {
    if (isRunning) return;
    editingTime = true;
    editError = "";
    const base = initialSeconds;
    const h = Math.floor(base / 3600);
    const m = Math.floor((base % 3600) / 60);
    const s = base % 60;
    editTimeStr = h
      ? `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`
      : `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }
  function saveTimeEdit() {
    const re = /^(\d{1,2}:)?\d{1,2}:\d{2}$/;
    if (!re.test(editTimeStr.trim())) {
      editError = "Use mm:ss or hh:mm:ss";
      setTimeout(() => editInputEl?.focus(), 0);
      return;
    }
    const parts = editTimeStr.trim().split(":").map(Number);
    let h = 0,
      m = 0,
      s = 0;
    if (parts.length === 3) {
      [h, m, s] = parts;
    } else {
      [m, s] = parts;
    }
    if (m > 59 || s > 59 || h < 0 || m < 0 || s < 0) {
      editError = "Minutes/seconds must be 0-59";
      setTimeout(() => editInputEl?.focus(), 0);
      return;
    }
    let total = h * 3600 + m * 60 + s;
    if (!Number.isFinite(total) || total < 1) {
      editError = "Time must be at least 1s";
      setTimeout(() => editInputEl?.focus(), 0);
      return;
    }
    remainingSeconds = total;
    runDurationSeconds = total;
    initialSeconds = total;
    editingTime = false;
    editError = "";
  }
  function cancelTimeEdit() {
    editingTime = false;
    editError = "";
  }

  function requestOverlayGoNotOnTop() {
    const isTauri =
      typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
    if (isTauri && !overlayAlwaysOnTop) {
      import("@tauri-apps/api/webviewWindow").then(
        ({ getCurrentWebviewWindow }) => {
          getCurrentWebviewWindow().setAlwaysOnTop(false);
        }
      );
    }
  }

  onMount(async () => {
    const url = new URL(window.location.href);
    const durationParam = url.searchParams.get("duration");
    if (durationParam) {
      const secs = Math.max(0, Number(durationParam) || 0);
      initialSeconds = secs;
      remainingSeconds = secs;
    }
    const autostart = url.searchParams.get("autostart");
    const startAt = url.searchParams.get("startAt");
    if (startAt) startAtMs = Number(startAt) || null;
    if (autostart === "1") start();

    const crit = url.searchParams.get("critical");
    const danger = url.searchParams.get("danger");
    if (crit)
      criticalPercent = Math.min(
        99,
        Math.max(1, Number(crit) || criticalPercent)
      );
    if (danger)
      dangerPercent = Math.min(
        99,
        Math.max(1, Number(danger) || dangerPercent)
      );
    const msgsStr = url.searchParams.get("msgs");
    if (msgsStr) {
      try {
        const decoded = JSON.parse(
          decodeURIComponent(atob(decodeURIComponent(msgsStr)))
        );
        if (Array.isArray(decoded)) userMessages = decoded;
      } catch {}
    }
    const snd = url.searchParams.get("snd");
    if (snd === "beep" || snd === "heartbeat" || snd === "none")
      dangerSound = snd;
    const csnd = url.searchParams.get("csnd");
    let criticalSound: "heartbeat" | "beep" | "none" = "none";
    if (csnd === "heartbeat" || csnd === "beep" || csnd === "none")
      criticalSound = csnd;
    const dcol = url.searchParams.get("dangerColor");
    const ccol = url.searchParams.get("criticalColor");
    if (dcol) dangerColor = dcol;
    if (ccol) criticalColor = ccol;
    const ask = url.searchParams.get("ask");
    askOnEnd = ask === "1";
    const q = url.searchParams.get("q");
    if (q) {
      try {
        endQuestionText = decodeURIComponent(q);
      } catch {}
    }
    const alwaysOnTopParam = url.searchParams.get("alwaysOnTop");
    const overlayModeParam = url.searchParams.get("overlayMode");
    const progressBarColorParam = url.searchParams.get("progressBarColor");
    const progressBarTypeParam = url.searchParams.get("progressBarType");
    overlayAlwaysOnTop =
      alwaysOnTopParam === "1" || alwaysOnTopParam === "true";
    if (overlayModeParam) overlayMode = overlayModeParam;
    if (progressBarColorParam) progressBarColor = progressBarColorParam;
    if (progressBarTypeParam) progressBarType = progressBarTypeParam;

    const isTauri =
      typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
    if (isTauri) {
      try {
        const { getCurrentWebviewWindow, WebviewWindow } = await import(
          "@tauri-apps/api/webviewWindow"
        );
        const current = getCurrentWebviewWindow();
        
        await current.onCloseRequested(async (event) => {
          if (isRunning || hasStarted) {
            const ok =
              typeof window !== "undefined"
                ? window.confirm(
                    "Timer is active. Close the overlay window?"
                  )
                : true;
            if (!ok) {
              event.preventDefault();
              return;
            }
          }
          try {
            bc?.postMessage({ source: "overlay", type: "close" });
          } catch {}
        });
        
        await current.setAlwaysOnTop(!!overlayAlwaysOnTop);
        await current.setVisibleOnAllWorkspaces(true);
        try {
          const posStr = localStorage.getItem("overlayPos");
          if (posStr) {
            const { x, y } = JSON.parse(posStr);
            const px = Number(x) || 0;
            const py = Number(y) || 0;
            await current.setPosition(new LogicalPosition(px, py));
          } else {
            await current.setPosition(new LogicalPosition(0, 0));
          }
          await current.listen("tauri://move", async (e: any) => {
            try {
              const p = e?.payload?.position || e?.payload;
              if (p) {
                const px = Number(p.x);
                const py = Number(p.y);
                if (!Number.isNaN(px) && !Number.isNaN(py)) {
                  localStorage.setItem(
                    "overlayPos",
                    JSON.stringify({ x: px, y: py })
                  );
                }
              }
            } catch {}
          });
        } catch {}
        const overlay = await WebviewWindow.getByLabel("overlay");
        await overlay?.show();
        await overlay?.setFocus();
      } catch {}
    }

    if (isTauri) {
      const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      await WebviewWindow.getByLabel("overlay").then((parent) =>
        parent?.listen<string>("overlay:navigate", (e: { payload: string }) => {
          const target = e.payload;
          if (typeof target === "string") {
            window.location.replace(target);
          }
        })
      );
    }

    setInterval(() => {
      now = new Date();
    }, 1000);

    if ("BroadcastChannel" in window) {
      bc = new BroadcastChannel("timer-sync");
      bc.onmessage = (e) => {
        (async () => {
          const data = e.data || {};
          if (data.source === "overlay") return;
          if (data.type === "start") {
            suppressBroadcast = true;
            const incomingStart = data.startAtMs ?? Date.now();
            if (lastStartSig && Math.abs(incomingStart - lastStartSig) < 150)
              return;
            initialSeconds = data.duration ?? initialSeconds;
            runDurationSeconds = data.duration ?? initialSeconds;
            startAtMs = incomingStart;
            lastStartSig = startAtMs;
            criticalPercent = data.critical ?? criticalPercent;
            dangerPercent = data.danger ?? dangerPercent;
            if (data.colors) {
              dangerColor = data.colors.dangerColor ?? dangerColor;
              criticalColor = data.colors.criticalColor ?? criticalColor;
            }
            if (Array.isArray(data.messages)) {
              userMessages = data.messages.map((msg: any) => ({ ...msg, fired: msg.fired || false }));
            }
            start();
            suppressBroadcast = false;
          } else if (data.type === "pause") {
            suppressBroadcast = true;
            pause();
            startAtMs = null;
            if (typeof data.remaining === "number")
              remainingSeconds = data.remaining;
            suppressBroadcast = false;
          } else if (data.type === "end") {
            pause();
            hasEnded = true;
            resultRecorded = false;
          } else if (data.type === "result") {
            resultRecorded = true;
            hasEnded = true;
          } else if (data.type === "reset") {
            suppressBroadcast = true;
            initialSeconds = data.duration ?? initialSeconds;
            reset();
            suppressBroadcast = false;
          } else if (data.type === "preset") {
            initialSeconds = data.duration ?? initialSeconds;
            remainingSeconds = initialSeconds;
          } else if (data.type === "message_fired" && typeof data.messageIndex === "number") {
            // Mark message as fired when main window fires it
            if (userMessages[data.messageIndex]) {
              userMessages[data.messageIndex].fired = true;
            }
          } else if (
            data.source === "main" &&
            data.type === "overlay_settings"
          ) {
            if (typeof data.alwaysOnTop === "boolean") {
              overlayAlwaysOnTop = data.alwaysOnTop;
              if (isTauri) {
                const { getCurrentWebviewWindow } = await import(
                  "@tauri-apps/api/webviewWindow"
                );
                const current = getCurrentWebviewWindow();
                await current.setAlwaysOnTop(overlayAlwaysOnTop);
              }
            }
            if (typeof data.overlayMode === "string") {
              overlayMode = data.overlayMode;
            }
            if (typeof data.progressBarColor === "string") {
              progressBarColor = data.progressBarColor;
            }
            if (typeof data.progressBarType === "string") {
              progressBarType = data.progressBarType;
            }
          } else if (data.source === "main" && data.type === "overlay_popup") {
            if (isTauri && !bringingToFront) {
              bringingToFront = true;
              import("@tauri-apps/api/webviewWindow").then(
                async ({ getCurrentWebviewWindow }) => {
                  const current = getCurrentWebviewWindow();
                  try {
                    const Ctx =
                      window.AudioContext || (window as any).webkitAudioContext;
                    if (Ctx && !audioCtx) audioCtx = new Ctx();
                    await audioCtx?.resume?.();
                  } catch {}
                  try {
                    await current.setAlwaysOnTop(true);
                  } catch {}
                  try {
                    await current.show();
                  } catch {}
                  try {
                    await current.unminimize?.();
                  } catch {}
                  try {
                    await current.setFocus();
                  } catch {}
                  try {
                    playBeep(300, 1200);
                  } catch {}
                  setTimeout(() => {
                    bringingToFront = false;
                  }, 350);
                }
              );
            }
          } else if (data.type === "theme" && data.theme) {
            updateOverlayTheme(data.theme);
          } else if (data.type === "state") {
            suppressBroadcast = true;
            const incomingDuration =
              typeof data.duration === "number"
                ? data.duration
                : initialSeconds;
            const incomingRemaining =
              typeof data.remaining === "number"
                ? data.remaining
                : remainingSeconds;
            const incomingStartAt =
              typeof data.startAtMs === "number" ? data.startAtMs : null;
            const incomingRunning = !!data.isRunning;
            initialSeconds = incomingDuration;
            runDurationSeconds = incomingDuration;
            if (incomingStartAt && incomingRunning) {
              startAtMs = incomingStartAt;
              isRunning = true;
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
                  const p = currentPercent();
                  const z = zoneFromPercent(p);
                  if (z !== prevZone) {
                    prevZone = z;
                  }
                }
                if (remainingSeconds === 0) {
                  stopDangerSound();
                  pause();
                  hasEnded = true;
                }
              }, 1000);
            } else {
              isRunning = false;
              startAtMs = null;
              if (typeof incomingRemaining === "number")
                remainingSeconds = incomingRemaining;
              else remainingSeconds = initialSeconds;
              if (intervalId !== null) {
                clearInterval(intervalId);
                intervalId = null;
              }
            }
            if (data.critical != null) criticalPercent = data.critical;
            if (data.danger != null) dangerPercent = data.danger;
            if (data.colors) {
              dangerColor = data.colors.dangerColor ?? dangerColor;
              criticalColor = data.colors.criticalColor ?? criticalColor;
            }
            if (Array.isArray(data.messages)) userMessages = data.messages;
            suppressBroadcast = false;
          }
        })();
      };
      try {
        bc.postMessage({ type: "request_state" });
      } catch {}
    }

    const localTheme = localStorage.getItem("colorTheme");
    if (localTheme) updateOverlayTheme(localTheme);
    else if (document.body.classList.contains("theme-light"))
      updateOverlayTheme("light");
    else updateOverlayTheme("dark");
  });

  function recordResult(win: boolean) {
    if (resultRecorded !== false) return;
    resultRecorded = true;
    try {
      bc?.postMessage({
        source: "overlay",
        type: "result",
        ts: Date.now(),
        result: win ? "win" : "loss",
      });
    } catch {}
    hasEnded = true;
    const isTauri =
      typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
    if (isTauri) {
      (async () => {
        try {
          const { getCurrentWebviewWindow } = await import(
            "@tauri-apps/api/webviewWindow"
          );
          const current = getCurrentWebviewWindow();
          await current.setSize(new LogicalSize(260, 140));
        } catch {}
      })();
    }
  }
</script>

<main
  class="relative flex flex-col gap-1.5 items-center justify-center w-full h-full px-2 py-1.5 overflow-hidden [-webkit-app-region:drag] text-center {overlayMode !== 'progressbar' ? (zoneFromPercent(currentPercent()).toLowerCase() === 'ok' ? 'bg-transparent' : zoneFromPercent(currentPercent()).toLowerCase() === 'danger' ? '[background:color-mix(in_srgb,var(--danger)_10%,transparent)]' : zoneFromPercent(currentPercent()).toLowerCase() === 'critical' ? '[background:color-mix(in_srgb,var(--critical)_12%,transparent)]' : '') : 'bg-transparent'} {remainingSeconds === 0 ? 'ended' : ''}"
  data-tauri-drag-region
  style={`--danger:${dangerColor}; --critical:${criticalColor}`}
  onpointerdown={requestOverlayGoNotOnTop}
>
  {#if overlayMode === "progressbar" && progressBarType === "full"}
    <div class="absolute left-0 top-0 bottom-0 w-full pointer-events-none [-webkit-app-region:no-drag] z-0">
      <div class="absolute left-0 top-0 bottom-0 transition-[width] duration-200 ease-linear opacity-80" style={`width: ${progressPercent()}%; background-color: ${progressBarColor};`}></div>
    </div>
  {/if}
  <div class="absolute top-1.5 left-2 right-2 z-10 flex justify-between text-xs opacity-90 [-webkit-app-region:no-drag]">
    <div>{now.toLocaleDateString()}</div>
    <div>{now.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</div>
  </div>
  <div class="relative z-10 flex flex-col items-center justify-center flex-1 w-full">
  {#if hasEnded && askOnEnd && !resultRecorded}
    <div class="text-lg font-semibold [-webkit-app-region:no-drag]">{endQuestionText}</div>
    <div class="flex gap-2.5 [-webkit-app-region:no-drag] justify-center">
      <button
        class="w-11 h-11 rounded-xl border border-[var(--card-border,#2a2a2a)] bg-black/35 text-white text-[0] grid place-items-center bg-[rgba(0,255,128,0.25)] [-webkit-app-region:no-drag]"
        onclick={() => recordResult(true)}
        title="Yes"
        aria-label="Yes"
      >
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M20.285 6.707a1 1 0 0 0-1.414-1.414L9.5 14.664l-3.371-3.37a1 1 0 1 0-1.415 1.413l4.078 4.079a1 1 0 0 0 1.415 0l10.078-10.079z" fill="currentColor" />
        </svg>
      </button>
      <button
        class="w-11 h-11 rounded-xl border border-[var(--card-border,#2a2a2a)] bg-black/35 text-white text-[0] grid place-items-center bg-[rgba(255,64,64,0.25)] [-webkit-app-region:no-drag]"
        onclick={() => recordResult(false)}
        title="No"
        aria-label="No"
      >
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M18.3 5.71a1 1 0 0 0-1.41 0L12 10.59 7.11 5.7a1 1 0 1 0-1.41 1.42L10.59 12l-4.9 4.89a1 1 0 1 0 1.41 1.42L12 13.41l4.89 4.9a1 1 0 0 0 1.41-1.42L13.41 12l4.89-4.89a1 1 0 0 0 0-1.4z" fill="currentColor" />
        </svg>
      </button>
    </div>
  {:else}
    <div
      class="text-4xl tracking-wider font-['Play',Inter,ui-sans-serif,system-ui,sans-serif] text-center w-full px-4 animate-[pulse_2s_ease-in-out_infinite] cursor-pointer {remainingSeconds === 0 ? 'animate-none' : ''} {overlayMode !== 'progressbar' ? (zoneFromPercent(currentPercent()) === 'DANGER' ? '[animation-duration:1.25s] [color:var(--danger)] [text-shadow:0_0_12px_color-mix(in_srgb,var(--danger)_50%,transparent)]' : zoneFromPercent(currentPercent()) === 'CRITICAL' ? '[animation-duration:0.75s] [color:var(--critical)] [text-shadow:0_0_14px_color-mix(in_srgb,var(--critical)_55%,transparent)]' : '') : ''} theme-light:[color:#17191b] theme-light:[text-shadow:none] [-webkit-app-region:no-drag]"
      role="button"
      tabindex="0"
      onclick={openTimeEditor}
      onkeydown={(e) => {
        if (e.key === "Enter" || e.key === " ") openTimeEditor();
      }}
      title="Click to edit time"
    >
      {#if editingTime}
        <div class="flex flex-col items-center gap-1">
          <input
            type="text"
            class="w-[86px] text-center text-base rounded-[7px] border border-[#8884] p-0.5"
            bind:value={editTimeStr}
            bind:this={editInputEl}
            autofocus={true}
            onblur={saveTimeEdit}
            onkeydown={(e) => {
              if (e.key === "Enter") saveTimeEdit();
              else if (e.key === "Escape") cancelTimeEdit();
            }}
          />
          {#if editError}
            <div class="text-[11px] text-[#ff5a5a]">{editError}</div>
          {/if}
        </div>
      {:else}
        {formatTime(remainingSeconds)}
      {/if}
    </div>
  {/if}
  <div class="flex gap-1.5 [-webkit-app-region:no-drag] justify-center w-full px-4 mt-2">
    {#if !isRunning && !hasEnded}
      <button class="rounded-lg border border-transparent py-1 px-2.5 text-[0.85em] font-medium font-inherit transition-[border-color] duration-250 shadow-[0_2px_2px_rgba(0,0,0,0.2)] cursor-pointer text-[#0f0f0f] bg-white hover:border-[#396cd8] active:bg-[#e8e8e8] dark:text-white dark:bg-[#0f0f0f98] dark:active:bg-[#0f0f0f69] theme-light:text-[#17191b] theme-light:bg-white theme-light:border-[#d3dbe3] theme-light:active:bg-[#e8e8e8]" onclick={start}>Resume</button>
    {:else if !hasEnded}
      <button class="rounded-lg border border-transparent py-1 px-2.5 text-[0.85em] font-medium font-inherit transition-[border-color] duration-250 shadow-[0_2px_2px_rgba(0,0,0,0.2)] cursor-pointer text-[#0f0f0f] bg-white hover:border-[#396cd8] active:bg-[#e8e8e8] dark:text-white dark:bg-[#0f0f0f98] dark:active:bg-[#0f0f0f69] theme-light:text-[#17191b] theme-light:bg-white theme-light:border-[#d3dbe3] theme-light:active:bg-[#e8e8e8]" onclick={pause}>Pause</button>
    {/if}
    {#if !hasEnded && hasStarted}
      <button class="rounded-lg border border-transparent py-1 px-2.5 text-[0.85em] font-medium font-inherit transition-[border-color] duration-250 shadow-[0_2px_2px_rgba(0,0,0,0.2)] cursor-pointer text-[#0f0f0f] bg-white hover:border-[#396cd8] active:bg-[#e8e8e8] dark:text-white dark:bg-[#0f0f0f98] dark:active:bg-[#0f0f0f69] theme-light:text-[#17191b] theme-light:bg-white theme-light:border-[#d3dbe3] theme-light:active:bg-[#e8e8e8]" onclick={() => { pause(); reset(); }}>Stop</button>
    {/if}
    <button class="rounded-lg border border-transparent py-1 px-2.5 text-[0.85em] font-medium font-inherit transition-[border-color] duration-250 shadow-[0_2px_2px_rgba(0,0,0,0.2)] cursor-pointer text-[#0f0f0f] bg-white hover:border-[#396cd8] active:bg-[#e8e8e8] dark:text-white dark:bg-[#0f0f0f98] dark:active:bg-[#0f0f0f69] theme-light:text-[#17191b] theme-light:bg-white theme-light:border-[#d3dbe3] theme-light:active:bg-[#e8e8e8]" onclick={() => { reset(); start(); }}>Restart</button>
  </div>
  </div>
</main>

