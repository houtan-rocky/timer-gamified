<script lang="ts">
  import { onMount } from "svelte";
  import { LogicalPosition, LogicalSize } from "@tauri-apps/api/dpi";
  import Input from "../../lib/components/Input.svelte";

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
  let endQuestionText = $state("Win?");
  let editingTime = $state(false);
  let editTimeStr = $state("");
  let editError = $state("");
  let editInputEl: any = null;
  let overlayAlwaysOnTop = false;
  let colorTheme = "dark";
  let bringingToFront = false;
  let overlayMode = $state("normal");
  let progressBarColor = $state("#8ef59b");
  let progressBarFinishedColor = $state("#ff0000");
  let progressBarType = $state("full");
  let overlayBackgroundImage = $state<string | null>(null);
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
        resultRecorded = false; // Reset so question can show
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
    const progressBarFinishedColorParam = url.searchParams.get("progressBarFinishedColor");
    const progressBarTypeParam = url.searchParams.get("progressBarType");
    const overlayBackgroundImageParam = url.searchParams.get("overlayBackgroundImage");
    overlayAlwaysOnTop =
      alwaysOnTopParam === "1" || alwaysOnTopParam === "true";
    if (overlayModeParam) overlayMode = overlayModeParam;
    if (progressBarColorParam) progressBarColor = progressBarColorParam;
    if (progressBarFinishedColorParam) progressBarFinishedColor = progressBarFinishedColorParam;
    if (progressBarTypeParam) progressBarType = progressBarTypeParam;
    if (overlayBackgroundImageParam && overlayBackgroundImageParam.length > 0) {
      try {
        overlayBackgroundImage = decodeURIComponent(overlayBackgroundImageParam);
      } catch {
        // If decoding fails, try direct assignment (may already be decoded)
        overlayBackgroundImage = overlayBackgroundImageParam;
      }
    }

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
          // Notify main window that overlay is closing
          try {
            bc?.postMessage({ source: "overlay", type: "close" });
          } catch {}
          // Allow the close to proceed (don't prevent default)
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
      // Notify main window that overlay is open
      try {
        bc.postMessage({ source: "overlay", type: "overlay_opened" });
      } catch {}
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
            if (typeof data.progressBarFinishedColor === "string") {
              progressBarFinishedColor = data.progressBarFinishedColor;
            }
            if (typeof data.progressBarType === "string") {
              progressBarType = data.progressBarType;
            }
          } else if (data.source === "main" && data.type === "overlay_background") {
            overlayBackgroundImage = data.backgroundImage || null;
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
            // Update end question settings
            if (typeof data.askOnEnd === 'boolean') askOnEnd = data.askOnEnd;
            if (typeof data.endQuestionText === 'string') endQuestionText = data.endQuestionText;
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
                  resultRecorded = false; // Reset so question can show
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
            if (Array.isArray(data.messages)) {
              userMessages = data.messages.map((msg: any) => ({ 
                ...msg, 
                fired: msg.fired || false 
              }));
            }
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
          // Resize back to normal after answering
          await current.setSize(new LogicalSize(260, 140));
        } catch {}
      })();
    }
  }
  
  // Watch for when question should appear and resize window
  $effect(() => {
    if (hasEnded && askOnEnd && !resultRecorded) {
      const isTauri =
        typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;
      if (isTauri) {
        (async () => {
          try {
            const { getCurrentWebviewWindow } = await import(
              "@tauri-apps/api/webviewWindow"
            );
            const { LogicalSize } = await import("@tauri-apps/api/dpi");
            const current = getCurrentWebviewWindow();
            // Resize to accommodate the question and buttons
            await current.setSize(new LogicalSize(300, 200));
          } catch {}
        })();
      }
    }
  });
</script>

<main
  class="relative flex flex-col gap-1.5 items-center justify-center w-full h-full px-2 py-1.5 overflow-hidden [-webkit-app-region:drag] text-center {overlayMode !== 'progressbar' ? (zoneFromPercent(currentPercent()).toLowerCase() === 'ok' ? 'bg-transparent' : zoneFromPercent(currentPercent()).toLowerCase() === 'danger' ? '[background:color-mix(in_srgb,var(--danger)_10%,transparent)]' : zoneFromPercent(currentPercent()).toLowerCase() === 'critical' ? '[background:color-mix(in_srgb,var(--critical)_12%,transparent)]' : '') : 'bg-transparent'} {remainingSeconds === 0 ? 'ended' : ''}"
  data-tauri-drag-region
  style={`--danger:${dangerColor}; --critical:${criticalColor}; ${overlayBackgroundImage ? `background-image: url(${overlayBackgroundImage}); background-size: cover; background-position: center; background-repeat: no-repeat; background-attachment: fixed;` : ''}`}
  onpointerdown={requestOverlayGoNotOnTop}
