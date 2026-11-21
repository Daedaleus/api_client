<script lang="ts">
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { TestResultView } from '../../lib/types';

  let { tests }: { tests: TestResultView[] } = $props();
</script>

<ul class="test-results" aria-label={i18n.t('response.tests')}>
  {#each tests as test, index (index)}
    <li class="test-results__item" class:test-results__item--failed={!test.passed}>
      <span
        class="test-results__mark"
        role="img"
        aria-label={i18n.t(test.passed ? 'tests.passed' : 'tests.failed')}
      >
        {test.passed ? '✓' : '✗'}
      </span>
      <span class="test-results__name">{test.name}</span>
      {#if test.message}
        <span class="test-results__message">{test.message}</span>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .test-results {
    margin: 0;
    padding: var(--space-s) var(--space-l);
    list-style: none;
  }

  .test-results__item {
    display: grid;
    grid-template-columns: 1.5em minmax(0, 1fr);
    gap: var(--space-xs) var(--space-s);
    padding: var(--space-xs) 0;
  }

  .test-results__mark {
    color: var(--color-success);
    font-weight: 700;
  }

  .test-results__item--failed .test-results__mark {
    color: var(--color-error);
  }

  .test-results__message {
    grid-column: 2;
    color: var(--color-error);
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
    overflow-wrap: anywhere;
  }
</style>
