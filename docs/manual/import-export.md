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

*From a URL (OpenAPI, Postman) …* downloads a description or collection and
imports it, e.g. `https://petstore3.swagger.io/api/v3/openapi.json`. A
relative server (`/api/v3`) is resolved against that URL.

## cURL

- *Paste cURL command …* turns a cURL command – from API documentation or
  the browser's developer tools (*Copy as cURL*) – into a new, unsaved
  request. Options without a counterpart are named.
- *Copy as cURL* in the request editor does the opposite.

## Export to Postman

*Export as Postman collection …* in a collection's menu writes the
collection as a Postman collection (v2.1) – to hand it to someone who uses
Postman. Folders, requests, headers, bodies, auth, variables, scripts (as
written) and the redirect and certificate settings of folders and requests
are included. **Secret values are never written**: secret variables are
exported without a value. What Postman has no place for – time limits, the
collection's own redirect and certificate settings – is listed afterwards.
Importing the file again gives the same collection.