>
  {#if overlayMode === "progressbar" && progressBarType === "full"}
    <div class="absolute left-0 top-0 bottom-0 w-full pointer-events-none [-webkit-app-region:no-drag] z-0">
      <div class="absolute left-0 top-0 bottom-0 transition-[width] duration-200 ease-linear opacity-80" style={`width: ${progressPercent()}%; background-color: ${remainingSeconds === 0 ? progressBarFinishedColor : progressBarColor};`}></div>
    </div>
  {/if}
  <!-- Header with date/time and zone indicator -->
  <div class="absolute top-0 left-0 right-0 z-20 flex items-center justify-between px-4 py-2 bg-gradient-to-b from-black/40 via-black/20 to-transparent [-webkit-app-region:no-drag]">
    <div class="flex items-center gap-2">
      <div class="text-[10px] font-medium [color:var(--color-muted)] uppercase tracking-wider">{now.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}</div>
      <div class="text-[10px] [color:var(--color-muted)]">{now.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</div>
    </div>
    {#if overlayMode !== 'progressbar' && isRunning && !hasEnded}
      {@const zone = zoneFromPercent(currentPercent())}
      <div class="px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider game-font
        {zone === 'OK' ? 'bg-[rgba(142,245,155,0.2)] border border-[rgba(142,245,155,0.4)] [color:rgba(142,245,155,0.9)] [text-shadow:0_0_8px_rgba(142,245,155,0.5)]' : ''}
        {zone === 'DANGER' ? `bg-[color-mix(in_srgb,var(--danger)_20%,transparent)] border border-[color-mix(in_srgb,var(--danger)_50%,transparent)] [color:var(--danger)] [text-shadow:0_0_10px_color-mix(in_srgb,var(--danger)_60%,transparent)] animate-[glow_1.25s_ease-in-out_infinite]` : ''}
        {zone === 'CRITICAL' ? `bg-[color-mix(in_srgb,var(--critical)_25%,transparent)] border border-[color-mix(in_srgb,var(--critical)_60%,transparent)] [color:var(--critical)] [text-shadow:0_0_12px_color-mix(in_srgb,var(--critical)_70%,transparent)] animate-[glow_0.75s_ease-in-out_infinite]` : ''}
        {!isRunning ? 'bg-[rgba(142,245,155,0.15)] border border-[rgba(142,245,155,0.3)] [color:rgba(142,245,155,0.8)]' : ''}
      ">
        {zone}
      </div>
    {/if}
  </div>

  <!-- Main content area -->
  <div class="relative z-10 flex flex-col items-center justify-center flex-1 w-full pt-8 pb-4">
  {#if hasEnded && askOnEnd && !resultRecorded}
    <div class="flex flex-col items-center gap-6 [-webkit-app-region:no-drag] animate-[slideIn_0.3s_ease-out]">
      <div class="text-2xl font-bold game-font [color:var(--color-foreground)] text-center px-6 tracking-wide">{endQuestionText}</div>
      <div class="flex gap-5 justify-center items-center">
        <button
          class="relative w-20 h-20 rounded-2xl border-2 border-[rgba(142,245,155,0.7)] bg-gradient-to-br from-[rgba(142,245,155,0.25)] to-[rgba(142,245,155,0.15)] hover:from-[rgba(142,245,155,0.35)] hover:to-[rgba(142,245,155,0.25)] hover:border-[rgba(142,245,155,0.9)] text-white grid place-items-center transition-all active:scale-95 shadow-[0_6px_20px_rgba(142,245,155,0.4)] [-webkit-app-region:no-drag] group overflow-hidden"
          onclick={() => recordResult(true)}
          title="Yes"
          aria-label="Yes"
        >
          <div class="absolute inset-0 bg-gradient-to-br from-white/20 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" class="group-hover:scale-110 transition-transform drop-shadow-[0_2px_4px_rgba(0,0,0,0.3)]">
            <path d="M20.285 6.707a1 1 0 0 0-1.414-1.414L9.5 14.664l-3.371-3.37a1 1 0 1 0-1.415 1.413l4.078 4.079a1 1 0 0 0 1.415 0l10.078-10.079z" fill="currentColor" />
          </svg>
        </button>
        <button
          class="relative w-20 h-20 rounded-2xl border-2 border-[rgba(255,68,68,0.7)] bg-gradient-to-br from-[rgba(255,68,68,0.25)] to-[rgba(255,68,68,0.15)] hover:from-[rgba(255,68,68,0.35)] hover:to-[rgba(255,68,68,0.25)] hover:border-[rgba(255,68,68,0.9)] text-white grid place-items-center transition-all active:scale-95 shadow-[0_6px_20px_rgba(255,68,68,0.4)] [-webkit-app-region:no-drag] group overflow-hidden"
          onclick={() => recordResult(false)}
          title="No"
          aria-label="No"
        >
          <div class="absolute inset-0 bg-gradient-to-br from-white/20 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" class="group-hover:scale-110 transition-transform drop-shadow-[0_2px_4px_rgba(0,0,0,0.3)]">
            <path d="M18.3 5.71a1 1 0 0 0-1.41 0L12 10.59 7.11 5.7a1 1 0 1 0-1.41 1.42L10.59 12l-4.9 4.89a1 1 0 1 0 1.41 1.42L12 13.41l4.89 4.9a1 1 0 0 0 1.41-1.42L13.41 12l4.89-4.89a1 1 0 0 0 0-1.4z" fill="currentColor" />
          </svg>
        </button>
      </div>
    </div>
  {:else}
    <!-- Timer display with enhanced styling -->
    <div class="relative flex flex-col items-center gap-3">
      <!-- Timer with glow effect and zone styling -->
      <div
        class="relative text-5xl font-bold game-font tracking-[0.1em] cursor-pointer transition-all duration-300 [-webkit-app-region:no-drag]
        {remainingSeconds === 0 ? 'animate-none opacity-90' : 'animate-[pulse_2s_ease-in-out_infinite]'} 
        {overlayMode !== 'progressbar' ? (
          zoneFromPercent(currentPercent()) === 'DANGER' 
            ? '[animation-duration:1.25s] [color:var(--danger)] [text-shadow:0_0_20px_color-mix(in_srgb,var(--danger)_70%,transparent),0_0_40px_color-mix(in_srgb,var(--danger)_40%,transparent)] drop-shadow-[0_4px_12px_color-mix(in_srgb,var(--danger)_50%,transparent)]' 
            : zoneFromPercent(currentPercent()) === 'CRITICAL' 
              ? '[animation-duration:0.75s] [color:var(--critical)] [text-shadow:0_0_24px_color-mix(in_srgb,var(--critical)_75%,transparent),0_0_50px_color-mix(in_srgb,var(--critical)_45%,transparent)] drop-shadow-[0_6px_16px_color-mix(in_srgb,var(--critical)_60%,transparent)]' 
              : '[color:var(--color-primary)] [text-shadow:0_0_16px_color-mix(in_srgb,var(--color-primary)_60%,transparent)] drop-shadow-[0_2px_8px_color-mix(in_srgb,var(--color-primary)_40%,transparent)]'
        ) : '[color:var(--color-foreground)]'} 
        theme-light:[color:#17191b] theme-light:[text-shadow:none] theme-light:drop-shadow-[0_2px_4px_rgba(0,0,0,0.1)]"
        role="button"
        tabindex="0"
        onclick={openTimeEditor}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") openTimeEditor();
        }}
        title="Click to edit time"
      >
        {#if editingTime}
          <div class="flex flex-col items-center gap-2">
            <Input
              type="text"
              class="w-[120px] text-center text-xl rounded-xl border-2 border-[rgba(142,245,155,0.5)] bg-[rgba(20,20,20,0.8)] [color:var(--color-foreground)] p-2 game-font focus:border-[rgba(142,245,155,0.8)] focus:outline-none focus:ring-2 focus:ring-[rgba(142,245,155,0.3)]"
              bind:value={editTimeStr}
              bind:editInputEl={editInputEl}
              autofocus={true}
              onblur={saveTimeEdit}
              onkeydown={(e) => {
                if (e.key === "Enter") saveTimeEdit();
                else if (e.key === "Escape") cancelTimeEdit();
              }}
            />
            {#if editError}
              <div class="text-xs text-[#ff5a5a] font-medium animate-[slideIn_0.2s_ease-out]">{editError}</div>
            {/if}
          </div>
        {:else}
          <div class="relative">
            <!-- Timer digits with enhanced styling -->
            <span class="inline-block">{formatTime(remainingSeconds)}</span>
            {#if isRunning && remainingSeconds > 0 && remainingSeconds <= 10}
              <span class="absolute inset-0 animate-[countdown-tick_1s_ease-in-out_infinite] opacity-50"></span>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Progress indicator bar (when not in progressbar mode) -->
      {#if overlayMode !== 'progressbar' && isRunning}
        <div class="w-32 h-1 bg-[rgba(255,255,255,0.1)] rounded-full overflow-hidden [-webkit-app-region:no-drag]">
          <div 
            class="h-full transition-all duration-300 ease-linear rounded-full
            {zoneFromPercent(currentPercent()) === 'OK' ? 'bg-gradient-to-r from-[var(--color-primary)] to-[color-mix(in_srgb,var(--color-primary)_80%,transparent)]' : ''}
            {zoneFromPercent(currentPercent()) === 'DANGER' ? `bg-gradient-to-r from-[var(--danger)] to-[color-mix(in_srgb,var(--danger)_80%,transparent)]` : ''}
            {zoneFromPercent(currentPercent()) === 'CRITICAL' ? `bg-gradient-to-r from-[var(--critical)] to-[color-mix(in_srgb,var(--critical)_80%,transparent)]` : ''}"
            style={`width: ${100 - currentPercent()}%`}
          ></div>
        </div>
      {/if}
    </div>

    <!-- Control buttons with connected bar styling -->
    <div class="flex [-webkit-app-region:no-drag] justify-center items-center w-full px-4 mt-3">
      <div class="flex rounded-lg overflow-hidden shadow-[0_1px_3px_rgba(0,0,0,0.2)] bg-[rgba(20,20,20,0.95)] border border-[rgba(255,255,255,0.12)] backdrop-blur-sm theme-light:bg-[rgba(255,255,255,0.95)] theme-light:border-[rgba(0,0,0,0.12)]">
        {#if !isRunning && !hasEnded}
          {#if !hasStarted}
            <!-- Start button (left, rounded left) -->
            <button 
              class="px-4 py-2 rounded-l-lg border-r border-[rgba(255,255,255,0.1)] bg-transparent hover:bg-[rgba(255,255,255,0.08)] active:bg-[rgba(255,255,255,0.12)] [color:var(--color-foreground)] text-xs font-medium uppercase tracking-wide transition-colors duration-150 theme-light:border-[rgba(0,0,0,0.1)] theme-light:hover:bg-[rgba(0,0,0,0.04)] theme-light:active:bg-[rgba(0,0,0,0.06)]" 
              onclick={start}
            >
              Start
            </button>
          {:else}
            <!-- Resume button (left, rounded left) -->
            <button 
              class="px-4 py-2 rounded-l-lg border-r border-[rgba(255,255,255,0.1)] bg-transparent hover:bg-[rgba(255,255,255,0.08)] active:bg-[rgba(255,255,255,0.12)] [color:var(--color-foreground)] text-xs font-medium uppercase tracking-wide transition-colors duration-150 theme-light:border-[rgba(0,0,0,0.1)] theme-light:hover:bg-[rgba(0,0,0,0.04)] theme-light:active:bg-[rgba(0,0,0,0.06)]" 
              onclick={start}
            >
              Resume
            </button>
          {/if}
        {:else if !hasEnded}
          <!-- Pause button (left, rounded left) -->
          <button 
            class="px-4 py-2 rounded-l-lg border-r border-[rgba(255,255,255,0.1)] bg-transparent hover:bg-[rgba(255,255,255,0.08)] active:bg-[rgba(255,255,255,0.12)] [color:var(--color-foreground)] text-xs font-medium uppercase tracking-wide transition-colors duration-150 theme-light:border-[rgba(0,0,0,0.1)] theme-light:hover:bg-[rgba(0,0,0,0.04)] theme-light:active:bg-[rgba(0,0,0,0.06)]" 
            onclick={pause}
          >
            Pause
          </button>
        {/if}
        {#if !hasEnded && hasStarted}
          <!-- Stop button (middle, square) -->
          <button 
            class="px-4 py-2 border-x border-[rgba(255,255,255,0.1)] bg-transparent hover:bg-[rgba(255,255,255,0.08)] active:bg-[rgba(255,255,255,0.12)] [color:var(--color-foreground)] text-xs font-medium uppercase tracking-wide transition-colors duration-150 theme-light:border-[rgba(0,0,0,0.1)] theme-light:hover:bg-[rgba(0,0,0,0.04)] theme-light:active:bg-[rgba(0,0,0,0.06)]" 
            onclick={() => { pause(); reset(); }}
          >
            Stop
          </button>
        {/if}
        <!-- Restart button (right, rounded right) -->
        <button 
          class="px-4 py-2 {!hasEnded && hasStarted ? 'rounded-r-lg border-l' : (!hasStarted && !isRunning ? 'rounded-r-lg border-l' : 'rounded-lg')} border-[rgba(255,255,255,0.1)] bg-transparent hover:bg-[rgba(255,255,255,0.08)] active:bg-[rgba(255,255,255,0.12)] [color:var(--color-foreground)] text-xs font-medium uppercase tracking-wide transition-colors duration-150 theme-light:border-[rgba(0,0,0,0.1)] theme-light:hover:bg-[rgba(0,0,0,0.04)] theme-light:active:bg-[rgba(0,0,0,0.06)]" 
          onclick={() => { reset(); start(); }}
        >
          Restart
        </button>
      </div>
    </div>
  {/if}
  </div>
</main>

