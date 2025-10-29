<script lang="ts">
  import { onMount } from "svelte";

  let initialSeconds = $state(60);
  let remainingSeconds = $state(initialSeconds);
  let isRunning = $state(false);
  let intervalId: number | null = null;

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
    intervalId = window.setInterval(() => {
      if (remainingSeconds > 0) {
        remainingSeconds -= 1;
      } else {
        stop();
      }
    }, 1000);
  }

  function stop() {
    isRunning = false;
    if (intervalId !== null) {
      clearInterval(intervalId);
      intervalId = null;
    }
  }

  function reset() {
    stop();
    remainingSeconds = initialSeconds;
  }

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
    if (autostart === "1") start();

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
  });
</script>

<main class="overlay" data-tauri-drag-region>
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

.time {
  font-size: 40px;
  letter-spacing: 2px;
}

.bar { width: 90%; height: 8px; background: #242424; border-radius: 6px; overflow: hidden; -webkit-app-region: no-drag; }
.bar .fill { height: 100%; background: #8ef59b; transition: width .2s ease; }

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


