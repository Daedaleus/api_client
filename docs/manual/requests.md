# Requests

## Method and URL

Any method can be used, also non-standard ones like `PROPFIND`. The URL may
contain `{{variables}}` anywhere ([Variables](environments.md)).

## Params

The *Params* section shows two tables:

- **Query parameters** – the `?name=value` pairs of the URL. The table and
  the URL stay in step: typing in either changes the other. Switching a
  parameter off removes it from the URL; it is kept with the request (not
  sent) and can be switched on again.
- **Path variables** – a path segment starting with a colon, as in
  `{{baseUrl}}/users/:id`, becomes a path variable with its own value field.
  When sending, `:id` is replaced by the value (which may contain
  `{{variables}}`). A port like `localhost:8080` is not a path variable.

## Headers

Rows of name and value; switched-off rows are kept but not sent. Names and
values may contain `{{variables}}`. A JSON or text body gets a matching
`Content-Type` automatically unless you set one.

## Body

| Type | What is sent |
|---|---|
| None | No body. |
| JSON | The text as typed (checked for valid JSON while typing), `Content-Type: application/json`. |
| Text | The text as typed, `Content-Type: text/plain`. |
| Form | URL-encoded fields (`application/x-www-form-urlencoded`). |
| Multipart | Text fields and files (`multipart/form-data`). |
| File | The contents of a file, as they are. |

File paths can be relative: they start at the collection folder, so the
request works for everyone on the team who has the files in the same place.

## Auth

| Type | What happens |
|---|---|
| Inherit | The auth of the innermost folder that sets one, else the collection's. |
| None | No authentication, even if folders or the collection set one. |
| Basic | User name and password as `Authorization: Basic …`. |
| Bearer token | `Authorization: Bearer <token>`. |
| API key | A header or query parameter with the key. |
| OAuth 2 | A token is fetched and sent as bearer token: *client credentials*, or *authorization code* where you sign in in your browser (with PKCE). Tokens are kept in memory until they expire. |

Put credentials into [secret variables](environments.md#secrets) and use
`{{token}}` in the auth fields, so they are never stored in plain text.

## Variables

Variables of this request only; they override all others except the ones
scripts set ([which value wins](environments.md#which-value-wins)).

## Settings

**Check TLS certificate:** on by default. Switching it off (per request,
folder or collection) accepts any certificate – only for test servers with
self-signed certificates. A warning stays visible while it is off.

## Scripts

TypeScript or JavaScript before the request and after the response, with
tests: see [Scripts and tests](../scripting.md).

## Copy as cURL

*Copy as cURL* puts the request – variables filled in, inherited auth
applied – on the clipboard as a cURL command.
