<script lang="ts">
  let className: string = $state('');
  export { className as class };
  
  type IconVariant = 'settings' | 'message' | 'bell' | 'bell-slash' | 'help' | 'play' | 'pause' | 'reset' | 'overlay';
  type IconSize = 'sm' | 'md' | 'lg';
  
  interface Props {
    variant: IconVariant;
    size?: IconSize;
  }
  
  let { variant, size = 'md' }: Props = $props();
  
  const sizeClasses = {
    sm: 'w-3 h-3',
    md: 'w-4 h-4',
    lg: 'w-5 h-5',
  };
  
  const icons: Record<IconVariant, string> = {
    settings: `<path d="M12 15.5A3.5 3.5 0 0 1 8.5 12A3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5a3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.25-.73.43-1.5.43-2.29c0-1.07-.18-2.07-.5-3H21c.55 0 1-.45 1-1s-.45-1-1-1h-2.09c.14-.56.23-1.16.23-1.79c0-1.07-.09-2.1-.23-3.08H21c.55 0 1-.45 1-1s-.45-1-1-1H3c-.55 0-1 .45-1 1s.45 1 1 1h2.09c-.14.56-.23 1.16-.23 1.79c0 1.07.09 2.1.23 3.08H3c-.55 0-1 .45-1 1s.45 1 1 1h2.09c.32.93.5 1.93.5 3c0 .79-.18 1.56-.43 2.29l-1.73.58c-.25.73-.43 1.5-.43 2.29c0 1.07.18 2.07.5 3H3c-.55 0-1 .45-1 1s.45 1 1 1h2.09c.32.93.5 1.93.5 3c0 .79-.18 1.56-.43 2.29l1.73.58c.25.73.43 1.5.43 2.29c0 1.07-.18 2.07-.5 3H21c.55 0 1-.45 1-1s-.45-1-1-1h-2.09c-.32-.93-.5-1.93-.5-3c0-.79.18-1.56.43-2.29l1.73-.58c.25-.73.43-1.5.43-2.29c0-1.07-.18-2.07-.5-3H21c.55 0 1-.45 1-1s-.45-1-1-1h-2.09c-.32-.93-.5-1.93-.5-3c0-.79.18-1.56.43-2.29l-1.73-.58Z" fill="currentColor"/>`,
    message: `<path d="M20 2H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h4l4 4 4-4h4c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm0 14H6l-2 2V4h16v12z" fill="currentColor"/>`,
    bell: `<path d="M12 22c1.1 0 2-.9 2-2h-4c0 1.1.9 2 2 2zm6-6v-5c0-3.07-1.64-5.64-4.5-6.32V4c0-.83-.67-1.5-1.5-1.5s-1.5.67-1.5 1.5v.68C7.63 5.36 6 7.92 6 11v5l-2 2v1h16v-1l-2-2z" fill="currentColor"/>`,
    'bell-slash': `<path d="M23 22l-20-20 1.41-1.41L24.41 20.59L23 22zM12 2c1.1 0 2 .9 2 2h-4c0-1.1.9-2 2-2zm6 6c0-2.76-1.86-5.08-4.4-5.79L12.81 5L9.39 8.42l.81.81c.25-.38.61-.7 1.03-.93L13 7h-2v2c0 2.21 1.79 4 4 4v2h1c.83 0 1.5-.67 1.5-1.5V8zM5.27 13L4 14.27l.86.86c-.31.78-.48 1.61-.48 2.48v5h14v-5c0-2.19-1.11-4.13-2.81-5.27L16.73 13H5.27z" fill="currentColor"/>`,
    help: `<path d="M11 18h2v-2h-2v2zm1-16C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8zm0-14c-2.21 0-4 1.79-4 4h2c0-1.1.9-2 2-2s2 .9 2 2c0 2-3 1.75-3 5h2c0-2.25 3-2.5 3-5 0-2.21-1.79-4-4-4z" fill="currentColor"/>`,
    play: `<path d="M8 5v14l11-7z" fill="currentColor"/>`,
    pause: `<path d="M6 4h4v16H6V4zm8 0h4v16h-4V4z" fill="currentColor"/>`,
    reset: `<path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z" fill="currentColor"/>`,
    overlay: `<path d="M4 18h7v-2H4v2zm0-4h10v-2H4v2zm0-4h10V8H4v2zm13 0v2h3v-2h-3zm0 4v2h3v-2h-3zm0-8v2h3V6h-3z" fill="currentColor"/>`,
  };
</script>

<svg
  class="{sizeClasses[size]} {className}"
  viewBox="0 0 24 24"
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
  aria-hidden="true"
>
  {@html icons[variant]}
</svg>

