# LAN transfer encryption (v1)

LAN transfer serves a page to phone browsers and an API to the TieZ app over
plain HTTP. Browsers reject self-signed certificates, and React Native cannot
pin one without native code, so encryption happens in the application layer
instead of TLS. Implemented by the desktop (`src-tauri/src/services/file_transfer/sealed.rs`,
the page in `web_ui.rs`) and the TieZ app; both follow this document byte for byte.

## Key exchange

Each time the transfer server starts it creates a session token and a 32-byte
random key. The pairing QR code carries both:

```
http://<ip>:<port>/?auth=<token>#k=<key as base64url, no padding, 43 chars>
```

The token is traded for an `HttpOnly` cookie on the first request (the app
sends it as `Authorization: Bearer`). The key sits in the URL fragment, which
browsers never put on the network; the page moves it to `sessionStorage` and
removes both from the address bar. Scanning the code is the trust step: no
key ever crosses the network, so a stranger on the same Wi-Fi can neither
read nor impersonate either side (but see Compatibility).

A page opened without `#k=` asks the user to scan the code again and makes no
requests; it never falls back to plain text.

## Sealed box

```
box = "TZL1" (4 bytes) || nonce (12, random) || AES-256-GCM ciphertext || tag (16)
aad = "tiez-lan-v1/<kind>"
```

| kind | used for |
|---|---|
| `poll` | response of `GET /s/poll?last_id=N`: JSON array of messages |
| `text` | body of `POST /s/send-text`: `{content, sender_id, sender_name}` |
| `upload` | body of `POST /s/upload-chunk`: `u32 BE metadata length ‖ metadata JSON ‖ chunk bytes` (metadata as `ChunkMetadata`) |
| `ws` | every frame, both ways, of `GET /ws?s=1` (binary frames; JSON inside) |
| `download/<i>/<last>` | frame `i` of `GET /s/download/<token>` |
| `batch` | response of `POST /s/download-batch/<id>`: `{url, name}` of the packaged zip |

Request and response bodies on `/s/*` are `application/octet-stream`. A body
that does not open is rejected with 400.

## Downloads

`GET /s/download/<token>` returns the file as frames:

```
frame = u32 BE length of box ‖ box
```

Each frame holds up to 256 KiB of the file. Frame `i` is sealed with kind
`download/<i>/<1 if it is the last frame, else 0>`; an empty file is one empty
last frame. A reader decides "last" from the stream ending right after a
frame, so dropped, reordered or truncated frames fail to open. There is no
Range support, and the URL carries no file name: message links look like
`/download/<token>?name=…`, and clients rewrite them to `/s/download/<token>`
and keep the name to themselves. `X-Tiez-Size` gives the plaintext size.

Readers should hold about one frame at a time:

- The app downloads the sealed stream to a scratch file, opens it frame by
  frame into a second one, and moves that into place.
- The page has nowhere to write but browser memory (streaming to disk needs a
  secure context). Files up to 64 MiB are decrypted there and handed over as
  8 MiB blobs, which Chromium can move to disk. Larger files, pictures and
  videos go to the browser as the message's plain link, which it saves to
  disk or streams by itself. This is automatic: nothing in the UI mentions
  encryption.

## Around the protocol

- The app sends the token as `Authorization: Bearer`, on the WebSocket
  upgrade too; it never goes into a URL.
- Copying a pairing link or a file download link from the desktop marks the
  clipboard private (`org.nspasteboard.ConcealedType` on macOS;
  `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory = 0`
  and `CanUploadToCloudClipboard = 0` on Windows), so TieZ, Windows' clipboard
  history and other well-behaved tools do not keep it.
- Only TieZ's own window may call the server cross-origin (it shares dropped
  files through `/share-chunk`); other origins get no CORS headers.
- With "close automatically" on, the server stops after five minutes with no
  connected device and no request.

## Compatibility

The plaintext routes (`/poll`, `/send-text`, `/upload-chunk*`, `/download/…`,
`/ws` without `s=1`) remain for TieZ apps that predate encryption; such apps
ignore the `#k=` fragment and keep working unencrypted. The page served by the
desktop uses the sealed routes, except for files past 64 MiB. Old apps get no
warning either.

These routes take the same session token as the sealed ones, and that token
crosses the network in the clear (in the first request's `?auth=`, then in a
cookie or header). While they exist, someone who captures it can read the
message list through `/poll` and fetch files through their links. The
encryption therefore stops passive listening, not an attacker who also sends
requests; closing that gap means retiring the plaintext routes.

## Browser crypto

The page is a plain-HTTP origin, where `crypto.subtle` is unavailable. It
inlines AES-GCM from `@noble/ciphers` 1.3.0 (`assets/lan-crypto.js`, MIT) and
takes nonces from `crypto.getRandomValues`, which works in any context. To
rebuild the bundle:

```bash
echo "import { gcm } from '@noble/ciphers/aes'; globalThis.TiezLanCrypto = { gcm };" > /tmp/entry.mjs
NODE_PATH=../tiez-mobile/node_modules node_modules/.bin/esbuild /tmp/entry.mjs \
  --bundle --minify --format=iife --target=es2018 --legal-comments=none --outfile=/tmp/out.js
```

then prepend the license header kept at the top of `assets/lan-crypto.js`.
