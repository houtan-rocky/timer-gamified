<script lang="ts">
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

  function formatTime(totalSeconds: number) {
    const m = Math.floor(totalSeconds / 60).toString().padStart(2, "0");
    const s = Math.floor(totalSeconds % 60).toString().padStart(2, "0");
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

  function minutes(secs: number) { return Math.round(secs / 60); }

  async function openOverlayAndStart() {
    const isTauri = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__;
    const params = new URLSearchParams({
      duration: String(selectedSeconds),
      autostart: "1",
    });
    if (!isTauri) {
      // Web fallback: open Picture-in-Picture mini overlay
      try {
        await openPictureInPicture(selectedSeconds);
      } catch (err) {
        // As a last resort, open overlay page (same tab to avoid popup blockers)
        const url = `/overlay?${params.toString()}`;
        location.href = url;
      }
      return;
    }
    const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const existing = await WebviewWindow.getByLabel("overlay");
    if (existing) {
      await existing.setAlwaysOnTop(true);
      await existing.setFocus();
      await existing.emit("overlay:navigate", `/overlay?${params.toString()}`);
      await existing.show();
      // @ts-ignore center may not be typed
      await existing.center?.();
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
    });
    try {
      await new Promise<void>((resolve, reject) => {
        w.once('tauri://created', () => resolve());
        w.once('tauri://error', (e) => reject(e));
      });
      await w.setAlwaysOnTop(true);
      await w.setFocus();
      await w.show();
      // @ts-ignore center may not be typed
      await w.center?.();
    } catch (e) {
      console.error('Failed to show overlay window', e);
    }
  }

  async function openPictureInPicture(totalSeconds: number) {
    // Create a canvas we can stream into PiP
    const width = 360, height = 200;
    const canvas = document.createElement('canvas');
    canvas.width = width; canvas.height = height;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('Canvas 2D context not available');

    // Create a video from the canvas stream
    const stream = (canvas as HTMLCanvasElement).captureStream?.(30);
    if (!stream) throw new Error('captureStream not supported');
    const video = document.createElement('video');
    video.muted = true; // required by autoplay policies
    (video as any).disablePictureInPicture = false;
    video.srcObject = stream;
    video.playsInline = true;
    video.style.position = 'fixed';
    video.style.width = '1px';
    video.style.height = '1px';
    video.style.opacity = '0';
    video.style.pointerEvents = 'none';
    document.body.appendChild(video);
    await video.play();

    // Start the countdown locally for the PiP view
    const startTs = Date.now();
    const endTs = startTs + totalSeconds * 1000;

    function formatTime(sec: number) {
      const m = Math.floor(sec / 60).toString().padStart(2, '0');
      const s = Math.max(0, Math.floor(sec % 60)).toString().padStart(2, '0');
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
      ctx.fillStyle = '#0b0b0b';
      ctx.fillRect(0, 0, width, height);

      // time text
      ctx.fillStyle = '#e8e8e8';
      ctx.font = 'bold 42px Inter, system-ui, -apple-system, sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(formatTime(remainingSec), width / 2, height / 2 - 16);

      // progress background
      const barW = width - 40, barH = 10, barX = 20, barY = height - 40;
      ctx.fillStyle = '#242424';
      ctx.fillRect(barX, barY, barW, barH);

      // progress fill (light green)
      ctx.fillStyle = '#8ef59b';
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
      if (document.pictureInPictureEnabled && (video as any).requestPictureInPicture) {
        // @ts-ignore
        pipWindow = await (video as any).requestPictureInPicture();
      } else if ((video as any).webkitSetPresentationMode) {
        // Safari
        (video as any).webkitSetPresentationMode('picture-in-picture');
        pipWindow = true;
      } else {
        throw new Error('Picture-in-Picture not supported');
      }
    } catch (e) {
      // Clean up before rethrow to trigger fallback
      video.pause();
      (video.srcObject as MediaStream)?.getTracks().forEach(t => t.stop());
      video.srcObject = null;
      video.remove();
      throw e;
    }

    // Clean up when PiP closes
    const onLeave = () => {
      cancelAnimationFrame(rafId);
      video.pause();
      (video.srcObject as MediaStream)?.getTracks().forEach(t => t.stop());
      video.srcObject = null;
      video.remove();
      (document as any).removeEventListener('leavepictureinpicture', onLeave);
    };
    (document as any).addEventListener('leavepictureinpicture', onLeave, { once: true });

    // Also start the main timer if not already running
    if (!isRunning) {
      remainingSeconds = totalSeconds;
      start();
    }

    return pipWindow;
  }
