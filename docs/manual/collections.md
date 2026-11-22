# Collections

## A collection is a folder

```
my-api/
├── collection.toml        name, order, collection variables and auth
├── .age-recipients        who secrets are encrypted for (see Secrets)
├── environments/
│   ├── dev.toml
│   └── staging.toml
├── secrets/
│   └── dev.age            encrypted secret values of "dev"
├── auth/
│   ├── folder.toml        name, order, folder variables and auth
│   └── login.toml
└── health.toml
```

Every request is a small, readable TOML file; folders are directories. The
format is documented in [docs/format/v1.md](../format/v1.md). You can edit
files by hand or with other tools – the app reads them anew and keeps
comments and unknown keys when it saves.

A file the app cannot read is listed under the collection (*Not loaded*)
instead of stopping the whole collection; it is never overwritten. Files
from a newer version of the format are shown read-only.

## Organising

From the menu ⋯ of a collection, folder or request (or a right click):

- **New request**, **New folder** – inside the collection or folder.
- **Rename** – changes the display name; the file keeps its name.
- **Duplicate** – copies a request, or a folder with everything in it, right
  after the original; the copy gets new ids.
- **Move** – *Up* and *Down* within the folder, *To folder …* into another
  folder or the top level. With the mouse, drag a request or folder: the
  line above or below a row shows where it lands, the outlined folder means
  into it.
- **Delete** – moves the file or folder to the system trash, never deletes
  it for good.

The order of the tree is stored in `order` in `collection.toml` and
`folder.toml`. Entries not listed there (e.g. added by hand or by a git
merge) are shown after the listed ones, sorted by name.

## Finding requests

- **Ctrl+K** opens a search over all open collections: type words from the
  request's name, URL, method, folder or collection; arrows and **Enter**
  open the request.
- The **filter** field above the tree narrows the tree down to matching
  requests and folders.

## What folders and the collection pass on

*Collection variables & auth* (collection menu) and *Folder variables &
auth* (folder menu) open a dialog with four sections:

- **Variables** for every request inside.
- **Auth**: requests set to *Inherit* use the innermost folder's auth, else
  the collection's.
- **Scripts** that run around every request inside: before sending from the
  outside in (collection, outer folders, inner folders, the request), after
  the response in reverse order ([Scripts](../scripting.md)).
- **Settings**: the certificate check, inherited the same way.

## Unsaved collections

An [imported](import-export.md) collection first lives in a temporary folder
and is marked *not saved* in the sidebar. It disappears
when you close it or quit the app. Click *not saved* (or *Save as …* in its
menu) to move it to a folder of your choice.

## Closing

*Close collection* in its menu closes it in the app; nothing on disk
changes.
