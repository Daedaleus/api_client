<script lang="ts">
  let { method }: { method: string } = $props();

  /** Colour group per method; other methods use the neutral colour. */
  const KIND: Record<string, string> = {
    GET: 'read',
    HEAD: 'read',
    OPTIONS: 'read',
    POST: 'create',
    PUT: 'change',
    PATCH: 'change',
    DELETE: 'delete',
  };

  let kind = $derived(KIND[method] ?? 'other');
  let label = $derived(method === 'DELETE' ? 'DEL' : method);
</script>

<span class="method-badge method-badge--{kind}">{label}</span>

<style>
  .method-badge {
    flex: none;
    min-width: 34px;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
  }

  .method-badge--read {
    color: var(--color-success);
  }

  .method-badge--create {
    color: var(--color-warning);
  }

  .method-badge--change {
    color: var(--color-redirect);
  }

  .method-badge--delete {
    color: var(--color-error);
  }

  .method-badge--other {
    color: var(--color-muted);
  }
</style>
