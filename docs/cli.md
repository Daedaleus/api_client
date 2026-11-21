# Command line runner

`api-client-cli` runs the requests of a collection one after another – all of
them, a folder or one request – with an environment, and reports what
happened. It is meant for CI pipelines and scripts.

```sh
api-client-cli run ./my-api --env Staging
api-client-cli run ./my-api --env Staging --only Auth/Login
api-client-cli run ./my-api --env Staging --format junit > report.xml
```

| Option | Meaning |
|---|---|
| `--env <name>` | The environment to use (by its name). |
| `--only <path>` | Only the folder or request at this path of names, e.g. `Auth` or `Auth/Login`. |
| `--format text\|json\|junit` | Text for people (default), JSON for programs, JUnit XML for CI systems. Everything goes to standard output. |
| `--allow-error-status` | A status of 400 or more does not count as failed. |

Requests run in the order of the collection tree, each exactly as the app
would send it: variables of every level, inherited auth, certificate check.
A request that cannot be sent (an unknown variable, a missing file) is
reported as failed and the run goes on.

**Exit code:** `0` when every request passed, `1` when one failed (no
response, or a status of 400 or more), `2` when the run could not start
(no collection there, unknown environment, nothing matches `--only`).

**Secrets** are decrypted with the user's own key (the same as the desktop
app's). In CI, put the content of an age key file into the environment
variable `API_CLIENT_IDENTITY` and add its public key to the collection's
recipients ([ADR S4](adr/s4-age-keys.md)).

**OAuth 2:** client credentials work as in the app. The authorization code
opens the browser for signing in, so it is meant for runs on a developer's
machine, not for CI.

**Language:** texts follow `LC_ALL`, `LC_MESSAGES` or `LANG` (German or
English); JSON and JUnit are the same in every language, apart from error
messages.

**Scripts and tests** run as in the app ([scripting](scripting.md)):
variables a script sets carry over to the following requests of the run. A
request passes only when it got a response (below 400 unless
`--allow-error-status`), its script after the response did not fail and all
its tests passed. The text report lists each test under its request, JSON
has them per result, and JUnit has one test case per request plus one per
test.
