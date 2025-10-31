<script lang="ts">
  let { value = $bindable<string | null | undefined>(), class: className = '', onvaluechange } = $props<{
    value?: string | null | undefined;
    class?: string;
    onvaluechange?: (value: string | null | undefined) => void;
  }>();

  const uniqueId = `img-editor-${Math.random().toString(36).substring(2, 9)}`;
  let fileInput = $state<HTMLInputElement | null>(null);
  let previewCanvas = $state<HTMLCanvasElement | null>(null);
  let sourceImage = $state<HTMLImageElement | null>(null);
  let showEditor = $state(false);
  
  // Crop state
  let cropX = $state(0);
  let cropY = $state(0);
  let cropWidth = $state(0);
  let cropHeight = $state(0);
  let isDragging = $state(false);
  let dragStartX = $state(0);
  let dragStartY = $state(0);
  let isResizing = $state(false);
  let resizeHandle = $state<string | null>(null);
  
  // Filter state
  let brightness = $state(100);
  let contrast = $state(100);
  let saturation = $state(100);
  let grayscale = $state(0);
  let sepia = $state(0);
  let blur = $state(0);

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
        const hash = await invoke<string>('upload_image', { data: Array.from(binaryData) });
        value = hash;
        onvaluechange?.(hash);
        await loadImage(hash);
      } catch (e) {
        console.error('Failed to upload image:', e);
        // Fallback to base64 if backend fails
        value = result;
        onvaluechange?.(result);
        loadImageFromDataUrl(result);
      }
    };
    reader.readAsDataURL(file);
  }

  async function loadImage(hashOrUrl: string) {
    // Check if it's a hash (64 hex chars) or data URL
    if (hashOrUrl.startsWith('data:')) {
      loadImageFromDataUrl(hashOrUrl);
    } else if (/^[a-f0-9]{64}$/i.test(hashOrUrl)) {
      // It's a hash, load from backend
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const data = await invoke<number[]>('get_image', { hash: hashOrUrl });
        const blob = new Blob([new Uint8Array(data)]);
        const url = URL.createObjectURL(blob);
        loadImageFromDataUrl(url);
      } catch (e) {
        console.error('Failed to load image from backend:', e);
      }
    } else {
      loadImageFromDataUrl(hashOrUrl);
    }
  }

  function loadImageFromDataUrl(url: string) {
    const img = new Image();
    img.onload = () => {
      sourceImage = img;
      showEditor = true;
      
      // Initialize crop to full image - wait for canvas to be ready
      setTimeout(() => {
        if (previewCanvas) {
          const scale = Math.min(
            previewCanvas.width / img.width,
            previewCanvas.height / img.height
          );
          const displayWidth = img.width * scale;
          const displayHeight = img.height * scale;
          const offsetX = (previewCanvas.width - displayWidth) / 2;
          const offsetY = (previewCanvas.height - displayHeight) / 2;
          
          cropX = offsetX;
          cropY = offsetY;
          cropWidth = displayWidth;
          cropHeight = displayHeight;
          
          drawImage();
        }
      }, 50);
    };
    img.src = url;
  }

  function drawImage() {
    if (!previewCanvas || !sourceImage) return;
    
    const ctx = previewCanvas.getContext('2d');
    if (!ctx) return;
    
    ctx.clearRect(0, 0, previewCanvas.width, previewCanvas.height);
    
    // Calculate display size (fit to canvas)
    const scale = Math.min(
      previewCanvas.width / sourceImage.width,
      previewCanvas.height / sourceImage.height
    );
    const displayWidth = sourceImage.width * scale;
    const displayHeight = sourceImage.height * scale;
    const offsetX = (previewCanvas.width - displayWidth) / 2;
    const offsetY = (previewCanvas.height - displayHeight) / 2;
    
    // Apply filters
    ctx.filter = `
      brightness(${brightness}%)
      contrast(${contrast}%)
      saturate(${saturation}%)
      grayscale(${grayscale}%)
      sepia(${sepia}%)
      blur(${blur}px)
    `;
    
    ctx.drawImage(sourceImage, offsetX, offsetY, displayWidth, displayHeight);
    
    // Draw crop overlay
    ctx.strokeStyle = '#4ade80';
    ctx.lineWidth = 2;
    ctx.setLineDash([5, 5]);
    ctx.strokeRect(cropX, cropY, cropWidth, cropHeight);
    
    // Draw semi-transparent overlay outside crop area
    ctx.fillStyle = 'rgba(0, 0, 0, 0.5)';
    ctx.fillRect(0, 0, previewCanvas.width, cropY);
    ctx.fillRect(0, cropY, cropX, cropHeight);
    ctx.fillRect(cropX + cropWidth, cropY, previewCanvas.width - cropX - cropWidth, cropHeight);
    ctx.fillRect(0, cropY + cropHeight, previewCanvas.width, previewCanvas.height - cropY - cropHeight);
    
    // Draw resize handles
    const handleSize = 8;
    ctx.fillStyle = '#4ade80';
    ctx.fillRect(cropX - handleSize/2, cropY - handleSize/2, handleSize, handleSize); // top-left
    ctx.fillRect(cropX + cropWidth - handleSize/2, cropY - handleSize/2, handleSize, handleSize); // top-right
    ctx.fillRect(cropX - handleSize/2, cropY + cropHeight - handleSize/2, handleSize, handleSize); // bottom-left
    ctx.fillRect(cropX + cropWidth - handleSize/2, cropY + cropHeight - handleSize/2, handleSize, handleSize); // bottom-right
  }

  function handleMouseDown(e: MouseEvent) {
    if (!previewCanvas) return;
    const rect = previewCanvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    const handleSize = 8;
    const handles = [
      { name: 'nw', x: cropX, y: cropY },
      { name: 'ne', x: cropX + cropWidth, y: cropY },
      { name: 'sw', x: cropX, y: cropY + cropHeight },
      { name: 'se', x: cropX + cropWidth, y: cropY + cropHeight },
    ];
    
    // Check if clicking on a resize handle
    for (const handle of handles) {
      if (Math.abs(x - handle.x) < handleSize && Math.abs(y - handle.y) < handleSize) {
        isResizing = true;
        resizeHandle = handle.name;
        dragStartX = x;
        dragStartY = y;
        return;
      }
    }
    
    // Check if clicking inside crop area
    if (x >= cropX && x <= cropX + cropWidth && y >= cropY && y <= cropY + cropHeight) {
      isDragging = true;
      dragStartX = x - cropX;
      dragStartY = y - cropY;
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (!previewCanvas) return;
    const rect = previewCanvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    if (isDragging) {
      cropX = Math.max(0, Math.min(x - dragStartX, previewCanvas.width - cropWidth));
      cropY = Math.max(0, Math.min(y - dragStartY, previewCanvas.height - cropHeight));
      drawImage();
    } else if (isResizing && resizeHandle) {
      const minSize = 20;
      if (resizeHandle.includes('e')) {
        cropWidth = Math.max(minSize, x - cropX);
      }
      if (resizeHandle.includes('w')) {
        const newWidth = cropWidth + cropX - x;
        if (newWidth >= minSize) {
          cropWidth = newWidth;
          cropX = x;
        }
      }
      if (resizeHandle.includes('s')) {
        cropHeight = Math.max(minSize, y - cropY);
      }
      if (resizeHandle.includes('n')) {
        const newHeight = cropHeight + cropY - y;
        if (newHeight >= minSize) {
          cropHeight = newHeight;
          cropY = y;
        }
      }
      
      // Keep within canvas bounds
      cropX = Math.max(0, Math.min(cropX, previewCanvas.width - cropWidth));
      cropY = Math.max(0, Math.min(cropY, previewCanvas.height - cropHeight));
      cropWidth = Math.min(cropWidth, previewCanvas.width - cropX);
      cropHeight = Math.min(cropHeight, previewCanvas.height - cropY);
      
      drawImage();
    }
  }

  function handleMouseUp() {
    isDragging = false;
    isResizing = false;
    resizeHandle = null;
  }

  function resetFilters() {
    brightness = 100;
    contrast = 100;
    saturation = 100;
    grayscale = 0;
    sepia = 0;
    blur = 0;
    drawImage();
  }

  async function applyChanges() {
    if (!sourceImage || !previewCanvas) return;
    
    // Calculate actual crop coordinates on source image
    const scale = Math.min(
      previewCanvas.width / sourceImage.width,
      previewCanvas.height / sourceImage.height
    );
    const displayWidth = sourceImage.width * scale;
    const displayHeight = sourceImage.height * scale;
    const offsetX = (previewCanvas.width - displayWidth) / 2;
    const offsetY = (previewCanvas.height - displayHeight) / 2;
    
    // Convert crop coordinates from display to source image coordinates
    const sourceX = Math.max(0, (cropX - offsetX) / scale);
    const sourceY = Math.max(0, (cropY - offsetY) / scale);
    const sourceWidth = Math.min(sourceImage.width - sourceX, cropWidth / scale);
    const sourceHeight = Math.min(sourceImage.height - sourceY, cropHeight / scale);
    
    // Create output canvas
    const outputCanvas = document.createElement('canvas');
    outputCanvas.width = sourceWidth;
    outputCanvas.height = sourceHeight;
    const ctx = outputCanvas.getContext('2d');
    if (!ctx) return;
    
    // Apply filters
    ctx.filter = `
      brightness(${brightness}%)
      contrast(${contrast}%)
      saturate(${saturation}%)
      grayscale(${grayscale}%)
      sepia(${sepia}%)
      blur(${blur}px)
    `;
    
    // Draw cropped and filtered image
    ctx.drawImage(
      sourceImage,
      sourceX, sourceY, sourceWidth, sourceHeight,
      0, 0, sourceWidth, sourceHeight
    );
    
    // Convert to PNG blob
    outputCanvas.toBlob(async (blob) => {
      if (!blob) return;
      
      const reader = new FileReader();
      reader.onload = async () => {
        const base64Data = reader.result as string;
        const base64Content = base64Data.split(',')[1];
        const binaryData = Uint8Array.from(atob(base64Content), c => c.charCodeAt(0));
        
        // Upload to backend
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const hash = await invoke<string>('upload_image', { data: Array.from(binaryData) });
          value = hash;
          onvaluechange?.(hash);
          
          // Update preview
          await loadImage(hash);
        } catch (e) {
          console.error('Failed to upload image:', e);
          // Fallback to data URL
          value = base64Data;
          onvaluechange?.(base64Data);
          loadImageFromDataUrl(base64Data);
        }
      };
      reader.readAsDataURL(blob);
    }, 'image/png');
  }

  function removeImage() {
    value = null;
    onvaluechange?.(null);
    showEditor = false;
    sourceImage = null;
    if (fileInput) {
      fileInput.value = '';
    }
    resetFilters();
  }

  $effect(() => {
    if (value && !sourceImage) {
      loadImage(value);
    } else if (!value || value === null || value === undefined) {
      showEditor = false;
      sourceImage = null;
    }
  });

  $effect(() => {
    if (showEditor && sourceImage) {
      drawImage();
    }
  });
  
  // Load image when value changes (including hash values)
  $effect(() => {
    if (value && !sourceImage) {
      // Wait for canvas to be created
      setTimeout(() => {
        loadImage(value);
      }, 100);
    }
  });
