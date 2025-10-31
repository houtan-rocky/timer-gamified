<script lang="ts">
  let { value = $bindable<string | undefined>(), class: className = '', onvaluechange } = $props<{
    value?: string | undefined;
    class?: string;
    onvaluechange?: (value: string | undefined) => void;
  }>();

  let audioEl = $state<HTMLAudioElement | null>(null);
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let startTime = $state(0);
  let endTime = $state(0);
  let loadedAudioUrl = $state<string | null>(null);
  let fileInput = $state<HTMLInputElement | null>(null);

  function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  }

  function handleFileChange(e: Event) {
    const file = (e.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      value = result;
      onvaluechange?.(result);
      loadAudio(result);
    };
    reader.readAsDataURL(file);
  }

  function loadAudio(url: string) {
    if (loadedAudioUrl && loadedAudioUrl !== url) {
      URL.revokeObjectURL(loadedAudioUrl);
    }
    
    // If it's already a data URL, use it directly
    if (url.startsWith('data:')) {
      loadedAudioUrl = url;
      if (audioEl) {
        audioEl.src = url;
        audioEl.load();
      }
      // Reset trim points
      startTime = 0;
      endTime = 0;
    }
  }


  $effect(() => {
    if (value && audioEl) {
      loadAudio(value);
    } else if (!value && audioEl) {
      audioEl.src = '';
      duration = 0;
      currentTime = 0;
      startTime = 0;
      endTime = 0;
    }
    // Initialize audio element when it becomes available
    if (audioEl && value && !audioEl.src) {
      loadAudio(value);
    }
  });

  function togglePlay() {
    if (!audioEl || !value) return;
    
    if (isPlaying) {
      audioEl.pause();
      isPlaying = false;
    } else {
      // If we're before start time, jump to start
      if (startTime > 0 && audioEl.currentTime < startTime) {
        audioEl.currentTime = startTime;
      }
      audioEl.play().catch(() => {
        isPlaying = false;
      });
      isPlaying = true;
    }
  }

  function seek(seconds: number) {
    if (!audioEl) return;
    const newTime = Math.max(0, Math.min(duration, audioEl.currentTime + seconds));
    audioEl.currentTime = newTime;
  }

  function setStartTime() {
    if (!audioEl) return;
    startTime = audioEl.currentTime;
    if (startTime >= endTime && endTime > 0) {
      endTime = Math.min(duration, startTime + 0.1);
    }
    audioEl.currentTime = startTime;
    applyTrim();
  }

  function setEndTime() {
    if (!audioEl) return;
    endTime = audioEl.currentTime;
    if (endTime <= startTime) {
      startTime = Math.max(0, endTime - 0.1);
    }
    audioEl.pause();
    isPlaying = false;
    audioEl.currentTime = startTime;
    applyTrim();
  }

  function resetTrim() {
    startTime = 0;
    endTime = 0;
    if (audioEl && value) {
      loadAudio(value);
    }
  }

  function applyTrim() {
    // Trim is applied conceptually - the actual trimming would require
    // creating a new audio buffer, which is complex. For now, we just
    // track the trim points and apply them during playback.
    // The full audio is still stored, but playback respects trim points.
  }

  function removeSound() {
    if (loadedAudioUrl && loadedAudioUrl.startsWith('blob:')) {
      URL.revokeObjectURL(loadedAudioUrl);
    }
    value = undefined;
    onvaluechange?.(undefined);
    loadedAudioUrl = null;
    if (audioEl) {
      audioEl.src = '';
      duration = 0;
      currentTime = 0;
      startTime = 0;
      endTime = 0;
    }
    if (fileInput) {
      fileInput.value = '';
    }
  }

  function seekTo(percent: number) {
    if (!audioEl || duration === 0) return;
    const targetTime = (percent / 100) * duration;
    audioEl.currentTime = Math.max(startTime, Math.min(endTime || duration, targetTime));
  }
</script>

