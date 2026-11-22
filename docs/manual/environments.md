# Variables, environments and secrets

## Variables

Write `{{name}}` in the URL, params, headers, body or auth fields; it is
replaced when the request is sent. An unknown variable stops the request
with a message naming it – nothing half-filled is sent.

Variables can be defined in several places:

| Where | Applies to | Edited in |
|---|---|---|
| Collection | every request of the collection | collection menu → *Collection variables & auth* |
| Environment | every request, while the environment is chosen | the environment picker → *Edit environment* |
| Folder | requests in the folder and its subfolders | folder menu → *Folder variables & auth* |
| Request | this request only | the request's *Variables* section |
| Scripts | following requests of the collection, until it is closed | `env.set(…)` in a [script](../scripting.md) |

### Which value wins

The more specific place wins: **collection < environment < outer folder <
inner folder < request < scripts**. So a request variable overrides the
environment, and a value a script set (e.g. a fresh token) overrides
everything. Variables set by scripts are kept in memory only and never
written to disk.

## Environments

An environment is a set of variables you switch between – *Dev*,
*Staging*, *Production*. Each is a file in `environments/`.

The environment picker sits next to the request name (requests of a
collection). **Ctrl+E** jumps to it. Choose *No environment* to send without
one. The choice is remembered per collection.

## Secrets

Tokens, passwords and API keys belong into **secret** variables of an
environment: tick *Secret* in the variable's row.

- The value is stored **encrypted** in `secrets/<environment>.age`, never in
  plain text. The environment file only says that the variable is secret.
- The app never shows a secret's value; the field stays empty. Typing a new
  value replaces it, leaving it empty keeps the old one.
- Secret values are hidden in the URL preview and in the [history](history.md).

Encryption uses [age](https://age-encryption.org/). Your own key is created
on first use in the app's configuration folder – never in the collection.

### Sharing secrets with a team (and CI)

Secrets are encrypted for every **recipient** listed in `.age-recipients`
in the collection folder, so the encrypted files can be committed and every
recipient can decrypt them.

1. A team member copies their public key: environment editor → *Secret
   recipients* → *Your public key* → *Copy*.
2. You add it under *Recipients* (an `age1…` key, or an SSH key
   `ssh-ed25519 …` / `ssh-rsa …`) and save. Every secret is encrypted anew
   for the new list.
3. Commit `.age-recipients` and `secrets/`. The team member pulls and can
   use the secrets.

Do not remove your own key from the list – the dialog warns you – or you
can no longer decrypt the secrets yourself.

For CI, create a key for the pipeline, add its public key as a recipient,
and give the pipeline the private key in the environment variable
`API_CLIENT_IDENTITY` ([command line runner](../cli.md)). Unencrypted SSH
keys in `~/.ssh` (`id_ed25519`, `id_rsa`) are used too.

If a secret cannot be decrypted (no matching key), plain variables keep
working; a request that uses the secret stops with a message saying why.
