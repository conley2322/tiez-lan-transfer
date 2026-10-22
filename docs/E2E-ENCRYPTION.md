# End-to-end encryption (v1)

Optional, and not offered to new setups: the settings pages (desktop and
mobile) show it only on a device that already has a key, so it can be turned
off. TieZ relies on TLS to the broker and the WebDAV server and trusts the
server the user picked. When a key is set, MQTT messages and WebDAV sync files are encrypted
on the sending device and only devices holding the same key can read them. The
broker and the WebDAV server see ciphertext. Implemented by the desktop app
(`src-tauri/src/services/e2e.rs`) and the mobile app (`tiez-mobile/src/lib/e2e.ts`);
both must follow this document byte for byte.

LAN transfer has its own per-session key and protocol: see
[LAN-TRANSFER-ENCRYPTION.md](LAN-TRANSFER-ENCRYPTION.md).

## Key

- 32 random bytes, written as **base64url without padding** (43 characters).
  Parsers also accept standard base64 and padding.
- Generated on the desktop; shared through the config QR codes (`e2eKey`) or
  pasted by hand. Stored with the other secrets (desktop: encrypted setting
  `e2e_key`, never synced; mobile: SecureStore).
- Fingerprint, shown so users can compare devices: first 4 bytes of
  `SHA-256(key)`, uppercase hex, as `XXXX-XXXX`.

Purpose keys, so one key never serves two jobs:

```
mqtt_key      = HMAC-SHA256(key, "tiez-e2e-v1 mqtt")
webdav_key    = HMAC-SHA256(key, "tiez-e2e-v1 webdav")
blob_name_key = HMAC-SHA256(key, "tiez-e2e-v1 blob-name")
```

## Sealed box

AES-256-GCM, a fresh random 12-byte nonce per message, 16-byte tag.
Associated data is the ASCII string `tiez-e2e-v1/<kind>`.

```
box = "TZE1" (4 bytes) || nonce (12) || ciphertext || tag (16)
```

| kind       | used for                                  | key        |
|------------|-------------------------------------------|------------|
| `mqtt`     | clipboard messages                        | mqtt_key   |
| `snapshot` | `devices/<device>.json`                   | webdav_key |
| `ops`      | op batches                                | webdav_key |
| `settings` | `settings/<device>.json`                  | webdav_key |
| `blob`     | `blobs/...` (image, long text, HTML)      | webdav_key |

## MQTT

The payload is text: `tze1:` followed by the standard base64 (with padding) of
the box. The plaintext is the usual JSON message plus a random id:

```json
{ "msg": "...", "sender": "mobile", "timestamp": 1760000000000, "id": "9f3c…" }
```

Receivers drop messages whose `timestamp` is more than 10 minutes away from
their clock, and ids they have already seen (replay protection). With a key
set, plaintext messages are rejected; without one, `tze1:` messages are
ignored.

## WebDAV

- Every file except the sync head (`head.json`, which holds only device ids,
  sequence numbers and times) is uploaded as a box of its JSON / blob bytes.
- Blob names: with a key, `hex(HMAC-SHA256(blob_name_key, plaintext bytes))`
  instead of `hex(SHA-256(plaintext bytes))`, so file names don't reveal
  content hashes. Items reference blobs by this name as before.
- Reading: a body starting with `TZE1` is opened with the key. With a key set,
  plaintext files (from a device without the key) are skipped; without a key,
  sealed files are skipped. A file that fails to open (different key) is
  skipped too. Skipping never fails the whole sync; the app reports it.

## Compatibility

Shown to users in the encryption settings, because none of it raises an error:

- Versions without encryption support can't read sealed content. An older
  desktop build fails to parse sealed WebDAV files and aborts the whole sync,
  its own uploads included; on MQTT it takes the `tze1:` text as the message
  and copies it. Update every device before setting a key.
- With a key set, plaintext MQTT messages are dropped (logged only), so
  third-party senders such as SMS forwarders stop getting through.
- While devices hold different keys, unreadable files are fetched and retried
  on every sync, which can trip WebDAV rate limits.

## Versioning

The `TZE1` / `tze1:` prefix names the format. A future format gets a new
prefix; readers skip prefixes they don't know.