</script>

<main class="wrap">
  <h1>Timer</h1>

  <div class="presets">
    {#each presets as p, i}
      <div class="preset card">
        <button class="preset-btn bordered {selectedSeconds === p ? 'selected' : ''}" onclick={() => setPreset(p)}>
          <span class="time">{Math.floor(p/3600) ? `${Math.floor(p/3600)}h ` : ''}{Math.floor((p%3600)/60)}m{p%60 ? ` ${p%60}s` : ''}</span>
          <span class="hint muted">click to select</span>
        </button>
        <div class="actions">
          <button class="icon edit" title="Edit" onclick={() => openEdit(i)} aria-label="Edit preset">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25z" fill="currentColor"/>
              <path d="M20.71 7.04a1.003 1.003 0 0 0 0-1.42l-2.34-2.34a1.003 1.003 0 0 0-1.42 0l-1.83 1.83 3.75 3.75 1.84-1.82z" fill="currentColor"/>
            </svg>
          </button>
          <button class="icon remove" title="Remove" onclick={() => removePresetAt(i)} aria-label="Remove preset">×</button>
        </div>
      </div>
    {/each}
    <div class="preset add card">
      <button class="preset-btn bordered" onclick={() => { editHours=0; editMinutes=5; editSeconds=0; editOpen=true; }}>
        + Add preset
      </button>
    </div>
  </div>

  <div class="display">{formatTime(remainingSeconds)}</div>

  <div class="controls">
    {#if !isRunning}
      <Button variant="primary" onclick={start}>▶ Start</Button>
    {:else}
      <Button variant="secondary" onclick={stop}>Pause</Button>
    {/if}
    <Button variant="ghost" onclick={reset}>Reset</Button>
    <Button variant="ghost" onclick={openOverlayAndStart} title="Always-on-top widget">View mode</Button>
  </div>

  <Modal open={editOpen} title="Edit time" subtitle="Adjust hours, minutes and seconds" onclose={closeEdit}>
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
    <div class="row modal-actions" style="justify-content:flex-end; margin-top: 12px;">
      {#if editingIndex !== null}
        <Button variant="ghost" onclick={closeEdit}>Cancel</Button>
        <Button variant="primary" onclick={saveEdit}>Save</Button>
      {:else}
        <Button variant="ghost" onclick={closeEdit}>Cancel</Button>
        <Button variant="primary" onclick={addFromEdit}>Add</Button>
      {/if}
    </div>
  </Modal>
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

.presets { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 16px; }
.preset { position: relative; padding: 8px; }
.preset-btn {
  width: 100%;
  padding: 16px;
  text-align: left;
  background: transparent;
  color: var(--text);
  position: relative;
  z-index: 1;
}
.preset-btn.selected { border-color: var(--primary); }
.preset .actions { position: absolute; top: 8px; right: 8px; display: inline-flex; gap: 6px; opacity: 0; pointer-events: none; transition: opacity .15s ease; z-index: 2; }
.preset:hover .actions, .preset:focus-within .actions { opacity: 1; pointer-events: auto; }
.icon { background: rgba(0,0,0,0.35); color: var(--text); border: 1px solid var(--card-border); border-radius: 8px; padding: 4px 8px; cursor: pointer; }
.icon.edit:hover { color: var(--text); }
.icon.remove:hover { color: #ff6b6b; }
.time { font-size: 18px; font-weight: 600; }
.hint { font-size: 12px; }

.display {
  font-size: 64px;
  text-align: center;
  letter-spacing: 2px;
}

.controls { display: flex; gap: 12px; flex-wrap: wrap; }
.selects select { background: #0f0f0f; color: var(--text); border: 1px solid var(--card-border); border-radius: 8px; padding: 8px; }

@media (prefers-color-scheme: dark) {}
</style>
