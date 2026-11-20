# ADR S4: age key handling

Status: accepted (2026-11-20). Issue #12.

## Context

Secrets in environments (tokens, passwords, API keys) must be committable to
git without being readable there, and shareable with several people and CI
([age](https://age-encryption.org/), multiple recipients). Private keys must
never end up in the collection folder.

## Decisions

1. **Own key (identity) in the app's config folder** as a plain age key file
   (`identity.txt`, e.g. `%APPDATA%\<app id>\identity.txt`), created on first
   use. It is protected by the operating system's user account, like
   `~/.ssh` keys. `API_CLIENT_IDENTITY_FILE` points to another file (other
   location, isolated end-to-end tests). A passphrase or the OS keychain may
   be added later as an option.
2. **Further identities:** the environment variable `API_CLIENT_IDENTITY`
   (content of an age key file, for CI) and the unencrypted SSH keys
   `~/.ssh/id_ed25519` and `~/.ssh/id_rsa`, if present. Passphrase-protected
   SSH keys are skipped.
3. **Recipients** in `.age-recipients` in the collection root: one public key
   per line, `age1…`, `ssh-ed25519 …` or `ssh-rsa …`; `#` starts a comment.
   If the file does not exist when the first secret is saved, it is created
   with the user's own public key.
4. **One encrypted file per environment:** `environments/dev.toml` marks a
   variable as secret (`secret = true`, no value); `secrets/dev.age` holds
   the values of all secrets of that environment (ASCII-armored age, inside a
   small versioned TOML document). The file names match by stem.
5. **The UI never receives secret values.** It shows them masked; entering a
   new value replaces the old one, an empty field keeps it. So secrets do not
   pass through the web view, its memory or its logs.
6. **Re-encryption** happens whenever secret values change, for the current
   recipients. Unchanged secrets are not rewritten (age output differs on
   every run, which would make noise in git).
7. **Not decryptable** (no matching key): plain variables keep working; using
   a secret stops the request with an error that says why. Changing secrets
   is refused until the user can decrypt them.

## Consequences

- Recipients are managed in the app (key button next to the environment
  picker): it shows the user's own public key to pass on, and saving a
  changed list re-encrypts the secrets of every environment. This happens
  all or nothing: if the user cannot decrypt one environment's secrets,
  nothing changes. Comments in `.age-recipients` survive.
- Renaming a secret variable needs its value entered again (values are
  matched by name).
- `ac_secrets` implements the `SecretVault` port of `ac_app`; tests use a
  fake vault.
