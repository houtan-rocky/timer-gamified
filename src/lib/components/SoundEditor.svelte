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

  async function handleFileChange(e: Event) {
    const file = (e.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    
    const reader = new FileReader();
    reader.onload = async () => {
      const result = reader.result as string;
      // Extract base64 content
      const base64Content = result.split(',')[1];
      const binaryData = Uint8Array.from(atob(base64Content), c => c.charCodeAt(0));
      
      // Upload to backend
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const hash = await invoke<string>('upload_sound', { data: Array.from(binaryData) });
        value = hash;
        onvaluechange?.(hash);
        await loadAudio(hash);
      } catch (e) {
        console.error('Failed to upload sound:', e);
        // Fallback to base64 if backend fails
        value = result;
        onvaluechange?.(result);
        loadAudioFromDataUrl(result);
      }
    };
    reader.readAsDataURL(file);
  }

  async function loadAudio(hashOrUrl: string) {
    if (loadedAudioUrl && loadedAudioUrl !== hashOrUrl) {
      URL.revokeObjectURL(loadedAudioUrl);
    }
    
    // Check if it's a hash (64 hex chars) or data URL
    if (hashOrUrl.startsWith('data:')) {
      loadAudioFromDataUrl(hashOrUrl);
    } else if (/^[a-f0-9]{64}$/i.test(hashOrUrl)) {
      // It's a hash, load from backend
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const data = await invoke<number[]>('get_sound', { hash: hashOrUrl });
        const blob = new Blob([new Uint8Array(data)], { type: 'audio/wav' });
        const url = URL.createObjectURL(blob);
        loadedAudioUrl = url;
        if (audioEl) {
          audioEl.src = url;
          audioEl.load();
        }
        // Reset trim points
        startTime = 0;
        endTime = 0;
      } catch (e) {
        console.error('Failed to load sound from backend:', e);
      }
    } else {
      loadAudioFromDataUrl(hashOrUrl);
    }
  }

  function loadAudioFromDataUrl(url: string) {
    if (loadedAudioUrl && loadedAudioUrl !== url && loadedAudioUrl.startsWith('blob:')) {
      URL.revokeObjectURL(loadedAudioUrl);
    }
    
    if (url.startsWith('data:')) {
      loadedAudioUrl = url;
      if (audioEl) {
        audioEl.src = url;
        audioEl.load();
      }
    } else {
      loadedAudioUrl = url;
      if (audioEl) {
        audioEl.src = url;
        audioEl.load();
      }
    }
    // Reset trim points
    startTime = 0;
    endTime = 0;
  }


  let loadTimeout: number | null = null;
  let hasLoadedInitial = $state(false);
  let shouldLoad = $state(false);
  
  // Only load audio when explicitly needed (user interaction or component becomes visible)
  function requestLoad() {
    if (value && !hasLoadedInitial) {
      shouldLoad = true;
      hasLoadedInitial = true;
    }
  }
  
  $effect(() => {
    // Clear any pending loads
    if (loadTimeout) {
      clearTimeout(loadTimeout);
      loadTimeout = null;
    }
    
    // Only load if requested and audio element exists
    if (shouldLoad && value && audioEl) {
      loadTimeout = window.setTimeout(() => {
        loadAudio(value);
        shouldLoad = false;
      }, 150);
    } else if (!value && audioEl) {
      audioEl.src = '';
      duration = 0;
      currentTime = 0;
      startTime = 0;
      endTime = 0;
      hasLoadedInitial = false;
      shouldLoad = false;
    }
    
    return () => {
      if (loadTimeout) {
        clearTimeout(loadTimeout);
        loadTimeout = null;
      }
    };
  });
  
  // Load audio when audio element becomes available (lazy)
  $effect(() => {
    if (audioEl && value && !hasLoadedInitial) {
      // Only load when audio element is interacted with or explicitly requested
      const loadOnInteraction = () => {
        requestLoad();
        audioEl?.removeEventListener('click', loadOnInteraction);
        audioEl?.removeEventListener('mouseenter', loadOnInteraction);
      };
      audioEl.addEventListener('click', loadOnInteraction);
      audioEl.addEventListener('mouseenter', loadOnInteraction);
      
      return () => {
        audioEl?.removeEventListener('click', loadOnInteraction);
        audioEl?.removeEventListener('mouseenter', loadOnInteraction);
      };
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
  }

  function resetTrim() {
    startTime = 0;
    endTime = 0;
    if (audioEl && value) {
      loadAudio(value);
    }
  }
  
  async function saveTrimmed() {
    if (startTime > 0 || (endTime > 0 && endTime < duration)) {
      await applyTrim();
    }
  }

  async function applyTrim() {
    if (!audioEl || !value || (startTime === 0 && endTime === duration)) {
      return; // No trimming needed
    }
    
    ensureAudio();
    if (!audioCtx) return;
    
    try {
      // Get audio data - either from hash or data URL
      let arrayBuffer: ArrayBuffer;
      if (/^[a-f0-9]{64}$/i.test(value)) {
        // Load from backend
        const { invoke } = await import('@tauri-apps/api/core');
        const data = await invoke<number[]>('get_sound', { hash: value });
        arrayBuffer = new Uint8Array(data).buffer;
      } else if (value.startsWith('data:')) {
        // Load from data URL
        const response = await fetch(value);
        arrayBuffer = await response.arrayBuffer();
      } else {
        return;
      }
      
      // Decode audio data
      const audioBuffer = await audioCtx.decodeAudioData(arrayBuffer);
      
      // Calculate trim sample range
      const sampleRate = audioBuffer.sampleRate;
      const startSample = Math.floor(startTime * sampleRate);
      const endSample = Math.min(
        Math.floor((endTime || duration) * sampleRate),
        audioBuffer.length
      );
      const length = endSample - startSample;
      
      // Create new audio buffer with trimmed data
      const trimmedBuffer = audioCtx!.createBuffer(
        audioBuffer.numberOfChannels,
        length,
        sampleRate
      );
      
      for (let channel = 0; channel < audioBuffer.numberOfChannels; channel++) {
        const inputData = audioBuffer.getChannelData(channel);
        const outputData = trimmedBuffer.getChannelData(channel);
        for (let i = 0; i < length; i++) {
          outputData[i] = inputData[startSample + i];
        }
      }
      
      // Convert to WAV format
      const wavData = audioBufferToWav(trimmedBuffer);
      const blob = new Blob([wavData], { type: 'audio/wav' });
      
      // Convert to base64 and upload
      const reader = new FileReader();
      reader.onload = async () => {
        const base64Data = reader.result as string;
        const dataUrl = base64Data;
        
        // Extract base64 part (remove data:audio/wav;base64,)
        const base64Content = base64Data.split(',')[1];
        const binaryData = Uint8Array.from(atob(base64Content), c => c.charCodeAt(0));
        
        // Upload to backend
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const hash = await invoke<string>('upload_sound', { data: Array.from(binaryData) });
          // Store hash instead of full data
          value = hash;
          onvaluechange?.(hash);
          
          // Reload with the trimmed version
          await loadAudio(hash);
        } catch (e) {
          console.error('Failed to upload trimmed sound:', e);
        }
      };
      reader.readAsDataURL(blob);
    } catch (e) {
      console.error('Failed to trim audio:', e);
    }
  }
  
  function audioBufferToWav(buffer: AudioBuffer): ArrayBuffer {
    const numChannels = buffer.numberOfChannels;
    const sampleRate = buffer.sampleRate;
    const format = 1; // PCM
    const bitDepth = 16;
    const bytesPerSample = bitDepth / 8;
    const blockAlign = numChannels * bytesPerSample;
    const byteRate = sampleRate * blockAlign;
    const length = buffer.length * blockAlign;
    
    const arrayBuffer = new ArrayBuffer(44 + length);
    const view = new DataView(arrayBuffer);
    
    // Write WAV header
    const writeString = (offset: number, string: string) => {
      for (let i = 0; i < string.length; i++) {
        view.setUint8(offset + i, string.charCodeAt(i));
      }
    };
    
    writeString(0, 'RIFF');
    view.setUint32(4, 36 + length, true);
    writeString(8, 'WAVE');
    writeString(12, 'fmt ');
    view.setUint32(16, 16, true);
    view.setUint16(20, format, true);
    view.setUint16(22, numChannels, true);
    view.setUint32(24, sampleRate, true);
    view.setUint32(28, byteRate, true);
    view.setUint16(32, blockAlign, true);
    view.setUint16(34, bitDepth, true);
    writeString(36, 'data');
    view.setUint32(40, length, true);
    
    // Write audio data
    let offset = 44;
    for (let i = 0; i < buffer.length; i++) {
      for (let channel = 0; channel < numChannels; channel++) {
        const sample = Math.max(-1, Math.min(1, buffer.getChannelData(channel)[i]));
        view.setInt16(offset, sample < 0 ? sample * 0x8000 : sample * 0x7FFF, true);
        offset += 2;
      }
    }
    
    return arrayBuffer;
  }
  
  let audioCtx: AudioContext | null = null;
  
  function ensureAudio() {
    if (!audioCtx) {
      const Ctx = window.AudioContext || (window as any).webkitAudioContext;
      if (Ctx) audioCtx = new Ctx();
    }
    audioCtx?.resume?.();
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
      <audio bind:this={audioEl} preload="none" onloadedmetadata={() => { if (audioEl) { duration = audioEl.duration; if (endTime === 0 || endTime > duration) endTime = duration; } }} ontimeupdate={() => { if (audioEl) { currentTime = audioEl.currentTime; if (endTime > 0 && audioEl.currentTime >= endTime) { audioEl.pause(); isPlaying = false; audioEl.currentTime = startTime; } } }} onended={() => { isPlaying = false; if (audioEl) audioEl.currentTime = startTime; }}></audio>
      
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

        {#if startTime > 0 || (endTime > 0 && endTime < duration)}
          <button
            class="px-2 py-1.5 text-xs rounded-lg border [border-color:var(--color-card-border)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors"
            onclick={resetTrim}
            title="Reset trim"
          >
            Reset
          </button>
          <button
            class="px-2 py-1.5 text-xs rounded-lg border [border-color:var(--color-primary)] [background-color:var(--color-primary)] [color:#05210c] hover:opacity-90 transition-colors font-semibold"
            onclick={saveTrimmed}
            title="Save trimmed sound"
          >
            Save Trim
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

