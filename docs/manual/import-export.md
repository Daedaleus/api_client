# Import and export

All imports are in the *New* menu → *Import*. An import opens as a new,
[unsaved](collections.md#unsaved-collections) collection – try it out first,
then use *Save as …* to give it a folder. What could not be taken over is
listed after the import.

## Postman

*Postman collection …* reads an exported collection (v2.1 or v2.0 JSON):
folders, requests, headers, bodies, auth, variables, path variables and
switched-off query parameters. Scripts and tests are taken over as written
and run with the `pm` object ([Scripts from Postman and
Bruno](../scripting.md#scripts-from-postman-and-bruno)).

## Insomnia

*Insomnia export …* reads an Insomnia export in format 4 (JSON or YAML;
in Insomnia: *Export* → *Insomnia v4*): request groups as folders with their
variables and auth, requests in Insomnia's order with query parameters,
headers, bodies, auth and redirect setting. The base environment gives the
collection's variables, its sub-environments become environments.
`{{ _.name }}` becomes `{{name}}`. Template tags such as
`{% response … %}` have no counterpart: they stay as text and are listed.
Scripts are taken over as written; `insomnia` is the same object as
Postman's `pm`. gRPC and WebSocket requests are not imported.

## HAR (browser recording)

*HAR file …* reads a recording from the browser's developer tools
(*Network* → *Save all as HAR*): one request per entry, in the order
recorded, with headers and body. If the file says what each entry was
(Chrome, Edge), only XHR/fetch requests – the API calls – are taken over;
images, scripts and the like are counted. Headers the program sets itself
(`Host`, `Content-Length` …) are left out. A recording may contain cookies
and tokens: check the headers before you share the collection.

## Bruno

*Bruno collection (folder) …* reads a Bruno collection folder (`.bru`
files): folders and their settings, requests, environments (secrets marked
as secret variables, without values), path and query parameters, scripts
and tests. Bruno's `assert` blocks become tests.

## OpenAPI

*OpenAPI description …* reads an OpenAPI 3 description (JSON or YAML): one
request per operation, in folders by tag. Path parameters and required query
parameters become `{{variables}}` of the request with their examples,
bodies are filled with an example, and security schemes become auth with
placeholders like `{{token}}` (put the values into an environment). The first
server becomes the collection variable `baseUrl`.

## From a URL

*From a URL (OpenAPI, Postman) …* downloads a description or collection
(any of the file formats above) and imports it, e.g. `https://petstore3.swagger.io/api/v3/openapi.json`. A
relative server (`/api/v3`) is resolved against that URL.

## cURL

- *Paste cURL command …* turns a cURL command – from API documentation or
  the browser's developer tools (*Copy as cURL*) – into a new, unsaved
  request. Options without a counterpart are named.
- *Copy as cURL* in the request editor does the opposite.

Export to other tools' formats is planned
([issues](https://github.com/Daedaleus/api_client/issues)).
