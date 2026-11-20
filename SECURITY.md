# Security policy

## Supported versions

There is no release yet. Once there is, only the
[latest release](https://github.com/Daedaleus/api_client/releases/latest)
receives fixes.

## Reporting a vulnerability

Please **do not** open a public issue. Use GitHub's
[private vulnerability reporting](https://github.com/Daedaleus/api_client/security/advisories/new)
instead. You will get an answer within a few days; this is a community
project, so please allow some time for a fix.

**Never include real tokens, passwords or private keys** in a report.

## Security model (planned)

- The app sends only the requests the user sends; no telemetry, no account.
- Collection files never contain secrets in plain text. Secrets are
  encrypted with [age](https://age-encryption.org/) for one or more
  recipients; private keys stay on the user's machine and are never stored
  in the collection folder.
- Secrets are masked in the interface and in logs.
- Scripts run in a sandbox without file or network access beyond the
  provided API, unless the user allows it.
- TLS certificate verification is on by default; switching it off is
  clearly visible.

## Verifying downloads

Release files will be built by GitHub Actions, scanned for viruses and carry
SHA-256 checksums and a signed build provenance attestation:

```
gh attestation verify <file> --repo Daedaleus/api_client
```
