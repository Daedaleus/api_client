# User manual

How to work with api_client (working name), the desktop API client whose
collections are plain files in a folder – made to live in git next to your
code. No account, no cloud: everything stays on your machine.

> The app is in early development; there is no download yet. To try it,
> build it from source ([CONTRIBUTING.md](../../CONTRIBUTING.md)).

## Contents

1. [Getting started](getting-started.md) – the window, a first request,
   reading the response.
2. [Requests](requests.md) – URL, params and path variables, headers,
   bodies, auth, settings.
3. [Collections](collections.md) – collections and folders on disk,
   organising them, what folders pass on, unsaved imports.
4. [Variables, environments and secrets](environments.md) – `{{variables}}`,
   which value wins, environments, encrypted secrets for a team.
5. [Scripts and tests](../scripting.md) – TypeScript before the request and
   after the response, tests, scripts from Postman and Bruno.
6. [Import and export](import-export.md) – Postman (also export),
   Insomnia, HAR, Bruno, OpenAPI, cURL.
7. [History](history.md) – every request you sent.
8. [Keyboard shortcuts](shortcuts.md)
9. [Settings](settings.md) – language, proxy, extra CA certificates,
   client certificates.
10. [Command line runner](../cli.md) – running a collection in CI.

Reference: [collection file format](../format/v1.md).

## Language and theme

The interface is in English and German; the app follows the language of the
operating system unless you choose one in the [settings](settings.md). Light and dark theme follow the system setting.
