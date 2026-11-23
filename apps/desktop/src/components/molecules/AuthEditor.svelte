<script lang="ts" module>
  import type { AuthDraft } from '../../lib/types';

  export type AuthKind = AuthDraft['kind'];

  /** A fresh draft of `kind`, keeping what fits from `previous`. */
  export function authOf(kind: AuthKind, previous: AuthDraft): AuthDraft {
    switch (kind) {
      case 'inherit':
      case 'none':
        return { kind };
      case 'basic':
        return previous.kind === 'basic' ? previous : { kind, username: '', password: '' };
      case 'bearer':
        return previous.kind === 'bearer' ? previous : { kind, token: '' };
      case 'digest':
        return previous.kind === 'digest' ? previous : { kind, username: '', password: '' };
      case 'awsSigV4':
        return previous.kind === 'awsSigV4'
          ? previous
          : {
              kind,
              accessKeyId: '',
              secretAccessKey: '',
              sessionToken: '',
              region: '',
              service: '',
            };
      case 'apiKey':
        return previous.kind === 'apiKey'
          ? previous
          : { kind, name: '', value: '', placement: 'header' };
      case 'oauth2':
        return previous.kind === 'oauth2'
          ? previous
          : {
              kind,
              grant: 'clientCredentials',
              tokenUrl: '',
              authUrl: '',
              redirectUrl: '',
              clientId: '',
              clientSecret: '',
              scope: '',
            };
    }
  }

  /** Whether a credential is typed in as plain text rather than a `{{variable}}`. */
  export function hasPlainCredential(auth: AuthDraft): boolean {
    const credentials =
      auth.kind === 'basic'
        ? [auth.password]
        : auth.kind === 'bearer'
          ? [auth.token]
          : auth.kind === 'apiKey'
            ? [auth.value]
            : auth.kind === 'oauth2'
              ? [auth.clientSecret]
              : auth.kind === 'digest'
                ? [auth.password]
                : auth.kind === 'awsSigV4'
                  ? [auth.secretAccessKey, auth.sessionToken]
                  : [];
    return credentials.some((value) => value.trim() !== '' && !/\{\{[^}]*\}\}/.test(value));
  }
</script>

<script lang="ts">
  import { i18n } from '../../lib/i18n/i18n.svelte';

  interface Props {
    auth: AuthDraft;
    /**
     * For the collection: nothing to inherit from, so "inherit" is shown as
     * "no authentication" (and an explicit "none" is not offered).
     */
    root?: boolean;
  }

  let { auth = $bindable(), root = false }: Props = $props();

  const KINDS: AuthKind[] = [
    'inherit',
    'none',
    'basic',
    'bearer',
    'apiKey',
    'digest',
    'oauth2',
    'awsSigV4',
  ];
  let kinds = $derived(root ? KINDS.filter((kind) => kind !== 'none') : KINDS);

  function label(kind: AuthKind): string {
    return i18n.t(root && kind === 'inherit' ? 'auth.kind_none' : `auth.kind_${kind}`);
  }

  function choose(event: Event) {
    auth = authOf((event.currentTarget as HTMLSelectElement).value as AuthKind, auth);
  }
</script>

