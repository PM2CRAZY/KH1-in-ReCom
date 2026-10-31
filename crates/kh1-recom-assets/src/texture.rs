//! Textures for the Keyblade models: DDS decoding (DXT1 / DXT3 / DXT5 and uncompressed 32-bit,
//! the forms seen in KH1's and Re:CoM's HD assets), a box downscale, and a deterministic
//! 256-colour quantiser (median cut) for the 8-bit TIM2 Re:CoM keeps as the non-HD texture.
//!
//! KH1 PC's Keyblade textures are DXT5 without mipmaps (Checked (files: all 35 cvbl textures of
//! the 17 Keyblades)); Re:CoM's own HD textures are A8R8G8B8 (PL0002: 1024×512). Re:CoM's DDS
//! loader (`0x1402D9760`) takes A8R8G8B8, DXT5/4, DXT3/2, DXT1 and 16-bit forms. Checked (decomp).

use crate::{bad, Result};

/// An RGBA8 image, rows top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    /// Width.
    pub w: usize,
    /// Height.
    pub h: usize,
    /// Pixels, `w * h`.
    pub px: Vec<[u8; 4]>,
}

impl Image {
    /// The pixel at `(x, y)`, clamped to the edges.
    pub fn at(&self, x: isize, y: isize) -> [u8; 4] {
        let x = x.clamp(0, self.w as isize - 1) as usize;
        let y = y.clamp(0, self.h as isize - 1) as usize;
        self.px[y * self.w + x]
    }
}

fn u32le(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

fn rgb565(c: u16) -> [u32; 3] {
    let r = (c >> 11) as u32 & 31;
    let g = (c >> 5) as u32 & 63;
    let b = c as u32 & 31;
    [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)]
}

/// The four colours of a BC1 colour block (`four` forces the 4-colour mode, as in DXT3/5).
fn colour_block(b: &[u8], four: bool) -> ([[u8; 3]; 4], [bool; 4]) {
    let c0 = u16::from_le_bytes([b[0], b[1]]);
    let c1 = u16::from_le_bytes([b[2], b[3]]);
    let (a, z) = (rgb565(c0), rgb565(c1));
    let mix = |p: u32, q: u32, wp: u32, wq: u32, d: u32| -> [u8; 3] { core::array::from_fn(|k| ((a[k] * p * wp + z[k] * q * wq) / d) as u8) };
    let ca = [a[0] as u8, a[1] as u8, a[2] as u8];
    let cz = [z[0] as u8, z[1] as u8, z[2] as u8];
    if four || c0 > c1 {
        ([ca, cz, mix(1, 1, 2, 1, 3), mix(1, 1, 1, 2, 3)], [true; 4])
    } else {
        ([ca, cz, mix(1, 1, 1, 1, 2), [0, 0, 0]], [true, true, true, false])
    }
}

/// Decodes a DDS (no DX10 header; the top mip level).
pub fn decode_dds(d: &[u8]) -> Result<Image> {
    if d.len() < 128 || &d[0..4] != b"DDS " || u32le(d, 4) != 124 {
        return bad("dds: no DDS header");
    }
    let (h, w) = (u32le(d, 12) as usize, u32le(d, 16) as usize);
    if w == 0 || h == 0 || w > 16384 || h > 16384 {
        return bad("dds: implausible size");
    }
    let pf_flags = u32le(d, 80);
    let fourcc = &d[84..88];
    let body = &d[128..];
    let mut px = vec![[0u8; 4]; w * h];
    if pf_flags & 4 != 0 {
        let kind = match fourcc {
            b"DXT1" => 1,
            b"DXT2" | b"DXT3" => 3,
            b"DXT4" | b"DXT5" => 5,
            f => return bad(format!("dds: fourcc {:?} not supported", String::from_utf8_lossy(f))),
        };
        let bs = if kind == 1 { 8 } else { 16 };
        let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
        if body.len() < bw * bh * bs {
            return bad("dds: data too short");
        }
        for by in 0..bh {
            for bx in 0..bw {
                let blk = &body[(by * bw + bx) * bs..][..bs];
                let (cb, alpha): (&[u8], [u8; 16]) = match kind {
                    1 => (blk, [255; 16]),
                    3 => (&blk[8..], core::array::from_fn(|i| ((blk[i / 2] >> (4 * (i & 1))) & 15) * 17)),
                    _ => {
                        let (a0, a1) = (blk[0] as u32, blk[1] as u32);
                        let mut pal = [0u32; 8];
                        pal[0] = a0;
                        pal[1] = a1;
                        if a0 > a1 {
                            for k in 1..7 {
                                pal[k + 1] = ((7 - k as u32) * a0 + k as u32 * a1) / 7;
                            }
                        } else {
                            for k in 1..5 {
                                pal[k + 1] = ((5 - k as u32) * a0 + k as u32 * a1) / 5;
                            }
                            pal[6] = 0;
                            pal[7] = 255;
                        }
                        let bits = blk[2..8].iter().enumerate().fold(0u64, |acc, (i, &x)| acc | (x as u64) << (8 * i));
                        (&blk[8..], core::array::from_fn(|i| pal[((bits >> (3 * i)) & 7) as usize] as u8))
                    }
                };
                let (cols, opaque) = colour_block(cb, kind != 1);
                let idx = u32le(cb, 4);
                for (i, &al) in alpha.iter().enumerate() {
                    let (x, y) = (bx * 4 + (i & 3), by * 4 + i / 4);
                    if x >= w || y >= h {
                        continue;
                    }
                    let k = ((idx >> (2 * i)) & 3) as usize;
                    let c = cols[k];
                    let a = if opaque[k] { al } else { 0 };
                    px[y * w + x] = [c[0], c[1], c[2], a];
                }
            }
        }
    } else if pf_flags & 0x40 != 0 && u32le(d, 88) == 32 {
        let masks = [u32le(d, 92), u32le(d, 96), u32le(d, 100), if pf_flags & 1 != 0 { u32le(d, 104) } else { 0 }];
        if body.len() < 4 * w * h {
            return bad("dds: data too short");
        }
        let chan = |v: u32, m: u32| -> u8 {
            if m == 0 {
                return 255;
            }
            ((v & m) >> m.trailing_zeros()) as u8
        };
        for (i, p) in px.iter_mut().enumerate() {
            let v = u32le(body, 4 * i);
            *p = [chan(v, masks[0]), chan(v, masks[1]), chan(v, masks[2]), chan(v, masks[3])];
        }
    } else {
        return bad("dds: pixel format not supported");
    }
    Ok(Image { w, h, px })
}

