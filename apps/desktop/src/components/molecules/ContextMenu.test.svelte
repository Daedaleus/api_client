<script lang="ts">
  import type { MenuItem } from '../../lib/menu';
  import ContextMenu from './ContextMenu.svelte';

  let { items }: { items: () => MenuItem[] } = $props();
</script>

<ContextMenu {items} label="Row actions">
  <button type="button">Row</button>
</ContextMenu>