</script>

<div class="flex flex-col gap-2 {className}">
  <label class="flex flex-col gap-2">
    <span class="[color:var(--color-muted)] text-sm">Image file</span>
    <input
      bind:this={fileInput}
      type="file"
      accept="image/*"
      onchange={handleFileChange}
      class="text-sm [color:var(--color-foreground)] [background-color:var(--color-card)] border [border-color:var(--color-card-border)] rounded-lg px-3 py-2 file:mr-4 file:py-1 file:px-3 file:rounded-lg file:border-0 file:text-sm file:font-semibold file:[background-color:var(--color-primary)] file:text-[#05210c] hover:file:opacity-90 cursor-pointer"
    />
  </label>

  {#if showEditor && sourceImage}
    <div class="flex flex-col gap-3 p-3 border rounded-lg [border-color:var(--color-card-border)] [background-color:var(--color-card)]">
      <!-- Canvas Preview -->
      <div class="relative border rounded-lg [border-color:var(--color-card-border)] overflow-hidden bg-gray-900">
        <canvas
          bind:this={previewCanvas}
          width={400}
          height={300}
          class="w-full h-auto cursor-crosshair"
          onmousedown={handleMouseDown}
          onmousemove={handleMouseMove}
          onmouseup={handleMouseUp}
          onmouseleave={handleMouseUp}
          style="max-height: 400px; object-fit: contain;"
        ></canvas>
      </div>

      <!-- Filter Controls -->
      <div class="grid grid-cols-2 gap-3">
        <div class="flex flex-col gap-1">
          <label for="{uniqueId}-brightness" class="text-xs [color:var(--color-muted)]">Brightness: {brightness}%</label>
          <input
            id="{uniqueId}-brightness"
            type="range"
            min="0"
            max="200"
            bind:value={brightness}
            oninput={() => drawImage()}
            class="w-full"
          />
        </div>
        <div class="flex flex-col gap-1">
          <label for="{uniqueId}-contrast" class="text-xs [color:var(--color-muted)]">Contrast: {contrast}%</label>
          <input
            id="{uniqueId}-contrast"
            type="range"
            min="0"
            max="200"
            bind:value={contrast}
            oninput={() => drawImage()}
            class="w-full"
          />
        </div>
        <div class="flex flex-col gap-1">
          <label for="{uniqueId}-saturation" class="text-xs [color:var(--color-muted)]">Saturation: {saturation}%</label>
          <input
            id="{uniqueId}-saturation"
            type="range"
            min="0"
            max="200"
            bind:value={saturation}
            oninput={() => drawImage()}
            class="w-full"
          />
        </div>
        <div class="flex flex-col gap-1">
          <label for="{uniqueId}-grayscale" class="text-xs [color:var(--color-muted)]">Grayscale: {grayscale}%</label>
          <input
            id="{uniqueId}-grayscale"
            type="range"
            min="0"
            max="100"
            bind:value={grayscale}
            oninput={() => drawImage()}
            class="w-full"
          />
        </div>
        <div class="flex flex-col gap-1">
          <label for="{uniqueId}-sepia" class="text-xs [color:var(--color-muted)]">Sepia: {sepia}%</label>
          <input
            id="{uniqueId}-sepia"
            type="range"
            min="0"
            max="100"
            bind:value={sepia}
            oninput={() => drawImage()}
            class="w-full"
          />
        </div>
        <div class="flex flex-col gap-1">
          <label for="{uniqueId}-blur" class="text-xs [color:var(--color-muted)]">Blur: {blur}px</label>
          <input
            id="{uniqueId}-blur"
            type="range"
            min="0"
            max="20"
            bind:value={blur}
            oninput={() => drawImage()}
            class="w-full"
          />
        </div>
      </div>

      <!-- Action Buttons -->
      <div class="flex gap-2 flex-wrap">
        <button
          class="px-3 py-1.5 text-sm rounded-lg border [border-color:var(--color-card-border)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors"
          onclick={resetFilters}
        >
          Reset Filters
        </button>
        <div class="flex-1"></div>
        <button
          class="px-3 py-1.5 text-sm rounded-lg border [border-color:var(--color-primary)] [background-color:var(--color-primary)] [color:#05210c] hover:opacity-90 transition-colors font-semibold"
          onclick={applyChanges}
        >
          Apply Changes
        </button>
        <button
          class="px-3 py-1.5 text-sm rounded-lg border [border-color:var(--color-card-border)] [color:var(--color-foreground)] hover:[background-color:var(--color-card-border)] transition-colors text-red-400 hover:text-red-300"
          onclick={removeImage}
        >
          Remove Image
        </button>
      </div>
      
      <div class="text-xs [color:var(--color-muted)] px-2 py-1 bg-[rgba(74,222,128,0.1)] rounded">
        💡 Drag the green box to move the crop area. Drag the corners to resize.
      </div>
    </div>
  {/if}
</div>

