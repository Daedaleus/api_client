<script lang="ts">
  import { i18n } from '../../lib/i18n/i18n.svelte';
  import {
    complete,
    placeholdersIn,
    suggestionAt,
    suggestionsFor,
    type Suggestion,
    type VariableInfo,
  } from '../../lib/variables';

  interface Props {
    value: string;
    /** The variables that apply, for suggestions after `{{` and the tooltip. */
    variables: ReadonlyMap<string, VariableInfo>;
    /** CSS class of the input (the caller's design). */
    class?: string;
    label: string;
    placeholder?: string;
    type?: 'text' | 'password';
    /** Marked as invalid (e.g. the request failed because of it). */
    invalid?: boolean;
  }

  let {
    value = $bindable(),
    variables,
    class: className = '',
    label,
    placeholder = '',
    type = 'text',
    invalid = false,
  }: Props = $props();

  let input = $state<HTMLInputElement | null>(null);
  let open = $state<Suggestion | null>(null);
  let selected = $state(0);
  const listId = `variable-field-${Math.random().toString(36).slice(2)}`;

  let names = $derived(open ? suggestionsFor(variables, open.typed) : []);
  let placeholders = $derived(placeholdersIn(value, variables));
  let unknown = $derived(placeholders.some((entry) => entry.info === null));

  /** The variables of the field and their values, for the tooltip. */
  let tooltip = $derived(
    placeholders
      .map(({ name, info }) =>
        info
          ? `${name} = ${info.value} (${i18n.t(`variables.source_${info.source}`)})`
          : `${name}: ${i18n.t('variables.unknown')}`,
      )
      .join('\n'),
  );

  function update() {
    if (!input) return;
    open = suggestionAt(value, input.selectionStart ?? value.length);
    selected = 0;
  }

  function accept(name: string) {
    if (!input || !open) return;
    const result = complete(value, input.selectionStart ?? value.length, open, name);
    value = result.text;
    open = null;
    queueMicrotask(() => {
      input?.focus();
      input?.setSelectionRange(result.caret, result.caret);
    });
  }

  function keydown(event: KeyboardEvent) {
    if (!open || names.length === 0) return;
    switch (event.key) {
      case 'ArrowDown':
        selected = (selected + 1) % names.length;
        break;
      case 'ArrowUp':
        selected = (selected - 1 + names.length) % names.length;
        break;
      case 'Enter':
      case 'Tab':
        accept(names[selected]!);
        break;
      case 'Escape':
        open = null;
        break;
      default:
        return;
    }
    event.preventDefault();
    event.stopPropagation();
  }
</script>

<span class="variable-field">
  <input
    class="{className} variable-field__input"
    class:variable-field__input--unknown={unknown}
    {type}
    spellcheck="false"
    autocomplete="off"
    aria-label={label}
    aria-invalid={invalid}
    aria-autocomplete="list"
    aria-expanded={names.length > 0}
    aria-controls={listId}
    aria-activedescendant={names.length > 0 ? `${listId}-${selected}` : undefined}
    title={tooltip || undefined}
    {placeholder}
    bind:value
    bind:this={input}
    oninput={update}
    onkeydown={keydown}
    onblur={() => (open = null)}
  />
  {#if names.length > 0}
    <ul
      id={listId}
      class="variable-field__list"
      role="listbox"
      aria-label={i18n.t('variables.suggestions')}
    >
      {#each names as name, index (name)}
        {@const info = variables.get(name)}
        <li
          id="{listId}-{index}"
          class="variable-field__option"
          class:variable-field__option--selected={index === selected}
          role="option"
          aria-selected={index === selected}
          onpointerdown={(event) => {
            // Keep the focus in the field.
            event.preventDefault();
            accept(name);
          }}
        >
          <span class="variable-field__name">{name}</span>
          {#if info}
            <span class="variable-field__value">{info.value}</span>
            <span class="variable-field__source">{i18n.t(`variables.source_${info.source}`)}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</span>

<style>
  .variable-field {
    position: relative;
    display: block;
    flex: 1;
    min-width: 0;
  }

  .variable-field__input {
    width: 100%;
  }

  .variable-field__input--unknown {
    text-decoration: wavy underline var(--color-error);
    text-decoration-skip-ink: none;
  }

  .variable-field__list {
    position: absolute;
    top: calc(100% + 2px);
    left: 0;
    z-index: 4;
    min-width: 260px;
    max-width: 480px;
    max-height: 240px;
    margin: 0;
    padding: var(--space-xs) 0;
    overflow-y: auto;
    list-style: none;
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    background: var(--color-surface);
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.25);
  }

  .variable-field__option {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: var(--space-s);
    align-items: baseline;
    padding: var(--space-xs) var(--space-s);
    cursor: pointer;
  }

  .variable-field__option--selected {
    background: var(--color-raised);
  }

  .variable-field__name {
    font-family: var(--font-mono);
  }

  .variable-field__value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-muted);
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
  }

  .variable-field__source {
    color: var(--color-muted);
    font-size: var(--font-size-s);
  }
</style>