/// Halves the image (2×2 box average, straight alpha) until both sides are at most `max`.
pub fn downscale_to(img: &Image, max: usize) -> Image {
    let mut cur = img.clone();
    while cur.w > max || cur.h > max {
        let (w, h) = ((cur.w / 2).max(1), (cur.h / 2).max(1));
        let mut px = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let mut s = [0u32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let p = cur.at((2 * x + dx) as isize, (2 * y + dy) as isize);
                    for k in 0..4 {
                        s[k] += p[k] as u32;
                    }
                }
                px.push(core::array::from_fn(|k| ((s[k] + 2) / 4) as u8));
            }
        }
        cur = Image { w, h, px };
    }
    cur
}

/// 8-bit alpha (255 opaque) to the PS2's (0x80 opaque).
pub fn ps2_alpha(a: u8) -> u8 {
    ((a as u32 * 128 + 127) / 255) as u8
}

/// Quantises to 256 colours (median cut over RGBA, count-weighted, deterministic), then maps
/// every pixel to its nearest palette entry. Returns the palette and one index per pixel.
pub fn quantise(img: &Image) -> ([[u8; 4]; 256], Vec<u8>) {
    // Distinct colours with counts, in a fixed order.
    let mut cols: Vec<([u8; 4], u32)> = {
        let mut v: Vec<[u8; 4]> = img.px.clone();
        v.sort_unstable();
        let mut out: Vec<([u8; 4], u32)> = Vec::new();
        for c in v {
            match out.last_mut() {
                Some((p, n)) if *p == c => *n += 1,
                _ => out.push((c, 1)),
            }
        }
        out
    };
    let mut boxes: Vec<std::ops::Range<usize>> = std::iter::once(0..cols.len()).collect();
    let range = |cols: &[([u8; 4], u32)], k: usize| -> u32 {
        let (mn, mx) = cols.iter().fold((255u8, 0u8), |(a, b), (c, _)| (a.min(c[k]), b.max(c[k])));
        mx.saturating_sub(mn) as u32
    };
    while boxes.len() < 256 {
        // The box with the widest channel (ties: the lower box index, then the lower channel).
        let mut best: Option<(u32, usize, usize)> = None;
        for (bi, r) in boxes.iter().enumerate() {
            if r.len() < 2 {
                continue;
            }
            for k in 0..4 {
                let w = range(&cols[r.clone()], k);
                if w > 0 && best.is_none_or(|b| w > b.0) {
                    best = Some((w, bi, k));
                }
            }
        }
        let Some((_, bi, k)) = best else { break };
        let r = boxes[bi].clone();
        let slice = &mut cols[r.clone()];
        slice.sort_by(|a, b| a.0[k].cmp(&b.0[k]).then(a.0.cmp(&b.0)));
        let total: u64 = slice.iter().map(|c| c.1 as u64).sum();
        let mut acc = 0u64;
        let mut cut = 1;
        for (i, c) in slice.iter().enumerate() {
            acc += c.1 as u64;
            if 2 * acc >= total {
                cut = (i + 1).clamp(1, slice.len() - 1);
                break;
            }
        }
        boxes[bi] = r.start..r.start + cut;
        boxes.push(r.start + cut..r.end);
    }
    let mut pal = [[0u8; 4]; 256];
    for (i, r) in boxes.iter().enumerate() {
        let mut s = [0u64; 4];
        let mut n = 0u64;
        for (c, m) in &cols[r.clone()] {
            for k in 0..4 {
                s[k] += c[k] as u64 * *m as u64;
            }
            n += *m as u64;
        }
        if n > 0 {
            pal[i] = core::array::from_fn(|k| ((s[k] + n / 2) / n) as u8);
        }
    }
    let used = boxes.len();
    let near = |c: [u8; 4]| -> u8 {
        let mut best = (u64::MAX, 0usize);
        for (i, p) in pal[..used].iter().enumerate() {
            let d: u64 = (0..4).map(|k| {
                let e = c[k] as i64 - p[k] as i64;
                (e * e) as u64 * if k == 3 { 2 } else { 1 }
            }).sum();
            if d < best.0 {
                best = (d, i);
            }
        }
        best.1 as u8
    };
    // Map each distinct colour once.
    cols.sort_unstable_by_key(|c| c.0);
    let map: Vec<([u8; 4], u8)> = cols.iter().map(|(c, _)| (*c, near(*c))).collect();
    let idx = img.px.iter().map(|c| map[map.binary_search_by(|m| m.0.cmp(c)).unwrap()].1).collect();
    (pal, idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dds_header(w: u32, h: u32, fourcc: Option<&[u8; 4]>) -> Vec<u8> {
        let mut d = vec![0u8; 128];
        d[0..4].copy_from_slice(b"DDS ");
        d[4..8].copy_from_slice(&124u32.to_le_bytes());
        d[12..16].copy_from_slice(&h.to_le_bytes());
        d[16..20].copy_from_slice(&w.to_le_bytes());
        match fourcc {
            Some(f) => {
                d[80..84].copy_from_slice(&4u32.to_le_bytes());
                d[84..88].copy_from_slice(f);
            }
            None => {
                d[80..84].copy_from_slice(&0x41u32.to_le_bytes());
                d[88..92].copy_from_slice(&32u32.to_le_bytes());
                for (k, m) in [0xFF0000u32, 0xFF00, 0xFF, 0xFF00_0000].iter().enumerate() {
                    d[92 + 4 * k..96 + 4 * k].copy_from_slice(&m.to_le_bytes());
                }
            }
        }
        d
    }

    #[test]
    fn dxt5_block() {
        let mut d = dds_header(4, 4, Some(b"DXT5"));
        // Alpha 255 / 0, indices: pixel 0 → a0, pixel 1 → a1, rest a0.
        d.extend_from_slice(&[255, 0, 0b0000_1000, 0, 0, 0, 0, 0]);
        // Colours: c0 = white, c1 = black; pixel 0 → c0, pixel 1 → c1, pixel 2 → 2/3 c0.
        d.extend_from_slice(&[0xFF, 0xFF, 0, 0, 0b0010_0100, 0, 0, 0]);
        let img = decode_dds(&d).unwrap();
        assert_eq!(img.px[0], [255, 255, 255, 255]);
        assert_eq!(img.px[1], [0, 0, 0, 0]);
        assert_eq!(img.px[2], [170, 170, 170, 255]);
    }

    #[test]
    fn argb_and_downscale_and_quantise() {
        let mut d = dds_header(4, 2, None);
        for i in 0..8u32 {
            d.extend_from_slice(&(0xFF00_0000 | (i * 30) << 16 | 5 << 8 | 7).to_le_bytes());
        }
        let img = decode_dds(&d).unwrap();
        assert_eq!(img.px[1], [30, 5, 7, 255]);
        let s = downscale_to(&img, 2);
        assert_eq!((s.w, s.h), (2, 1));
        assert_eq!(s.px[0], [((30 + 120 + 150 + 2) / 4) as u8, 5, 7, 255]);
        let (pal, idx) = quantise(&img);
        // Fewer than 256 colours: exact.
        assert!(idx.iter().zip(&img.px).all(|(&i, c)| pal[i as usize] == *c));
        assert_eq!(ps2_alpha(255), 128);
        assert_eq!(ps2_alpha(0), 0);
    }

    #[test]
    fn quantise_many_colours_is_close_and_deterministic() {
        let px: Vec<[u8; 4]> = (0..64 * 64u32).map(|i| [(i % 64 * 4) as u8, (i / 64 * 4) as u8, ((i * 7) % 256) as u8, 128]).collect();
        let img = Image { w: 64, h: 64, px };
        let (pal, idx) = quantise(&img);
        let (pal2, idx2) = quantise(&img);
        assert_eq!((pal, &idx), (pal2, &idx2));
        let err: f64 = idx.iter().zip(&img.px).map(|(&i, c)| (0..4).map(|k| (c[k] as f64 - pal[i as usize][k] as f64).abs()).sum::<f64>()).sum::<f64>() / img.px.len() as f64;
        assert!(err < 60.0, "{err}");
    }
}
