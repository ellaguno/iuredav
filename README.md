# IureDav

[Leer en español](README.es.md)

[![CI](https://github.com/ellaguno/iuredav/actions/workflows/ci.yml/badge.svg)](https://github.com/ellaguno/iuredav/actions/workflows/ci.yml)

Mount a WebDAV server as a drive on your computer, Mountain Duck style.
Linux, macOS and Windows. With a profile optimized for **Iurefficient**.

Unlike other clients, IureDav **doesn't believe what the server advertises**:
it tests every operation, measures what really works, and from that works out how
to mount the drive and what to explain to you when something can't be done.

> **Status: in development.** The probe, mounting, the desktop interface, the
> system tray, autostart, the installers for all three platforms and offline
> folders all work.
>
> Only really tested on Linux. Windows and macOS **build and are packaged in
> continuous integration**, but nobody has run them on a real machine yet.

## Language

The interface is available in English and Spanish. It starts in **Spanish if the
operating system is in Spanish** and in English otherwise; you can change it in
the preferences card (*Language / Idioma*: automatic, English or Español). The
change applies immediately, including the tray menu and the notices coming from
the mount.

## Two kinds of server

| Profile | For | What it asks for |
|---|---|---|
| **Iurefficient** | Iurefficient instances | Just the domain; `/webdav/` is added. An `iurdav_…` app password |
| **Other WebDAV server** | Nextcloud, ownCloud, Synology, Seafile, `mod_dav`… | The full URL of your WebDAV and your password |

Only **basic authentication over HTTPS** is supported. There is no support for
NTLM (SharePoint) or OAuth flows, and none is planned.

## Why the probe exists

Iurefficient's WebDAV server isn't a WebDAV over files: it's a custom provider
that exposes a virtual tree backed by the document database. And it **advertises
capabilities it doesn't have**: its `Allow:` header promises `DELETE`, `COPY`,
`MOVE` and `PROPPATCH`, but using them returns 403, 403, 502 and 403.

That isn't a cosmetic detail. A client that believes that advertisement plans with
it and fails when executing. With rclone it looks like this:

```
$ rclone backend features :webdav:          # unprotected
  Move     True        ← lie
  DirMove  True        ← lie
  Copy     True        ← lie
  Purge    True        ← lie

$ rclone moveto ...
ERROR : Server side directory move failed: DirMove MOVE call failed: 502 Bad Gateway
ERROR : Attempt 1/3 failed with 1 errors
```

That's why IureDav **doesn't read the advertisement to decide anything**. It tests
every verb, measures what the server really does, and from that measurement works
out how to mount the drive. If tomorrow the server fixes `DELETE`, the probe
detects it and the restriction goes away on its own.

## Usage

```bash
cargo build --workspace

# 1. See what your server can really do, and save the connection
IUREDAV_PASS='iurdav_...' ./target/debug/iuredav probe \
  --url https://YOUR-INSTANCE/webdav/ \
  --user you@email.com \
  --guardar-como work

# 2. Mount it (Ctrl+C to unmount)
./target/debug/iuredav mount work

# 3. Manage connections
./target/debug/iuredav perfiles
./target/debug/iuredav olvidar work
```

The password is stored in the system keychain, never in the profiles file. Add
`--escritura` to `probe` to also test `PUT`, `MKCOL`, `MOVE`, `DELETE`,
`PROPPATCH` and `LOCK`. (The diagnostic command line is in Spanish.)

**The mount is read-only until the probe confirms that the server accepts
writes**, and even then you have to ask for it with `--escritura`. Against
Iurefficient that's the right thing: deleting and renaming don't work, and every
save creates a new version of the document.

> **Careful with `--escritura`:** since the server rejects `DELETE`, the probe
> **can't clean up what it creates**. That's why it always writes to the same
> fixed path, `General/.iuredav-selftest.txt`, so that repeating the diagnostic
> generates versions of a single document instead of piling up orphan files.

The password is passed through `IUREDAV_PASS` on purpose: any other process on
the machine can read `argv`.

### Without touching a real instance

There's a test double that mimics the measured behavior, lie included:

```bash
python3 tests/servidor-falso.py 8099 &
IUREDAV_PASS='iurdav_falso' ./target/debug/iuredav probe \
  --url http://127.0.0.1:8099/webdav/ --user prueba@ejemplo.com --escritura
```

It should report 4 advertised capabilities that don't exist, and exit with code 1.
That disagreement between the two columns *is* the proof that it works.

## How it's put together

| Piece | What it does |
|---|---|
| `crates/iuredav-core/src/probe.rs` | Tests every WebDAV verb and produces the measurement. |
| `crates/iuredav-core/src/caps.rs` | Turns the measurement into rclone options. |
| `crates/iuredav-core/src/errors.rs` | Translates rclone failures into plain language, in English or Spanish. |
| `crates/iuredav-core/src/rclone.rs` | Supervises the sidecar that does the mounting. |
| `crates/iuredav-core/src/perfiles.rs` | Saved connections, with no secrets inside. |
| `crates/iuredav-core/src/secretos.rs` | Passwords in the system keychain. |
| `crates/iuredav-cli` | The `iuredav` binary. |

The mount is done by [rclone](https://rclone.org) as a helper process, controlled
through its remote API. Neither the WebDAV password nor that API's credentials go
through the command line: any user on the machine can read `/proc/PID/cmdline`
(permissions 444), while only its owner can read the environment (400). The names
and types of the options come from `rclone rc --loopback options/get`, which is
the authoritative list: durations are in nanoseconds, `CacheMode` is an integer
and the read chunk key is `ChunkSize`. Getting any of those three wrong breaks the
mount silently.

### How it mounts on each system

| System | Mechanism | Needs installing | Where it appears |
|---|---|---|---|
| Linux | FUSE 3 | `fuse3` (the `.deb` requires it) | `~/Iurefficient` |
| macOS | rclone's local NFS server | **nothing** | `~/Iurefficient` |
| Windows | WinFsp | **nothing**: the installer bundles it and installs it if missing | drive `I:` |

macOS doesn't need macFUSE. rclone starts a local NFS server and the system mounts
it, so nobody has to install a kernel extension or authorize it in System
Preferences — which is the biggest installation hurdle for this kind of program.

The mechanism **isn't hard-coded**: when mounting, rclone is asked which
mechanisms it has (`mount/types`) and the first one from a per-platform preference
list is chosen. The names change between versions — `nfsmount` doesn't exist in
rclone 1.60 but does in 1.75 — and asking for one that isn't there gives an error
that doesn't help.

IureDav checks these requirements **before** trying to mount, and if one is
missing it explains which one and how to get it, instead of letting rclone fail
with a system message.

## The desktop app

```bash
npm install
npm run tauri dev
```

The interface turns the server's limits into something actionable: instead of
"doesn't allow delete, mkcol, move" it says **"it doesn't let you delete
documents, create folders or move or rename; those operations are done in
Iurefficient"**. The *See what this server can do* screen sets, row by row, what
the server advertises against what it delivers, and from there you can **check it
again**: the measurement is saved with the connection and doesn't expire, so a
server that changes isn't detected on its own.

The new-connection form probes **reading only**, so as not to leave a trace on a
server that may never even be saved. That leaves `PUT` as "not tested", and
without a confirmed `PUT` the mount is forced to read-only: that's why turning on
*Allow editing* offers to check uploads first, warning about the diagnostic file
that leaves on the server.

## Offline folders

Mark a folder and it's downloaded in full so you can open it without internet.
It's checked again every time you mount.

rclone **has no native "pin"**: what it has is a cache with an expiry and a
maximum size. IureDav walks the folder and reads every file through the mount,
which is what forces the VFS to download them and keep them in that cache. It's a
good approximation, not a guarantee: **if the cache fills up, eviction by age can
push out pinned content**.

The alternative approach — syncing to a real local folder — is ruled out
precisely because of what the probe measures: with no content fingerprint and no
reliable date, the comparison would fall back to looking at size only, which
doesn't detect an edited file that weighs the same.

## It lives in the tray

Like any agent of this kind, the usual thing is to mount when the computer starts
and not open the window again for weeks. That's why **closing the window doesn't
unmount or stop the program**: it just hides it. To really quit there's *Quit* in
the tray menu, which unmounts first.

If the desktop doesn't offer a tray, IureDav detects it and closing the window
does quit the program — otherwise there would be no way out.

With *Start when you sign in* turned on, IureDav starts **minimized**: only the
tray icon appears, without opening the window. That's the default, and it can be
turned off right below that option. It only affects starting with the session —
the autostart entry launches the program with `--autoarranque` —; opening it by
hand always shows the window, and without a tray it never hides either.

## It tells you about new versions

At startup, and once a day while it stays in the tray, IureDav checks the latest
release of this repository. If it's newer than the installed one it says so in the
window and in the tray menu, with a link to the download page. **It only
notifies**: it doesn't download or install anything. The check is anonymous and
can be turned off in the preferences card.

## Building the installers

```bash
python3 scripts/descargar-rclone.py    # the rclone that gets bundled
npm ci
npm run tauri build
```

Produces `.deb`, `.rpm` and `.AppImage` on Linux; `.dmg` on macOS; `.msi` and
`.exe` on Windows. Continuous integration builds them for Linux x86-64, Windows
x86-64 and macOS, and keeps them as artifacts of each run.

The macOS `.dmg` is **universal**: a single installer valid for both Intel and
Apple Silicon. It's built on an Apple Silicon runner because GitHub's Intel ones
are being retired and jobs sit in the queue indefinitely. The two rclone binaries
are joined with `lipo` before packaging, because Tauri expects the external
binary already joined and doesn't combine it on its own.

rclone ships inside the package under the name **`iuredav-rclone`**, not
`rclone`. External binaries end up in `/usr/bin`, and there a plain `rclone` would
clash with the distribution's package: dpkg refuses to overwrite another package's
file, so installation would fail on any computer that already has it.

The macOS and Windows packages are **unsigned**. macOS will warn that the app is
from an unidentified developer, and Windows will show SmartScreen. Signing them
requires an Apple Developer account and a code-signing certificate.

## Publishing a version

```bash
python3 scripts/version.py            # see the current version and that it matches
python3 scripts/version.py 0.2.0      # change it in the three files
# write the 0.2.0 section in CHANGELOG.md
git commit -am "Versión 0.2.0"
git tag -a v0.2.0 -m "IureDav 0.2.0" && git push --follow-tags
```

The tag triggers the build for all three platforms and publishes a release with
the installers attached and the notes taken from the [CHANGELOG](CHANGELOG.md).

The tag has to be **annotated** (`-a`): `--follow-tags` doesn't push lightweight
ones, so with a plain `git tag v0.2.0` the `push` takes the commit, leaves the tag
on your machine and doesn't warn you. Nothing is published and it looks like it
was.

The version lives in three files — `Cargo.toml`, `package.json` and
`tauri.conf.json` — because each tool has its own. If they drift apart you get an
installer that says one version and contains another: the file name and the MSI
properties come from `tauri.conf.json`, not from the code. That's why CI checks
that they match, and publishing stops before building anything if the tag doesn't
match what the files say.

The artifacts each CI run leaves **expire after 14 days**; those of a release
don't expire.

## Development

```bash
cargo test --workspace     # capabilities, error translator, profiles
cargo build --workspace
npx tsc --noEmit           # the frontend, in strict mode
python3 scripts/generar-iconos.py   # regenerates the icons from assets/
```

On Linux, the system tray needs a package that isn't installed yet:

```bash
sudo apt install libayatana-appindicator3-dev
```

## The icon

It comes from `assets/iuredav_icon.png`. The script crops it, squares it and
scales it down to every size the packagers ask for.

Sizes below 64 px — system tray, taskbar, tab — use **only the mark, without the
"WebDAVs" wordmark**: at 16 px that text is an illegible smudge and it also takes a
third of the height, leaving the mark cramped. The Windows `.ico` is written by
hand so it can carry different art at each resolution, which is something Pillow
can't do.

Corners are rounded at 22 %, which is the radius of macOS icons. The tray one is
separate and **fully round**: there it sits next to the system icons, which are
circular on all three desktops. Rounding is always done at the final size —
rounding and then scaling down blurs the edge and leaves a halo of the background
color — and the Windows Store tiles stay square on purpose, because there the
system sets the shape.

## A note on data

This repository is public. Never include instance URLs, emails, `iurdav_…`
passwords or probe reports: the `Casos/` listings carry real case-file names. The
tests use the fake server, never a real instance.

## Code signing policy

Free code signing provided by [SignPath.io](https://signpath.io), certificate by
[SignPath Foundation](https://signpath.org).

*This is what makes Windows show a known publisher instead of the SmartScreen
warning.*

- **Committers and reviewers:** Eduardo Llaguno ([@ellaguno](https://github.com/ellaguno)).
- **Approvers:** Eduardo Llaguno ([@ellaguno](https://github.com/ellaguno)).
- Every Windows release is built from this repository by GitHub Actions
  (`.github/workflows/release.yml`), submitted to SignPath from that workflow and
  approved manually before it is signed. Only the installer published on the
  [releases page](https://github.com/ellaguno/iuredav/releases) is signed.

### Privacy policy

This program will not transfer any information to other networked systems unless
specifically requested by the user or the person installing or operating it.

Specifically, IureDav connects only to:

- the WebDAV server (an Iurefficient instance or any other) that the user
  configures, to mount it as a drive; the transfer is done by the bundled
  [rclone](https://rclone.org) (MIT), which talks only to that server;
- `api.github.com`, once at start-up and once a day while it stays in the tray, to
  check whether a newer release exists. This check is anonymous, downloads
  nothing and can be turned off in the preferences card.

It collects no telemetry and no usage statistics. Credentials are stored in the
operating system keychain, never in configuration files.

## License

MIT — see [LICENSE](LICENSE).
