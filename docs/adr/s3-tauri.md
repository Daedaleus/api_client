# ADR S3: Tauri reality check

Status: proposed (2026-11-21), go for Tauri on Windows; Linux and macOS
still to be checked at runtime. Issue #11.

## Context

The UI runs in Tauri 2 (web view + Svelte) with egui as the fallback for a
pure-Rust UI ([architecture](../architecture.md)). Before investing heavily
in the web UI we wanted a go/no-go: does the web view keep up with large
responses, and do the program and its bundles work on all three systems?

## What was checked

Measured on Windows 11 with WebView2 157, with the release build and the
end-to-end smoke test (`mise run e2e`, step 14). The test server answers with
generated JSON; times are measured in the window, from the click on Send
until the body is in the page and painted, together with the longest gap
between two frames (a frozen window shows as a long gap).

| Response | Shown as | Until painted | Longest frame gap |
|---|---|---|---|
| 0.9 MB JSON | formatted (re-indented, ~1.3 MB of text) | ~0.5 s | ~0.5 s |
| 10 MB JSON | first 1 MB, as received (one long line) | ~0.1 s | ~0.1 s |

- **IPC:** the backend keeps the body and sends only a preview of at most
  1 MB through the event channel ([`PREVIEW_LIMIT`](../../crates/ac_app/src/view/body.rs)),
  so the size of the response barely matters: 10 MB arrive, are read and
  shown in about 0.1 s.
- **Rendering is the cost:** a formatted body of many short lines in a plain
  `<pre>` freezes the window for about half a second. A long single line is
  cheap. A viewer that renders only the visible lines (CodeMirror 6, already
  planned for the script editor) removes this; until then it is acceptable.
- **Windows runtime:** the release executable imports no `VCRUNTIME140.dll`
  (Tauri links the Visual C++ runtime statically; the universal CRT is part
  of Windows 10/11), so no redistributable is needed. It needs the WebView2
  runtime, which Windows 11 and current Windows 10 ship.
- **Size:** release executable 17.4 MB (no installer yet).
- **Linux and macOS:** CI compiles, lints and tests the desktop crate on
  `ubuntu-latest` (WebKitGTK 4.1) and `macos-latest` on every PR, so the
  program builds there. It has **not been started** there yet.

## Not checked yet

- **Linux at runtime** (WebKitGTK on Wayland and X11) and **macOS** (WKWebView):
  covered by running the end-to-end test on Linux in CI with `tauri-driver`
  (#24); macOS manually or later in CI.
- **Bundles** (MSI/NSIS, AppImage/deb, dmg): `tauri build` without
  `--no-bundle`. On Windows it downloads WiX and NSIS on first use; on
  Linux and macOS it belongs in CI. Part of the release pipeline (#8).
- **Older WebView2 versions** and Windows 10 without WebView2 (the installer
  can bundle the bootstrapper).

## Decision (proposed)

1. **Stay with Tauri.** On Windows nothing speaks against it: large
   responses are fast because only a preview crosses the IPC boundary, and
   the one slow spot (rendering many lines) has a known fix in the web
   stack. egui stays the fallback but is not pursued.
2. **Show response bodies in a virtualised viewer** (CodeMirror 6, read-only)
   before raising `PREVIEW_LIMIT`. *Done (2026-11-22):* the 0.9 MB formatted
   body now appears after ~0.08 s with no frame longer than ~17 ms (was
   ~0.5 s frozen). Bodies with a line longer than 20 000 characters (minified
   JSON) stay plain text: the viewer draws whole lines, and such a line
   would freeze it.
3. **Confirm Linux and macOS at runtime** before the first release: the
   end-to-end test on Linux in CI (#24) and a manual start on macOS. If
   WebKitGTK turns out unusable, this ADR is reopened.
4. Step 14 of the end-to-end test stays as a regression check: a body that
   takes more than 5 s to appear or freezes the window for more than 2 s
   fails it.
