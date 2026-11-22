# Getting started

## The window

- **Sidebar (left):** your open collections as a tree, or – switched with
  the tabs at the top – the [history](history.md). The folder button opens
  a collection, the *New* menu creates requests and collections and
  imports from other tools.
- **Tabs (top):** every open request has a tab. A dot marks unsaved
  changes.
- **Editor:** method and URL, below them the sections *Params*, *Headers*,
  *Body*, *Auth*, *Variables*, *Settings* and *Scripts*.
- **Response (below the editor):** status, time, size, and the body,
  headers, script log and test results.

## A first request

You do not need a collection to try an API: the app starts with an empty,
unsaved tab (a *scratch* request).

1. Choose the method (e.g. `GET`) and enter a URL, e.g.
   `https://petstore3.swagger.io/api/v3/pet/1`. Without `http://` or
   `https://`, `http://` is assumed.
2. Press **Send** or **Ctrl+Enter**.

Scratch requests are not saved. To keep a request, create it in a
collection (next section) – or open an [imported](import-export.md)
collection.

## Your first collection

A collection is a folder on your disk.

1. *New* → *New collection …*, choose an empty folder and a name.
2. In the sidebar, use **+** next to the collection (or its menu ⋯) to add a
   request or a folder.
3. Edit the request and press **Save** or **Ctrl+S**.

Each request is one TOML file in that folder. Put the folder under git and
your team gets every request with a normal pull. See
[Collections](collections.md).

To open an existing collection, use the folder button in the sidebar. Open
collections are opened again when the app starts.

## Reading the response

- **Status** is coloured by its class (2xx green, 4xx/5xx red), with time
  and size.
- **Body:** JSON is shown formatted with syntax colours; parts can be folded
  and **Ctrl+F** searches in it. Very large bodies are shown truncated, binary
  bodies are not shown.
- **Headers** as received.
- **Log** and **Tests** appear when [scripts](../scripting.md) wrote to the log
  or ran tests; the tab shows how many tests passed.
- **Request failed** means there was no response (e.g. the server could not
  be reached); the message says why.

When a URL contains `{{variables}}`, the line *Sent to:* below it shows the
URL as it will be sent, and unknown variables are listed.
