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
  <div class="modal-backdrop" on:click={handleBackdrop}>
    <div class="modal-content">
      <slot />
    </div>
  </div>
{/if}

<style>
.modal-backdrop {
  position: fixed;
  top: 0; left: 0;
  width: 100vw;
  height: 100vh;
  z-index: 10000;
  background: rgba(0,0,0,0.33);
  display: flex;
  align-items: center;
  justify-content: center;
}
.sheet {
  padding: 16px;
}
.modal-content {
  position: fixed;
  top: 50%; left: 50%;
  transform: translate(-50%, -50%);
  width: min(560px, 92vw);
  max-height: 92vh;
  overflow: auto;
  z-index: 1010;
  background: var(--modal-bg, #222d);
  border-radius: 12px;
  padding: 24px 20px 18px 20px;
  box-shadow: 0 8px 48px #000a;
}
.head { display: flex; align-items: center; justify-content: space-between; }
.titles h2 { margin: 0 0 4px 0; }
.titles p { margin: 0; }
.close { background: transparent; border: 1px solid var(--card-border); border-radius: 8px; color: var(--text); padding: 4px 10px; cursor: pointer; }
/* .body { margin-top: 12px; } */
</style>

