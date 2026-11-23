<script lang="ts">
  import Button from '../atoms/Button.svelte';
  import IconButton from '../atoms/IconButton.svelte';
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import type { CookiesState } from '../../lib/workspace/cookies.svelte';

  let { cookies }: { cookies: CookiesState } = $props();

  /** Session cookies have no date; others in the user's language. */
  function expiry(expires: number | null | undefined): string {
    if (expires === null || expires === undefined) return i18n.t('cookies.session');
    return new Date(expires * 1000).toLocaleString(i18n.locale);
  }

  function submit(event: SubmitEvent) {
    event.preventDefault();
    void cookies.save();
  }

  function keydown(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    if (cookies.editing) cookies.cancelEdit();
    else cookies.close();
  }
</script>

{#if cookies.shown}
  <div class="cookies-dialog">
    <div
      class="cookies-dialog__dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="cookies-dialog-title"
      tabindex="-1"
      onkeydown={keydown}
    >
      <h2 id="cookies-dialog-title" class="cookies-dialog__title">
        {i18n.t('cookies.title', { name: cookies.shown.name })}
      </h2>
      <p class="cookies-dialog__note">{i18n.t('cookies.intro')}</p>

      {#if cookies.cookies.length === 0}
        <p class="cookies-dialog__empty">{i18n.t('cookies.empty')}</p>
      {:else}
        <div class="cookies-dialog__scroll">
          <table class="cookies-dialog__table">
            <thead>
              <tr>
                <th class="cookies-dialog__cell cookies-dialog__cell--head">
                  {i18n.t('cookies.name')}
                </th>
                <th class="cookies-dialog__cell cookies-dialog__cell--head">
                  {i18n.t('cookies.value')}
                </th>
                <th class="cookies-dialog__cell cookies-dialog__cell--head">
                  {i18n.t('cookies.domain')}
                </th>
                <th class="cookies-dialog__cell cookies-dialog__cell--head">
                  {i18n.t('cookies.path')}
                </th>
                <th class="cookies-dialog__cell cookies-dialog__cell--head">
                  {i18n.t('cookies.expires')}
                </th>
                <th class="cookies-dialog__cell cookies-dialog__cell--head">
                  <span class="cookies-dialog__hidden">{i18n.t('cookies.actions')}</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {#each cookies.cookies as cookie (`${cookie.domain} ${cookie.path} ${cookie.name}`)}
                <tr>
                  <td class="cookies-dialog__cell cookies-dialog__code">{cookie.name}</td>
                  <td
                    class="cookies-dialog__cell cookies-dialog__code cookies-dialog__value"
                    title={cookie.value}
                  >
                    {cookie.value}
                  </td>
                  <td class="cookies-dialog__cell">
                    {cookie.hostOnly ? cookie.domain : `.${cookie.domain}`}
                  </td>
                  <td class="cookies-dialog__cell">{cookie.path}</td>
                  <td class="cookies-dialog__cell">{expiry(cookie.expires)}</td>
                  <td class="cookies-dialog__cell cookies-dialog__row-actions">
                    <IconButton
                      icon="edit"
                      label={i18n.t('cookies.edit', { name: cookie.name })}
                      onclick={() => cookies.edit(cookie)}
                    />
                    <IconButton
                      icon="trash"
                      label={i18n.t('cookies.delete', { name: cookie.name })}
                      onclick={() => void cookies.remove(cookie)}
                    />
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      {#if cookies.editing}
        {@const draft = cookies.editing.draft}
        <form class="cookies-dialog__form" onsubmit={submit}>
          <h3 class="cookies-dialog__subtitle cookies-dialog__wide">
            {cookies.editing.original ? i18n.t('cookies.edit_title') : i18n.t('cookies.add_title')}
          </h3>
          <label class="cookies-dialog__field">
            <span>{i18n.t('cookies.name')}</span>
            <input class="cookies-dialog__input" spellcheck="false" bind:value={draft.name} />
          </label>
          <label class="cookies-dialog__field">
            <span>{i18n.t('cookies.value')}</span>
            <input class="cookies-dialog__input" spellcheck="false" bind:value={draft.value} />
          </label>
          <label class="cookies-dialog__field">
            <span>{i18n.t('cookies.domain')}</span>
            <input class="cookies-dialog__input" spellcheck="false" bind:value={draft.domain} />
          </label>
          <label class="cookies-dialog__field">
            <span>{i18n.t('cookies.path')}</span>
            <input class="cookies-dialog__input" spellcheck="false" bind:value={draft.path} />
          </label>
          <label class="cookies-dialog__check">
            <input
              type="checkbox"
              checked={!draft.hostOnly}
              onchange={(event) => (draft.hostOnly = !event.currentTarget.checked)}
            />
            {i18n.t('cookies.subdomains')}
          </label>
          <label class="cookies-dialog__check">
            <input type="checkbox" bind:checked={draft.secure} />
            {i18n.t('cookies.secure')}
          </label>
          <div class="cookies-dialog__actions cookies-dialog__wide">
            <Button onclick={() => cookies.cancelEdit()}>{i18n.t('dialog.cancel')}</Button>
            <Button type="submit" variant="primary">{i18n.t('editor.save')}</Button>
          </div>
        </form>
      {/if}

      {#if cookies.error}
        <p class="cookies-dialog__error" role="alert">{i18n.text(cookies.error)}</p>
      {/if}
      <div class="cookies-dialog__actions">
        {#if !cookies.editing}
          <Button onclick={() => cookies.add()}>{i18n.t('cookies.add')}</Button>
        {/if}
        <Button onclick={() => void cookies.clear()} disabled={cookies.cookies.length === 0}>
          {i18n.t('cookies.clear')}
        </Button>
        <span class="cookies-dialog__spacer"></span>
        <Button variant="primary" onclick={() => cookies.close()}>{i18n.t('cookies.close')}</Button>
      </div>
    </div>
  </div>
{/if}

<style>
  .cookies-dialog {
    position: fixed;
    inset: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
  }

  .cookies-dialog__dialog {
    width: min(860px, calc(100% - 2 * var(--space-l)));
    max-height: calc(100% - 2 * var(--space-l));
    display: flex;
    flex-direction: column;
    gap: var(--space-m);
    padding: var(--space-xl);
    border-radius: var(--radius);
    background: var(--color-surface);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
    overflow-y: auto;
  }

  .cookies-dialog__title {
    margin: 0;
    font-size: var(--font-size-l);
  }

  .cookies-dialog__subtitle {
    margin: 0;
    font-size: var(--font-size-m);
  }

  .cookies-dialog__note,
  .cookies-dialog__empty {
    margin: 0;
    color: var(--color-muted);
    line-height: 1.5;
  }

  .cookies-dialog__scroll {
    overflow-x: auto;
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
  }

  .cookies-dialog__table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-s);
  }

  .cookies-dialog__cell {
    padding: var(--space-xs) var(--space-s);
    text-align: left;
    vertical-align: middle;
    border-bottom: 1px solid var(--color-border);
  }

  .cookies-dialog__cell--head {
    color: var(--color-muted);
    font-weight: normal;
  }

  .cookies-dialog__code {
    font-family: var(--font-mono);
  }

  .cookies-dialog__value {
    max-width: 16em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cookies-dialog__row-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-xs);
  }

  .cookies-dialog__hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }

  .cookies-dialog__form {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-s) var(--space-m);
    padding: var(--space-m);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
  }

  .cookies-dialog__wide {
    grid-column: 1 / -1;
  }

  .cookies-dialog__field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    color: var(--color-muted);
  }

  .cookies-dialog__check {
    display: flex;
    align-items: center;
    gap: var(--space-s);
  }

  .cookies-dialog__input {
    height: var(--control-height);
    padding: 0 var(--space-m);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-bg);
    color: var(--color-text);
    font-family: var(--font-mono);
  }

  .cookies-dialog__error {
    margin: 0;
    color: var(--color-error);
  }

  .cookies-dialog__actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-s);
  }

  .cookies-dialog__spacer {
    flex: 1;
  }
</style>
