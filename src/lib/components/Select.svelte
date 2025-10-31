<script lang="ts">
  export let value: string | number = '';
  export let disabled: boolean = false;
  export let onchange: ((e: Event) => void) | undefined = undefined;
  let className: string = '';
  export { className as class };
</script>

<select
  bind:value
  {disabled}
  {onchange}
  class="rounded-lg border [border-color:var(--color-card-border)] [background-color:var(--color-card)] [color:var(--color-foreground)] px-2 py-1 cursor-pointer transition-colors focus:outline-none focus:ring-2 focus:[ring-color:var(--color-primary)] focus:ring-opacity-50 {className}"
>
  <slot />
</select>

