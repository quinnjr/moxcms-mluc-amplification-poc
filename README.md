# moxcms `mluc` record-aliasing memory amplification

Private reproduction for a security report to **awxkee/moxcms**. Shared read-only
with the maintainer for coordinated disclosure. Please keep it private until a fix
ships.

## Summary

`ColorProfile::new_from_slice_with_options` decodes every record of a
`multiLocalizedUnicodeType` (`mluc`) tag independently. Records are allowed to
share the same `(offset, length)`, so a tag with many records that all point at
one string costs **O(records × string length)** even though the on-disk tag is
small. A crafted profile under the 4 MiB `max_profile_size` cap drives memory to
hundreds of MB to GB; a larger cap reaches hundreds of GB. When the allocation
fails the process aborts, which `catch_unwind` cannot intercept.

Such a profile compresses well: a PNG `iCCP` chunk deflates it to a few KB, so a
small image file can take down any application that colour-manages untrusted
images with moxcms.

## Reproduction

```
cargo run --release -- <string_len_bytes> <record_count>
```

The program builds an RGB profile whose single `desc` tag is an `mluc` with
`record_count` records all pointing at one `string_len_bytes` string, parses it
with moxcms (`max_profile_size = 4 MiB`), and prints peak RSS (`VmHWM`) before and
after. Source is in `src/main.rs`.

## Measured on moxcms 0.9.1 (current release)

| On-disk profile | Records x string | Peak RSS after parse |
| --------------- | ---------------- | -------------------- |
| ~40 KB          | 2000 x 16 KiB    | ~18 MB               |
| ~268 KB         | 1000 x 256 KiB   | ~131 MB              |
| ~1.0 MB         | 1500 x 1 MiB     | ~775 MB              |

The last row is a ~1 MB input (well under the 4 MiB cap) expanding roughly 750x.
Growing the string toward the cap pushes peak RSS into the multi-GB range and then
to an allocation abort.

## Expected

Total decoded text should be bounded by the tag size (or a fixed cap), independent
of the record count. Any of these would bound it: decode each distinct
`(offset, length)` span once; cap the record count; reject an `mluc` whose summed
record lengths exceed the tag size.

## Context

Found while hardening PenguinView, an image viewer that uses moxcms for ICC to
sRGB conversion. Reported privately so a fix can ship before public disclosure.
