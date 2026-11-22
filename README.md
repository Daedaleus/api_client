# api_client

[![CI](https://github.com/Daedaleus/api_client/actions/workflows/ci.yml/badge.svg)](https://github.com/Daedaleus/api_client/actions/workflows/ci.yml)
[![Audit](https://github.com/Daedaleus/api_client/actions/workflows/audit.yml/badge.svg)](https://github.com/Daedaleus/api_client/actions/workflows/audit.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A fast, fully open-source desktop API client written in Rust – an
alternative to tools like Bruno or Postman. **No premium tier, no account,
no cloud:** every feature is available to everyone, and your collections stay
on your machine.

> **Status:** early development. There is nothing to download yet – to try
> it, build it from source ([CONTRIBUTING.md](CONTRIBUTING.md)). The product
> name is not final; `api_client` is the working name.

**[User manual](docs/manual/README.md)** – getting started, requests,
collections, environments and secrets, scripts, import, shortcuts.

## Features

- **Collections as plain files** – one human-readable TOML file per request
  in a folder, made to be versioned in git next to your code
  ([format](docs/format/v1.md)).
- **Requests** – any method, query parameters and path variables (`/users/:id`)
  as tables, headers, bodies (JSON, text, form, multipart, file) and auth
  (Basic, Bearer, API key, OAuth 2 with client credentials or browser sign-in).
- **Variables and environments** – `{{variable}}` in every field, defined on
  the collection, environments, folders, the request or by scripts; a
  preview shows the URL as it will be sent.
- **Secrets encrypted with [age](https://age-encryption.org/)** for several
  recipients, so a team (and CI) can share them safely through git.
- **TypeScript scripts and tests** before and after a request, also on
  folders and the collection, sandboxed, with type checking and completion in
  the editor ([scripting](docs/scripting.md)).
- **Response viewer** – status, time, size, headers, formatted and foldable
  JSON with search, script log and test results.
- **Import** from Postman, Bruno (including their scripts) and OpenAPI, also
  straight from a URL; **cURL** import and export.
- **Request history** – every sent request with its response, on your
  machine only, secrets hidden.
- **Command line runner** for CI with JUnit and JSON reports
  ([CLI](docs/cli.md)).
- Keyboard shortcuts, interface in **English and German**, light and dark
  theme.

Planned next: a collection runner in the app, cookies, more auth types, request chains as automated tests
([all ideas](https://github.com/Daedaleus/api_client/issues?q=is%3Aopen+label%3Aenhancement)).

## Privacy and security

- No telemetry, no account, no sync – nothing about you leaves your machine.
- The app only sends the requests you send.
- Secrets are never stored in plain text in collection files.

Found a vulnerability? Please see [SECURITY.md](SECURITY.md).

## Contributing

Building, tests and conventions: [CONTRIBUTING.md](CONTRIBUTING.md).
Architecture: [docs/architecture.md](docs/architecture.md). Ideas and bugs go
into [issues](https://github.com/Daedaleus/api_client/issues).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

"Bruno" and "Postman" are names of third-party products; this project is not
affiliated with them.
