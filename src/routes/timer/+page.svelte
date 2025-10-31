<script lang="ts">
  // Tauri API will be imported dynamically only when available
  import Input from "../lib/components/Input.svelte";

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
      autostart: "0",
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
      x: 0,
      y: 0,
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

<main class="max-w-[520px] mx-auto px-4 py-8 flex flex-col gap-4">
  <h1 class="text-4xl font-bold">Timer</h1>

  <div class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-x-3 gap-y-2">
    {#each presets as p}
      <div class="flex gap-1.5 items-center">
        <button class="rounded-lg border border-transparent py-2.5 px-5 text-base font-medium font-inherit transition-[border-color] duration-250 shadow-[0_2px_2px_rgba(0,0,0,0.2)] cursor-pointer text-[#0f0f0f] bg-white hover:border-[#396cd8] active:bg-[#e8e8e8] dark:text-white dark:bg-[#0f0f0f98] dark:active:bg-[#0f0f0f69] {selectedSeconds === p ? 'border-[#396cd8]' : ''}" onclick={() => setPreset(p)}>
          {minutes(p)} min
        </button>
        <div class="flex gap-1.5 items-center">
          <Input type="number" min={1} value={minutes(p)} oninput={(e) => updatePreset(p, Number((e.currentTarget as HTMLInputElement).value))} class="w-16 px-2 py-1" />
          <button class="rounded-lg border border-transparent py-1.5 px-3 text-base font-medium font-inherit transition-[border-color] duration-250 shadow-[0_2px_2px_rgba(0,0,0,0.2)] cursor-pointer text-[#0f0f0f] bg-white hover:border-[#396cd8] active:bg-[#e8e8e8] dark:text-white dark:bg-[#0f0f0f98] dark:active:bg-[#0f0f0f69]" title="Remove" onclick={() => removePreset(p)}>×</button>
        </div>
      </div>
    {/each}
  </div>

  <div class="flex gap-2 items-center">
    <Input type="number" min={1} bind:value={newPresetMinutes} class="w-20 px-2 py-1" />
    <button class="rounded-lg border border-transparent py-2.5 px-5 text-base font-medium font-inherit transition-[border-color] duration-250 shadow-[0_2px_2px_rgba(0,0,0,0.2)] cursor-pointer text-[#0f0f0f] bg-white hover:border-[#396cd8] active:bg-[#e8e8e8] dark:text-white dark:bg-[#0f0f0f98] dark:active:bg-[#0f0f0f69]" onclick={addPreset}>Add preset (min)</button>
  </div>

  <div class="text-6xl text-center tracking-wider font-['Play',Inter,ui-sans-serif,system-ui,sans-serif]">{formatTime(remainingSeconds)}</div>

  <div class="flex gap-2 flex-wrap p-2">
    {#if !isRunning}
      <button class="rounded-lg border border-transparent py-2.5 px-5 text-base font-medium font-inherit transition-[border-color] duration-250 shadow-[0_2px_2px_rgba(0,0,0,0.2)] cursor-pointer text-[#0f0f0f] bg-white hover:border-[#396cd8] active:bg-[#e8e8e8] dark:text-white dark:bg-[#0f0f0f98] dark:active:bg-[#0f0f0f69]" onclick={start}>Start</button>
    {:else}
      <button class="rounded-lg border border-transparent py-2.5 px-5 text-base font-medium font-inherit transition-[border-color] duration-250 shadow-[0_2px_2px_rgba(0,0,0,0.2)] cursor-pointer text-[#0f0f0f] bg-white hover:border-[#396cd8] active:bg-[#e8e8e8] dark:text-white dark:bg-[#0f0f0f98] dark:active:bg-[#0f0f0f69]" onclick={stop}>Pause</button>
    {/if}
    <button class="rounded-lg border border-transparent py-2.5 px-5 text-base font-medium font-inherit transition-[border-color] duration-250 shadow-[0_2px_2px_rgba(0,0,0,0.2)] cursor-pointer text-[#0f0f0f] bg-white hover:border-[#396cd8] active:bg-[#e8e8e8] dark:text-white dark:bg-[#0f0f0f98] dark:active:bg-[#0f0f0f69]" onclick={reset}>Reset</button>
    <button class="rounded-lg border border-transparent py-2.5 px-5 text-base font-medium font-inherit transition-[border-color] duration-250 shadow-[0_2px_2px_rgba(0,0,0,0.2)] cursor-pointer text-[#0f0f0f] bg-white hover:border-[#396cd8] active:bg-[#e8e8e8] dark:text-white dark:bg-[#0f0f0f98] dark:active:bg-[#0f0f0f69] ml-auto" onclick={openOverlayAndStart}>View mode</button>
  </div>
</main>


