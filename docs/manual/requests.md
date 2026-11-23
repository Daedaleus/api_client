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

**Redirects:** followed by default (up to 10), and the last response is
shown. *Do not follow* shows the redirect response itself (3xx with its
`Location` header).

**Time limit:** how many seconds the whole request may take, including
reading the response; 30 by default. A request that takes longer fails with
a time-out.

**Cookies:** on by default – see [Cookies](#cookies). *Off* neither sends
the collection's cookies nor keeps the ones the server sets.

All these settings are inherited: a request set to *Inherit* (or with an
empty time limit) uses the innermost folder that sets it, else the
collection, else the default. Folders and the collection set them in their
*Settings* section.

## Cookies

Cookies a server sets are kept – one cookie jar per collection, and one for
requests outside a collection – and sent with the following requests like a
browser would: to the matching domain and path, not after they expire,
`Secure` ones only over HTTPS (or to your own machine). So a login that
sets a session cookie works for the requests after it, also when the
cookie comes with a redirect. The CLI runner keeps cookies during a run.

Cookies stay in memory only: they are never written into the collection
and are gone when the collection is closed or the app ends. A `Cookie`
header you set on a request yourself replaces the jar's cookies for that
request.

## Scripts

TypeScript or JavaScript before the request and after the response, with
tests: see [Scripts and tests](../scripting.md).

## Copy as cURL or code

*Copy as cURL* puts the request – variables filled in, inherited auth
applied – on the clipboard as a cURL command. *As code …* next to it copies
the same request as code: JavaScript (`fetch` or axios), Python
(`requests`), Go (`net/http`) or HTTPie. Redirects, the time limit and a
switched-off certificate check carry over; files are referenced by their
path. An OAuth 2 token is never fetched for this – the code contains a
placeholder instead.
