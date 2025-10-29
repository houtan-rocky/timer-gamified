<script lang="ts">
  export let variant: 'primary' | 'secondary' | 'ghost' = 'primary';
  export let size: 'sm' | 'md' | 'lg' = 'md';
  export let title: string | undefined = undefined;
  export let type: 'button' | 'submit' = 'button';
  export let disabled: boolean = false;
  export let onclick: ((e: MouseEvent) => void) | undefined = undefined;
</script>

<button
  {title}
  {type}
  {disabled}
  onclick={onclick}
  class={`btn ${variant} ${size}`}
>
  <slot />
  <style>
  .btn {
    border-radius: 10px;
    border: 1px solid transparent;
    padding: 0.6em 1.0em;
    font-size: 1em;
    font-weight: 600;
    color: var(--text);
    background: var(--card);
    border-color: var(--card-border);
    box-shadow: var(--shadow);
    cursor: pointer;
    display: inline-flex;
    gap: 8px;
    align-items: center;
    justify-content: center;
  }
  .btn:hover { border-color: #3a3a3a; }
  .btn:active { transform: translateY(1px); }

  .primary {
    background: var(--primary);
    color: #05210c;
    border-color: #72d480;
  }
  .primary:hover { filter: brightness(0.98); }

  .secondary { background: #1b1b1b; }
  .ghost { background: transparent; border-color: #2a2a2a; }

  .sm { padding: 0.4em 0.8em; font-size: 0.9em; }
  .lg { padding: 0.8em 1.2em; font-size: 1.1em; }
  </style>
</button>

