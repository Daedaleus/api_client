<script lang="ts" module>
  // Stroke icons in the current text colour; decorative (labels sit on the button).
  export type IconName = 'plus' | 'folder-plus' | 'edit' | 'trash' | 'close' | 'chevron' | 'open';
</script>

<script lang="ts">
  let { name, size = 14 }: { name: IconName; size?: number } = $props();

  const PATHS: Record<IconName, string> = {
    plus: 'M12 5v14M5 12h14',
    'folder-plus':
      'M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM12 10v6M9 13h6',
    edit: 'M4 20h4L19 9l-4-4L4 16zM14 6l4 4',
    trash: 'M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3',
    close: 'M6 6l12 12M18 6L6 18',
    chevron: 'M9 6l6 6-6 6',
    open: 'M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z',
  };
</script>

<svg
  class="icon"
  aria-hidden="true"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
>
  <path d={PATHS[name]} />
</svg>

<style>
  .icon {
    flex: none;
  }
</style>
