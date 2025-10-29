<script lang="ts">
  import { onMount } from "svelte";
  export let open: boolean = false;
  export let title: string = '';
  export let subtitle: string = '';
  export let onclose: (() => void) | undefined = undefined;

  function handleBackdrop(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('backdrop')) {
      onclose?.();
    }
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose?.();
  }

  onMount(() => {
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  });
</script>

{#if open}
  <div class="backdrop" onclick={handleBackdrop}>
    <div class="sheet card" role="dialog" aria-modal="true">
      <div class="head">
        <div class="titles">
          <h2>{title}</h2>
          {#if subtitle}<p class="muted">{subtitle}</p>{/if}
        </div>
        <button class="close" title="Close" onclick={onclose}>×</button>
      </div>
      <div class="body">
        <slot />
      </div>
    </div>
  </div>
{/if}

<style>
.backdrop {
  position: fixed; inset: 0; background: rgba(0,0,0,0.6);
  display: grid; place-items: center; z-index: 9999;
}
.sheet { width: min(560px, 92vw); padding: 16px; }
.head { display: flex; align-items: center; justify-content: space-between; }
.titles h2 { margin: 0 0 4px 0; }
.titles p { margin: 0; }
.close { background: transparent; border: 1px solid var(--card-border); border-radius: 8px; color: var(--text); padding: 4px 10px; cursor: pointer; }
.body { margin-top: 12px; }
</style>