<div class="auth-editor">
  <label class="auth-editor__field">
    <span class="auth-editor__label">{i18n.t('auth.kind')}</span>
    <select class="auth-editor__select" value={auth.kind} onchange={choose}>
      {#each kinds as kind (kind)}
        <option value={kind}>{label(kind)}</option>
      {/each}
    </select>
  </label>

  {#if auth.kind === 'inherit' && !root}
    <p class="auth-editor__note">{i18n.t('auth.inherit_note')}</p>
  {:else if auth.kind === 'basic'}
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.username')}</span>
      <input class="auth-editor__input" type="text" spellcheck="false" bind:value={auth.username} />
    </label>
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.password')}</span>
      <input class="auth-editor__input" type="text" spellcheck="false" bind:value={auth.password} />
    </label>
  {:else if auth.kind === 'digest'}
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.username')}</span>
      <input class="auth-editor__input" type="text" spellcheck="false" bind:value={auth.username} />
    </label>
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.password')}</span>
      <input class="auth-editor__input" type="text" spellcheck="false" bind:value={auth.password} />
    </label>
    <p class="auth-editor__note">{i18n.t('auth.digest_note')}</p>
  {:else if auth.kind === 'awsSigV4'}
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.aws_access_key')}</span>
      <input
        class="auth-editor__input"
        type="text"
        spellcheck="false"
        bind:value={auth.accessKeyId}
      />
    </label>
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.aws_secret_key')}</span>
      <input
        class="auth-editor__input"
        type="text"
        spellcheck="false"
        bind:value={auth.secretAccessKey}
      />
    </label>
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.aws_session_token')}</span>
      <input
        class="auth-editor__input"
        type="text"
        spellcheck="false"
        placeholder={i18n.t('auth.optional')}
        bind:value={auth.sessionToken}
      />
    </label>
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.aws_region')}</span>
      <input
        class="auth-editor__input"
        type="text"
        spellcheck="false"
        placeholder="eu-central-1"
        bind:value={auth.region}
      />
    </label>
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.aws_service')}</span>
      <input
        class="auth-editor__input"
        type="text"
        spellcheck="false"
        placeholder={i18n.t('auth.aws_service_placeholder')}
        bind:value={auth.service}
      />
    </label>
    <p class="auth-editor__note">{i18n.t('auth.aws_note')}</p>
  {:else if auth.kind === 'bearer'}
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.token')}</span>
      <input class="auth-editor__input" type="text" spellcheck="false" bind:value={auth.token} />
    </label>
  {:else if auth.kind === 'apiKey'}
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.key_name')}</span>
      <input class="auth-editor__input" type="text" spellcheck="false" bind:value={auth.name} />
    </label>
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.key_value')}</span>
      <input class="auth-editor__input" type="text" spellcheck="false" bind:value={auth.value} />
    </label>
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.key_placement')}</span>
      <select class="auth-editor__select" bind:value={auth.placement}>
        <option value="header">{i18n.t('auth.placement_header')}</option>
        <option value="query">{i18n.t('auth.placement_query')}</option>
      </select>
    </label>
  {:else if auth.kind === 'oauth2'}
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.grant')}</span>
      <select class="auth-editor__select" bind:value={auth.grant}>
        <option value="clientCredentials">{i18n.t('auth.grant_clientCredentials')}</option>
        <option value="authorizationCode">{i18n.t('auth.grant_authorizationCode')}</option>
      </select>
    </label>
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.token_url')}</span>
      <input class="auth-editor__input" type="text" spellcheck="false" bind:value={auth.tokenUrl} />
    </label>
    {#if auth.grant === 'authorizationCode'}
      <label class="auth-editor__field">
        <span class="auth-editor__label">{i18n.t('auth.auth_url')}</span>
        <input
          class="auth-editor__input"
          type="text"
          spellcheck="false"
          bind:value={auth.authUrl}
        />
      </label>
      <label class="auth-editor__field">
        <span class="auth-editor__label">{i18n.t('auth.redirect_url')}</span>
        <input
          class="auth-editor__input"
          type="text"
          spellcheck="false"
          placeholder={i18n.t('auth.redirect_placeholder')}
          bind:value={auth.redirectUrl}
        />
      </label>
    {/if}
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.client_id')}</span>
      <input class="auth-editor__input" type="text" spellcheck="false" bind:value={auth.clientId} />
    </label>
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.client_secret')}</span>
      <input
        class="auth-editor__input"
        type="text"
        spellcheck="false"
        bind:value={auth.clientSecret}
      />
    </label>
    <label class="auth-editor__field">
      <span class="auth-editor__label">{i18n.t('auth.scope')}</span>
      <input class="auth-editor__input" type="text" spellcheck="false" bind:value={auth.scope} />
    </label>
    <p class="auth-editor__note">
      {i18n.t(auth.grant === 'authorizationCode' ? 'auth.oauth2_sign_in_note' : 'auth.oauth2_note')}
    </p>
  {/if}

  {#if hasPlainCredential(auth)}
    <p class="auth-editor__warning" role="status">{i18n.t('auth.plain_warning')}</p>
  {/if}
</div>

<style>
  .auth-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-s);
  }

  .auth-editor__field {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    align-items: center;
    gap: var(--space-s);
  }

  .auth-editor__label {
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .auth-editor__input,
  .auth-editor__select {
    height: 30px;
    padding: 0 var(--space-s);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface);
    color: var(--color-text);
  }

  .auth-editor__input {
    font-family: var(--font-mono);
  }

  .auth-editor__note {
    margin: 0;
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }

  .auth-editor__warning {
    margin: 0;
    color: var(--color-warning);
    font-size: var(--font-size-s);
  }
</style>
