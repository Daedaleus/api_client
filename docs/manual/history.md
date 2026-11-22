# History

Every request you send is recorded with its response (or why there was
none): switch the sidebar to **History**.

- Entries are grouped by day, newest first. The filter field narrows them
  down by method, URL, status or collection.
- Clicking an entry opens it as a new, unsaved tab with the request as it
  was sent and its response.
- *Clear history* (trash icon) deletes all entries after asking.

**Privacy:** the history is kept on your computer only, in the app's data
folder – never in a collection, so it does not end up in git. Values of
secret variables are replaced by `•••`, and credentials (`Authorization`,
`Cookie`, API keys) are never recorded. The newest 200 requests are kept;
bodies up to 64 KB each.

Requests from the [command line runner](../cli.md) are not recorded.
