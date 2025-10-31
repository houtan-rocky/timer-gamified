<script lang="ts">
  export let checked: boolean = false;
  export let disabled: boolean = false;
  export let onchange: ((e: Event) => void) | undefined = undefined;
  export let label: string = '';
  let className: string = '';
  export { className as class };
</script>

<label class="flex gap-2 items-center cursor-pointer {disabled ? 'opacity-50 cursor-not-allowed' : ''} {className}">
  <input
    type="checkbox"
    bind:checked
    {disabled}
    {onchange}
    class="w-4 h-4 rounded border-2 [border-color:var(--color-card-border)] [background-color:var(--color-card)] cursor-pointer appearance-none checked:[background-color:var(--color-primary)] checked:border-[var(--color-primary)] transition-colors relative after:content-['✓'] after:absolute after:left-1/2 after:top-1/2 after:-translate-x-1/2 after:-translate-y-1/2 after:text-white after:text-xs after:font-bold after:opacity-0 checked:after:opacity-100"
  />
  {#if label}
    <span class="[color:var(--color-foreground)]">{label}</span>
  {/if}
  {#if $$slots.default}
    <slot />
  {/if}
</label>

