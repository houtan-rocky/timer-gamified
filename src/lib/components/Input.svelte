<script lang="ts">
  export let type: 'text' | 'number' | 'color' | 'range' = 'text';
  export let value: string | number = '';
  export let placeholder: string = '';
  export let min: number | string | undefined = undefined;
  export let max: number | string | undefined = undefined;
  export let disabled: boolean = false;
  export let id: string | undefined = undefined;
  let className: string = '';
  export { className as class };
  export let onchange: ((e: Event) => void) | undefined = undefined;
  export let oninput: ((e: Event) => void) | undefined = undefined;
  export let onblur: ((e: Event) => void) | undefined = undefined;
  export let autofocus: boolean = false;
  export let onkeydown: ((e: KeyboardEvent) => void) | undefined = undefined;
  let inputElement: HTMLInputElement | undefined = undefined;
  
  export { inputElement as editInputEl };
  
  const baseClasses = "rounded-lg border [border-color:var(--color-card-border)] [background-color:var(--color-card)] [color:var(--color-foreground)] px-2 py-1 transition-colors focus:outline-none focus:ring-2 focus:[ring-color:var(--color-primary)] focus:ring-opacity-50";
  
  const typeClasses = {
    text: "p-2",
    number: "p-2",
    color: "h-10 w-20 cursor-pointer border-2",
    range: "w-full h-2 rounded-lg appearance-none bg-[#242424] cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[var(--color-primary)] [&::-webkit-slider-thumb]:cursor-pointer [&::-moz-range-thumb]:w-4 [&::-moz-range-thumb]:h-4 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:bg-[var(--color-primary)] [&::-moz-range-thumb]:cursor-pointer [&::-moz-range-thumb]:border-none"
  };
</script>

{#if type === 'range'}
  <input
    {type}
    bind:value
    {min}
    {max}
    {disabled}
    {onchange}
    {oninput}
    class="{baseClasses} {typeClasses[type]} {className}"
  />
{:else}
  <input
    {id}
    {type}
    bind:value
    {placeholder}
    {min}
    {max}
    {disabled}
    {autofocus}
    bind:this={inputElement}
    {onchange}
    {oninput}
    {onblur}
    {onkeydown}
    class="{baseClasses} {typeClasses[type]} {className}"
  />
{/if}

