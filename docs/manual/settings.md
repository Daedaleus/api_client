# Settings

The gear button in the sidebar opens the app's settings. They belong to
this computer, not to a collection, and apply to every request of every
collection. They are kept in `settings.toml` in the app's configuration
folder – never in a collection, so they do not end up in git.

## Language

German or English. By default the app follows the operating system.

## Network

**Proxy**

| Choice | What happens |
|---|---|
| As set in the system | The operating system's proxy settings and the variables `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY` (default). |
| No proxy | Every request goes directly to the server. |
| Own proxy | Requests go through the proxy URL you enter (`http://…`, `https://…` or `socks5://…`). *Without proxy for* lists hosts reached directly, separated by commas (e.g. `localhost,.internal`). |

**Extra CA certificates** – PEM or DER files with certificates of
certificate authorities to trust in addition to the system's, e.g. your
company's CA for internal servers. A PEM file may contain several
certificates.

**Client certificates (mTLS)** – for servers that ask for a certificate
from you. Each entry has

- a **host**: `api.internal`, `api.internal:8443` (only that port) or
  `*.internal` (all subdomains); the first matching entry is used,
- the **certificate**: a PEM file (certificate chain, the key may be in the
  same file) or a PKCS#12 file (`.p12`, `.pfx`) with certificate and key,
- the **key**: a separate PEM key file, if the key is not in the
  certificate file,
- an optional **passphrase** for an encrypted PEM key (`ENCRYPTED PRIVATE
  KEY`) or the PKCS#12 file. It is stored only encrypted with your own age
  key – the same key as for [secrets](environments.md) – never in plain
  text. Keys encrypted the old OpenSSL way (`Proc-Type: 4,ENCRYPTED`) are
  refused; convert them with `openssl pkcs8 -topk8 -in key.pem -out
  key-pkcs8.pem`.

Settings are checked before they are saved: an invalid proxy URL or a
certificate file that cannot be read is refused with the reason, and the
previous settings stay in effect. If saved settings cannot be applied when
the app starts (e.g. a certificate file was moved), requests go out with the
defaults and a message says why.

