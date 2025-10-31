<script lang="ts">
  export let variant: 'primary' | 'secondary' | 'ghost' = 'primary';
  export let size: 'sm' | 'md' | 'lg' = 'md';
  export let title: string | undefined = undefined;
  export let type: 'button' | 'submit' = 'button';
  export let disabled: boolean = false;
  export let onclick: ((e: MouseEvent) => void) | undefined = undefined;
  
  const baseClasses = "rounded-[10px] border font-semibold cursor-pointer inline-flex gap-2 items-center justify-center transition-all active:translate-y-px";
  
  const variantClasses = {
    primary: "[background-color:var(--color-primary)] text-[#05210c] border-[#72d480] hover:brightness-[0.98]",
    secondary: "bg-[#1b1b1b] [color:var(--color-foreground)] [border-color:var(--color-card-border)]",
    ghost: "bg-transparent [color:var(--color-foreground)] border-[#2a2a2a]"
  };
  
  const sizeClasses = {
    sm: "px-3 py-1.5 text-sm",
    md: "px-4 py-2.5 text-base",
    lg: "px-5 py-3 text-lg"
  };
</script>

<button
  {title}
  {type}
  {disabled}
  onclick={onclick}
  class="{baseClasses} {variantClasses[variant]} {sizeClasses[size]}"
>
  <slot />
</button>

