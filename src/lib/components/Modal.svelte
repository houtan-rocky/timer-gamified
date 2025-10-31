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
  <div class="backdrop fixed inset-0 z-[10000] bg-black/33 flex items-center justify-center" on:click={handleBackdrop}>
    <div class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[min(560px,92vw)] max-h-[92vh] overflow-auto z-[1010] [background-color:var(--color-card)] rounded-xl px-5 py-6 shadow-[0_8px_48px_rgba(0,0,0,0.67)]">
      <slot />
    </div>
  </div>
{/if}

