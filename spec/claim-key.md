<!-- SPDX-License-Identifier: AGPL-3.0-only -->
# Claim and slot keys

Umwelt derives stable lowercase hexadecimal SHA-256 keys from UTF-8 encoded canonical JSON.

## `claimKey`

The `claimKey` input object contains exactly these fields:

```json
{
  "subject": "...",
  "predicate": "...",
  "object": {},
  "qualifiers": {},
  "about": null
}
```

`polarity` is deliberately excluded. Claims that differ only in polarity therefore have the same `claimKey`.

## `slotKey`

The `slotKey` input object contains exactly `subject` and `predicate`:

```json
{"subject":"...","predicate":"..."}
```

## Canonical JSON

Canonical JSON is produced recursively with these rules:

- Object keys are sorted by Unicode code point order.
- No insignificant whitespace is emitted.
- Strings use the implementation's exact JSON escaping: quotation mark, reverse solidus, backspace, form feed, newline, carriage return, and tab use their short escapes; other control characters from U+0000 through U+001F use lowercase four-digit `\u` escapes; all other Unicode scalar values are emitted unchanged.
- Numbers use the `serde_json` representation. In particular, `1` and `1.0` are different values and produce different canonical JSON.
- Unicode is not normalized.

The SHA-256 digest is calculated over the canonical JSON UTF-8 bytes and encoded as lowercase hexadecimal.

## Fixed test vector

Input claim:

```json
{"subject":"player","predicate":"gave-gift","object":{"kind":"entity","id":"mira"},"qualifiers":{"item":"lantern"},"polarity":true,"about":null}
```

Canonical `claimKey` input:

```json
{"about":null,"object":{"id":"mira","kind":"entity"},"predicate":"gave-gift","qualifiers":{"item":"lantern"},"subject":"player"}
```

Expected `claimKey`:

```text
a0a8d5482c1eb719682e2cf8ef931adb41673a7d6ce1c509a625e93cfa1ed391
```

## Storage note

On the wire and in every public API, `claimKey` and `slotKey` are the 64-character
lowercase hex strings defined above. The SQLite store keeps
only the first 16 bytes of each digest (128 bits, a BLOB) in its record table and
indexes. Two different claims share a stored key only by a 128-bit SHA-256 prefix
collision, the same odds as two random UUIDs being equal. The conversion happens
only inside `umwelt-sqlite`. A string that is not 64 hex characters cannot match
any stored key, so a query with such a key returns no rows rather than an error.
