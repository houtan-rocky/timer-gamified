<script lang="ts">
  // Tauri API will be imported dynamically only when available

  let presets = $state<number[]>([120, 600, 1200]);
  let selectedSeconds = $state(120);
  let remainingSeconds = $state(selectedSeconds);
  let isRunning = $state(false);
  let intervalId: number | null = null;
  let newPresetMinutes = $state(5);

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
  }

  function start() {
    if (isRunning) return;
    isRunning = true;
    if (remainingSeconds <= 0) remainingSeconds = selectedSeconds;
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
    remainingSeconds = selectedSeconds;
  }

  function minutes(secs: number) { return Math.round(secs / 60); }

  function addPreset() {
    const secs = Math.max(1, Math.floor(newPresetMinutes)) * 60;
    presets = Array.from(new Set([...presets, secs])).sort((a, b) => a - b);
    newPresetMinutes = 5;
  }

  function removePreset(secs: number) {
    presets = presets.filter((p) => p !== secs);
    if (selectedSeconds === secs) {
      const fallback = presets[0] ?? 60;
      setPreset(fallback);
    }
  }

  function updatePreset(oldSecs: number, newMinutes: number) {
    const secs = Math.max(1, Math.floor(newMinutes)) * 60;
    presets = presets
      .map((p) => (p === oldSecs ? secs : p))
      .filter((v, i, arr) => arr.indexOf(v) === i)
      .sort((a, b) => a - b);
    if (selectedSeconds === oldSecs) setPreset(secs);
  }

  async function openOverlayAndStart() {
    const isTauri = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__;
    // Ensure we use currently selected duration, and request auto-start
    const params = new URLSearchParams({
      duration: String(selectedSeconds),
      autostart: "1",
    });
    if (!isTauri) {
      const url = `${location.origin}/overlay?${params.toString()}`;
      window.open(url, "_blank", "noopener,noreferrer");
      return;
    }
    const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const existing = await WebviewWindow.getByLabel("overlay");
    if (existing) {
      // If overlay exists, just bring to front and navigate with params
      await existing.setFocus();
      await existing.emit("overlay:navigate", `/overlay?${params.toString()}`);
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
    });
    try {
      await new Promise<void>((resolve, reject) => {
        w.once('tauri://created', () => resolve());
        w.once('tauri://error', (e) => reject(e));
      });
      await w.setAlwaysOnTop(true);
      await w.setFocus();
      await w.show();
      await w.center?.();
    } catch {}
  }
</script>

<main class="wrap">
  <h1>Timer</h1>

  <div class="presets">
    {#each presets as p}
      <div class="preset-item">
        <button class:selected={selectedSeconds === p} onclick={() => setPreset(p)}>
          {minutes(p)} min
        </button>
        <div class="edit">
          <input type="number" min="1" value={minutes(p)} oninput={(e) => updatePreset(p, Number(e.currentTarget.value))} />
          <button class="remove" title="Remove" onclick={() => removePreset(p)}>×</button>
        </div>
      </div>
    {/each}
  </div>

  <div class="add">
    <input type="number" min="1" bind:value={newPresetMinutes} />
    <button onclick={addPreset}>Add preset (min)</button>
  </div>

  <div class="display">{formatTime(remainingSeconds)}</div>

  <div class="controls">
    {#if !isRunning}
      <button onclick={start}>Start</button>
    {:else}
      <button onclick={stop}>Pause</button>
    {/if}
    <button onclick={reset}>Reset</button>
    <button class="view" onclick={openOverlayAndStart}>View mode</button>
  </div>
</main>

<style>
.wrap {
  max-width: 520px;
  margin: 0 auto;
  padding: 32px 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.presets { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 8px 12px; }
.preset-item { display: flex; gap: 6px; align-items: center; }
.preset-item .edit { display: flex; gap: 6px; align-items: center; }
.preset-item .edit input { width: 64px; }

button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
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

button.selected {
  border-color: #396cd8;
}

.display {
  font-size: 64px;
  text-align: center;
  letter-spacing: 2px;
}

.controls { display: flex; gap: 8px; flex-wrap: wrap; }
.controls .view { margin-left: auto; }
.add { display: flex; gap: 8px; align-items: center; }
.remove { padding: 0.4em 0.8em; }

@media (prefers-color-scheme: dark) {
  button { color: #ffffff; background-color: #0f0f0f98; }
  button:active { background-color: #0f0f0f69; }
}
</style>