<div class="flex flex-col gap-2 {className}">
  <label class="flex flex-col gap-2">
    <span class="[color:var(--color-muted)] text-sm">Sound file</span>
    <input
      bind:this={fileInput}
      type="file"
      accept="audio/*"
      onchange={handleFileChange}
      class="text-sm [color:var(--color-foreground)] [background-color:var(--color-card)] border [border-color:var(--color-card-border)] rounded-lg px-3 py-2 file:mr-4 file:py-1 file:px-3 file:rounded-lg file:border-0 file:text-sm file:font-semibold file:[background-color:var(--color-primary)] file:text-[#05210c] hover:file:opacity-90 cursor-pointer"
    />
  </label>

  {#if value}
    <div class="flex flex-col gap-3 p-3 border rounded-lg [border-color:var(--color-card-border)] [background-color:var(--color-card)]">
      <audio bind:this={audioEl} preload="metadata" onloadedmetadata={() => { if (audioEl) { duration = audioEl.duration; if (endTime === 0 || endTime > duration) endTime = duration; } }} ontimeupdate={() => { if (audioEl) { currentTime = audioEl.currentTime; if (endTime > 0 && audioEl.currentTime >= endTime) { audioEl.pause(); isPlaying = false; audioEl.currentTime = startTime; } } }} onended={() => { isPlaying = false; if (audioEl) audioEl.currentTime = startTime; }}></audio>
      
      <!-- Progress bar -->
      <div class="flex flex-col gap-1">
          <button
            type="button"
            class="w-full h-2 bg-[rgba(255,255,255,0.1)] rounded-full cursor-pointer relative overflow-hidden border-0 p-0"
            onclick={(e) => {
              const rect = e.currentTarget.getBoundingClientRect();
              const percent = ((e.clientX - rect.left) / rect.width) * 100;
              seekTo(percent);
            }}
            onkeydown={(e) => {
              if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
                e.preventDefault();
                seek(e.key === 'ArrowLeft' ? -1 : 1);
              }
            }}
            aria-label="Seek audio"
          >
          <div
            class="absolute top-0 left-0 h-full [background-color:var(--color-primary)] transition-all duration-100"
            style={`width: ${duration > 0 ? (currentTime / duration) * 100 : 0}%`}
          ></div>
          {#if startTime > 0}
            <div
              class="absolute top-0 h-full bg-[rgba(255,255,0,0.3)]"
              style={`left: ${(startTime / duration) * 100}%; width: ${((endTime || duration) - startTime) / duration * 100}%`}
              title="Selected section"
            ></div>
          {/if}
        </button>
        <div class="flex justify-between text-xs [color:var(--color-muted)]">
          <span>{formatTime(currentTime)}</span>
          <span>{formatTime(duration)}</span>
        </div>
      </div>

      <!-- Controls -->
      <div class="flex items-center gap-2 flex-wrap">
        <button
          class="px-3 py-1.5 text-sm rounded-lg border [border-color:var(--color-card-border)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors"
          onclick={togglePlay}
          title={isPlaying ? "Pause" : "Play"}
        >
          {#if isPlaying}
            ⏸ Pause
          {:else}
            ▶ Play
          {/if}
        </button>
        
        <button
          class="px-2 py-1.5 text-sm rounded-lg border [border-color:var(--color-card-border)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors"
          onclick={() => seek(-5)}
          title="Rewind 5 seconds"
        >
          ⏪ -5s
        </button>
        
        <button
          class="px-2 py-1.5 text-sm rounded-lg border [border-color:var(--color-card-border)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors"
          onclick={() => seek(5)}
          title="Forward 5 seconds"
        >
          +5s ⏩
        </button>

        <div class="flex-1"></div>

        <button
          class="px-2 py-1.5 text-xs rounded-lg border [border-color:var(--color-card-border)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors"
          onclick={setStartTime}
          title="Set start point"
        >
          ⬅ Set start
        </button>
        
        <button
          class="px-2 py-1.5 text-xs rounded-lg border [border-color:var(--color-card-border)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors"
          onclick={setEndTime}
          title="Set end point"
        >
          Set end ➡
        </button>

        {#if startTime > 0 || endTime < duration}
          <button
            class="px-2 py-1.5 text-xs rounded-lg border [border-color:var(--color-card-border)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors"
            onclick={resetTrim}
            title="Reset trim"
          >
            Reset
          </button>
        {/if}
      </div>

      {#if startTime > 0 || (endTime > 0 && endTime < duration)}
        <div class="text-xs [color:var(--color-muted)] px-2 py-1 bg-[rgba(255,255,0,0.1)] rounded">
          Selected: {formatTime(startTime)} - {formatTime(endTime || duration)}
        </div>
      {/if}

      <button
        class="px-3 py-1.5 text-sm rounded-lg border [border-color:var(--color-card-border)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors text-red-400 hover:text-red-300"
        onclick={removeSound}
      >
        Remove sound
      </button>
    </div>
  {/if}
</div>

