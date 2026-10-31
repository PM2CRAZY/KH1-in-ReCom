//! PC (1.5+2.5 ReMIX) archives: `<name>.hed` (index) + `<name>.pkg` (data), and [`Install`],
//! KH1's five archive pairs read as one.
//!
//! Layout and cipher are ported from OpenKH's `OpenKh.Egs` (`Hed.cs`, `EgsHdAsset.cs`,
//! `EgsEncryption.cs`), Apache-2.0, Copyright OpenKH Team and Contributors, by way of the KH2
//! rewrite's `kh2-formats` (same owner). Changes from the original: rewritten in Rust; the
//! repeated per-round XOR is folded into one 16-byte mask; the substitution table is located in
//! the user's executable at runtime instead of being embedded.
//!
//! ## HED
//! A flat array of 0x20-byte entries:
//! `md5(file name)[16] | pkg offset i64 | stored length i32 | real length i32`.
//!
//! ## PKG entry at `offset`
//! `header[16]` = `decompressed i32 | remastered count i32 | compressed i32 | date i32`,
//! then `remastered count` x 0x30-byte HD asset headers, then the original (PS2-format) file
//! data. `compressed` is -2 for plain data, -1 for encrypted-only, otherwise the zlib-compressed
//! size. Only the first 0x100 bytes of a stream are encrypted; the 16-byte header doubles as the
//! seed.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use md5::{Digest, Md5};
use sha2::Sha256;

use crate::reader::Bytes;
use crate::{bad, Error, Result};

/// SHA-256 of the 256-byte substitution table used by the PKG key schedule.
/// The table itself is not stored in this repository.
pub const KEY_TABLE_SHA256: &str = "759b2f22f7e0bfd257063a01f3de0c0835905217aa926df6def134a14313672d";

/// AES round constants, as used by the PKG key schedule (a public constant, not game data).
const RCON: [u8; 10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36];
const PASS_COUNT: usize = 10;

/// KH1's archive stems under `Image/dt/`, in the game's order.
pub const KH1_ARCHIVES: [&str; 5] = ["kh1_first", "kh1_second", "kh1_third", "kh1_fourth", "kh1_fifth"];
/// KH2 FM's archive stems under `Image/dt/` (same layout and cipher; read for the KH2 Keyblades,
/// decision 25). Checked (files: the owner's Steam install).
pub const KH2_ARCHIVES: [&str; 6] = ["kh2_first", "kh2_second", "kh2_third", "kh2_fourth", "kh2_fifth", "kh2_sixth"];

/// The KH1 executable's file name in the game folder.
pub const KH1_EXE: &str = "KINGDOM HEARTS FINAL MIX.exe";

/// The 256-byte substitution table needed to decrypt PKG entries.
#[derive(Clone)]
pub struct KeyTable([u8; 256]);

impl std::fmt::Debug for KeyTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KeyTable(<verified>)")
    }
}

/// Lower-case hex SHA-256 of `b`.
pub fn sha256_hex(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

impl KeyTable {
    /// Accepts a table only if its hash matches [`KEY_TABLE_SHA256`].
    pub fn from_bytes(b: &[u8]) -> Result<KeyTable> {
        if b.len() != 256 || sha256_hex(b) != KEY_TABLE_SHA256 {
            return bad("key table hash mismatch");
        }
        Ok(KeyTable(b.try_into().expect("length checked")))
    }

    /// Finds the table inside a game executable (any of the 1.5+2.5 exes that read PKG files
    /// carry it).
    pub fn from_executable(path: impl AsRef<Path>) -> Result<KeyTable> {
        let data = std::fs::read(path.as_ref())?;
        Self::find_in(&data).ok_or_else(|| {
            Error::Format(format!(
                "PKG key table not found in {} (unexpected executable version?)",
                path.as_ref().display()
            ))
        })
    }

    /// Finds the table in raw bytes. The table is a permutation of 0..=255, so a sliding window
    /// tracks distinct byte values and only hashes windows that are full permutations.
    pub fn find_in(data: &[u8]) -> Option<KeyTable> {
        if data.len() < 256 {
            return None;
        }
        let mut counts = [0u32; 256];
        let mut distinct = 0usize;
        for (i, &b) in data.iter().enumerate() {
            if counts[b as usize] == 0 {
                distinct += 1;
            }
            counts[b as usize] += 1;
            if i >= 256 {
                let out = data[i - 256] as usize;
                counts[out] -= 1;
                if counts[out] == 0 {
                    distinct -= 1;
                }
            }
            if i >= 255 && distinct == 256 {
                if let Ok(t) = KeyTable::from_bytes(&data[i - 255..=i]) {
                    return Some(t);
                }
            }
        }
        None
    }

    /// Expands a 16-byte seed into the XOR mask applied to each 16-byte block. The original
    /// applies 11 round keys in sequence; XOR is associative, so their XOR is equivalent.
    fn mask(&self, seed: &[u8; 16]) -> [u8; 16] {
        let mut k = [0u8; 16 * (PASS_COUNT + 1)];
        for i in 0..16 {
            k[i] = if seed[i] == 0 { i as u8 } else { seed[i] };
        }
        for i in 0..PASS_COUNT * 4 {
            let p = 0x0C + i * 4;
            let mut f = [k[p], k[p + 1], k[p + 2], k[p + 3]];
            if i % 4 == 0 {
                // The original indexes a byte array of little-endian u32 round constants, so only
                // the first byte is non-zero.
                let r = RCON.get(i / 4).copied().unwrap_or(0);
                f = [self.0[f[1] as usize] ^ r, self.0[f[2] as usize], self.0[f[3] as usize], self.0[f[0] as usize]];
            }
            for j in 0..4 {
                k[0x10 + i * 4 + j] = k[i * 4 + j] ^ f[j];
            }
        }
        let mut m = [0u8; 16];
        for round in 0..=PASS_COUNT {
            for j in 0..16 {
                m[j] ^= k[round * 16 + j];
            }
        }
        m
    }

    /// XORs the first 0x100 bytes of a stream with the mask of `seed` (the entry's 16-byte
    /// header). The same call decrypts and encrypts. Remastered assets use their entry's seed.
    pub fn decrypt_head(&self, seed: &[u8; 16], data: &mut [u8]) {
        let m = self.mask(seed);
        let end = data.len().min(0x100);
        for (i, b) in data[..end].iter_mut().enumerate() {
            *b ^= m[i % 16];
        }
    }
}

/// The 16-byte header of a PKG entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PkgHeader {
    /// Size of the original data once decompressed.
    pub decompressed: i32,
    /// Number of 0x30-byte remastered (HD) asset headers that follow.
    pub remastered: i32,
    /// -2 plain, -1 encrypted only, else the compressed size.
    pub compressed: i32,
    /// A date stamp (unused here).
    pub date: i32,
}

impl PkgHeader {
    /// Parses the first 16 bytes of an entry.
    pub fn parse(seed: &[u8; 16]) -> Result<PkgHeader> {
        let b = Bytes(seed);
        let h = PkgHeader { decompressed: b.i32(0)?, remastered: b.i32(4)?, compressed: b.i32(8)?, date: b.i32(12)? };
        if h.decompressed < 0 || !(0..=0x10000).contains(&h.remastered) || h.compressed < -2 {
            return bad("implausible pkg entry header");
        }
        Ok(h)
    }

    /// Bytes from the entry start to the end of the original data.
    pub fn original_end(&self) -> usize {
        16 + self.remastered as usize * 0x30 + self.stored_data_len()
    }

    fn stored_data_len(&self) -> usize {
        (if self.compressed >= 0 { self.compressed } else { self.decompressed }) as usize
    }
}

/// Decodes the original-format file from a raw PKG entry (the bytes at its hed offset; at least
/// up to [`PkgHeader::original_end`]): decrypts the first 0x100 bytes and inflates.
pub fn decode_entry(keys: &KeyTable, raw: &[u8]) -> Result<Vec<u8>> {
    let seed: [u8; 16] = Bytes(raw).arr(0)?;
    let h = PkgHeader::parse(&seed)?;
    let start = 16 + h.remastered as usize * 0x30;
    let mut data = Bytes(raw).slice(start, h.stored_data_len())?.to_vec();
    if h.compressed > -2 {
        keys.decrypt_head(&seed, &mut data);
    }
    if h.compressed < 0 {
        return Ok(data);
    }
    if data.len() < 2 {
        return bad("compressed data too short");
    }
    let mut out = Vec::with_capacity(h.decompressed as usize);
    flate2::read::DeflateDecoder::new(&data[2..]).read_to_end(&mut out)?;
    if out.len() != h.decompressed as usize {
        return bad(format!("decompressed {} bytes, expected {}", out.len(), h.decompressed));
    }
    Ok(out)
}

/// One remastered (HD) asset of a PKG entry: `.dds` textures, `.cvbl` meshes, `.scd` sounds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HdAsset {
    /// The asset's name, e.g. `-c0.cvbl` or `-1c0.dds` (at most 31 bytes).
    pub name: String,
    /// The tag the game matches it by: an offset in the original file, or -1.
    pub orig_offset: i32,
    /// The asset's bytes, decoded.
    pub data: Vec<u8>,
}

/// Size of one remastered asset header in a PKG entry.
pub const HD_HEADER_SIZE: usize = 0x30;

fn align16(n: usize) -> usize {
    (n + 15) & !15
}

/// Decodes an entry's original file and every remastered asset from the whole raw PKG entry
/// (the bytes at its hed offset, `stored_len` long).
///
/// Layout (OpenKH `EgsHdAsset.cs`; Checked (files): all 500 KH1 `.mdls` entries): the 16-byte
/// header, `remastered` headers of 0x30 bytes (`name[0x20] | offset | orig_offset |
/// decompressed | compressed`, not encrypted), the original data, then each asset's data, each
/// padded to 16 bytes. Assets use the entry header as their seed.
pub fn decode_entry_full(keys: &KeyTable, raw: &[u8]) -> Result<(Vec<u8>, Vec<HdAsset>)> {
    let data = decode_entry(keys, raw)?;
    let seed: [u8; 16] = Bytes(raw).arr(0)?;
    let h = PkgHeader::parse(&seed)?;
    let mut pos = 16 + h.remastered as usize * HD_HEADER_SIZE + align16(h.stored_data_len());
    let mut assets = Vec::with_capacity(h.remastered as usize);
    for i in 0..h.remastered as usize {
        let at = 16 + i * HD_HEADER_SIZE;
        let b = Bytes(raw);
        let name = b.cstr(at, 0x20)?;
        let orig_offset = b.i32(at + 0x24)?;
        let dec = b.i32(at + 0x28)?;
        let comp = b.i32(at + 0x2C)?;
        if dec < 0 || comp < -2 {
            return bad(format!("hd asset {name}: implausible sizes {dec} / {comp}"));
        }
        let stored = if comp >= 0 { comp } else { dec } as usize;
        let mut d = b.slice(pos, stored)?.to_vec();
        if comp > -2 {
            keys.decrypt_head(&seed, &mut d);
        }
        if comp >= 0 {
            if d.len() < 2 {
                return bad(format!("hd asset {name}: compressed data too short"));
            }
            let mut out = Vec::with_capacity(dec as usize);
            flate2::read::DeflateDecoder::new(&d[2..]).read_to_end(&mut out)?;
            // Five Halloween Town models' `-1c2.dds` inflate to 12 bytes more than their
            // header says (Checked (files)); the size the header gives is what the game asks
            // the decompressor for, so the rest is dropped.
            if out.len() > dec as usize {
                out.truncate(dec as usize);
            }
            d = out;
        }
        if d.len() != dec as usize {
            return bad(format!("hd asset {name}: {} bytes, expected {dec}", d.len()));
        }
        assets.push(HdAsset { name, orig_offset, data: d });
        pos += align16(stored);
    }
    Ok((data, assets))
}

/// Builds a plain (not compressed, not encrypted) PKG entry: what Panacea serves from
/// `<mod_path>/kh1/raw/<file>` in place of the archive's entry, HD assets included.
///
/// Layout as [`decode_entry_full`] reads it, with every size field `-2` (plain). Each asset's
/// offset field is its position in the entry (Panacea's `LoadFile` recomputes the same values
/// for a raw file). `date` is the header's date word.
pub fn build_plain_entry(data: &[u8], assets: &[HdAsset], date: i32) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let n = assets.len();
    for v in [data.len() as i32, n as i32, -2, date] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    let mut pos = 16 + n * HD_HEADER_SIZE + align16(data.len());
    for a in assets {
        if a.name.len() > 0x1F || a.name.contains('\0') {
            return bad(format!("hd asset name {:?} does not fit", a.name));
        }
        let mut name = [0u8; 0x20];
        name[..a.name.len()].copy_from_slice(a.name.as_bytes());
        out.extend_from_slice(&name);
        for v in [pos as i32, a.orig_offset, a.data.len() as i32, -2] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        pos += align16(a.data.len());
    }
    out.extend_from_slice(data);
    out.resize(align16(out.len()), 0);
    for a in assets {
        out.extend_from_slice(&a.data);
        out.resize(align16(out.len()), 0);
    }
    Ok(out)
}

/// One HED entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HedEntry {
    /// MD5 of the file name.
    pub md5: [u8; 16],
    /// Offset of the entry in the `.pkg`.
    pub offset: u64,
    /// Bytes the entry takes in the `.pkg` (header, HD assets and data).
    pub stored_len: u32,
    /// Size of the original file.
    pub real_len: u32,
}

/// MD5 of a file name, the HED key (e.g. `name_hash("btltbl.bin")`).
pub fn name_hash(name: &str) -> [u8; 16] {
    Md5::digest(name.as_bytes()).into()
}

/// One `.hed`/`.pkg` pair.
#[derive(Debug)]
pub struct Archive {
    /// The `.hed` path.
    pub hed_path: PathBuf,
    /// The `.pkg` path (only opened when a file is read).
    pub pkg_path: PathBuf,
    /// Every readable entry.
    pub entries: Vec<HedEntry>,
}

impl Archive {
    /// Opens `<stem>.hed` and remembers `<stem>.pkg`. Accepts either path.
    pub fn open(path: impl AsRef<Path>) -> Result<Archive> {
        let p = path.as_ref();
        let hed_path = p.with_extension("hed");
        let pkg_path = p.with_extension("pkg");
        let hed = std::fs::read(&hed_path)?;
        Ok(Archive { entries: Self::parse_hed(&hed).map_err(|e| Error::Format(format!("{}: {e}", hed_path.display())))?, hed_path, pkg_path })
    }

    /// Parses a `.hed`'s bytes.
    pub fn parse_hed(hed: &[u8]) -> Result<Vec<HedEntry>> {
        if !hed.len().is_multiple_of(0x20) {
            return bad("hed size is not a multiple of 0x20");
        }
        let b = Bytes(hed);
        let mut entries = Vec::with_capacity(hed.len() / 0x20);
        for at in (0..hed.len()).step_by(0x20) {
            let offset = b.i64(at + 16)?;
            let stored_len = b.i32(at + 24)?;
            let real_len = b.i32(at + 28)?;
            // A junk entry (KH1's own `kh1_fifth.hed` has one, a length of 0xFDFDFDFD) names no
            // readable file: left out rather than refusing the whole archive.
            if offset < 0 || stored_len < 0 || real_len < 0 {
                continue;
            }
            entries.push(HedEntry { md5: b.arr(at)?, offset: offset as u64, stored_len: stored_len as u32, real_len: real_len as u32 });
        }
        Ok(entries)
    }

    /// The entry for a file name, if this archive has it.
    pub fn find(&self, name: &str) -> Option<&HedEntry> {
        let h = name_hash(name);
        self.entries.iter().find(|e| e.md5 == h)
    }

    /// Reads, decrypts and decompresses the original-format data of `name`.
    pub fn read(&self, keys: &KeyTable, name: &str) -> Result<Vec<u8>> {
        let e = *self.find(name).ok_or_else(|| Error::NotFound(name.to_string()))?;
        self.read_entry(keys, &e).map_err(|err| match err {
            Error::Format(s) => Error::Format(format!("{name}: {s}")),
            other => other,
        })
    }

    /// Reads one entry's whole raw bytes (header, HD headers, data and HD assets), as stored.
    pub fn read_raw_entry(&self, e: &HedEntry) -> Result<Vec<u8>> {
        let mut f = File::open(&self.pkg_path)?;
        f.seek(SeekFrom::Start(e.offset))?;
        let mut raw = vec![0u8; e.stored_len as usize];
        f.read_exact(&mut raw)?;
        Ok(raw)
    }

    /// Reads one entry's original-format data.
    pub fn read_entry(&self, keys: &KeyTable, e: &HedEntry) -> Result<Vec<u8>> {
        let mut f = File::open(&self.pkg_path)?;
        f.seek(SeekFrom::Start(e.offset))?;
        let mut seed = [0u8; 16];
        f.read_exact(&mut seed)?;
        let h = PkgHeader::parse(&seed)?;
        let need = h.original_end();
        if need as u64 > e.stored_len as u64 {
            return bad("data length exceeds hed entry");
        }
        let mut raw = vec![0u8; need];
        raw[..16].copy_from_slice(&seed);
        f.read_exact(&mut raw[16..])?;
        decode_entry(keys, &raw)
    }
}

/// A KH1 FM install: the five `Image/dt/kh1_*.hed`/`.pkg` pairs read as one.
#[derive(Debug)]
pub struct Install {
    /// The game folder.
    pub root: PathBuf,
    /// The archives found, in [`KH1_ARCHIVES`] order.
    pub archives: Vec<Archive>,
    /// The PKG key table.
    pub keys: KeyTable,
}

impl Install {
    /// Opens the game folder (it holds `Image/dt/kh1_first.hed` … and [`KH1_EXE`]), taking the
    /// key table from the KH1 executable.
    pub fn open(root: impl AsRef<Path>) -> Result<Install> {
        let keys = KeyTable::from_executable(root.as_ref().join(KH1_EXE))?;
        Self::with_keys(root, keys)
    }

    /// Opens the game folder with a key table found elsewhere (any 1.5+2.5 exe carries it).
    /// Archives other than `kh1_first` that are missing are skipped.
    pub fn with_keys(root: impl AsRef<Path>, keys: KeyTable) -> Result<Install> {
        Self::with_archives(root, keys, &KH1_ARCHIVES)
    }

    /// Opens the archive pairs `stems` (e.g. [`KH2_ARCHIVES`]) under `Image/dt/` as one install.
    /// The first must be there; later ones that are missing are skipped.
    pub fn with_archives(root: impl AsRef<Path>, keys: KeyTable, stems: &[&str]) -> Result<Install> {
        let root = root.as_ref().to_path_buf();
        let dt = root.join("Image").join("dt");
        let mut archives = Vec::new();
        for (i, stem) in stems.iter().enumerate() {
            let hed = dt.join(format!("{stem}.hed"));
            if !hed.is_file() {
                if i == 0 {
                    return bad(format!("{} not found (not a KH 1.5+2.5 PC install?)", hed.display()));
                }
                continue;
            }
            archives.push(Archive::open(&hed)?);
        }
        Ok(Install { root, archives, keys })
    }

    /// The first archive (in [`KH1_ARCHIVES`] order) holding `name`, with its entry.
    pub fn find(&self, name: &str) -> Option<(&Archive, &HedEntry)> {
        self.archives.iter().find_map(|a| a.find(name).map(|e| (a, e)))
    }

    /// Reads `name` from whichever archive holds it.
    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        let (a, _) = self.find(name).ok_or_else(|| Error::NotFound(name.to_string()))?;
        a.read(&self.keys, name)
    }

    /// Reads `name` with its HD assets, and the entry header's date word.
    pub fn read_full(&self, name: &str) -> Result<(Vec<u8>, Vec<HdAsset>, i32)> {
        let (a, e) = self.find(name).ok_or_else(|| Error::NotFound(name.to_string()))?;
        let raw = a.read_raw_entry(e)?;
        let (data, assets) = decode_entry_full(&self.keys, &raw).map_err(|err| match err {
            Error::Format(s) => Error::Format(format!("{name}: {s}")),
            other => other,
        })?;
        Ok((data, assets, Bytes(&raw).i32(12)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata;

    #[test]
    fn name_hash_is_md5() {
        // RFC 1321 test vector.
        assert_eq!(
            name_hash("abc"),
            [0x90, 0x01, 0x50, 0x98, 0x3c, 0xd2, 0x4f, 0xb0, 0xd6, 0x96, 0x3f, 0x7d, 0x28, 0xe1, 0x7f, 0x72]
        );
    }

    /// Straight transcription of the reference algorithm (11 sequential XOR passes per block,
    /// 4 bytes at a time) to check the folded mask.
    fn reference_decrypt(table: &[u8; 256], seed: &[u8; 16], data: &mut [u8]) {
        let mut sk = [0u8; 40];
        for (i, r) in RCON.iter().enumerate() {
            sk[i * 4] = *r;
        }
        let mut k = [0u8; 0xB0];
        for i in 0..16 {
            k[i] = if seed[i] == 0 { i as u8 } else { seed[i] };
        }
        for i in 0..40 {
            let mut f = [k[0x0C + i * 4], k[0x0D + i * 4], k[0x0E + i * 4], k[0x0F + i * 4]];
            if i % 4 == 0 {
                f = [
                    table[f[1] as usize] ^ sk[i],
                    table[f[2] as usize] ^ sk[i + 1],
                    table[f[3] as usize] ^ sk[i + 2],
                    table[f[0] as usize] ^ sk[i + 3],
                ];
            }
            for j in 0..4 {
                k[0x10 + i * 4 + j] = k[i * 4 + j] ^ f[j];
            }
        }
        let mut idx = 0;
        while idx < data.len().min(0x100) {
            let n = (data.len() - idx).min(16);
            for pass in (0..=10).rev() {
                for j in 0..n {
                    data[idx + j] ^= k[pass * 16 + j];
                }
            }
            idx += 16;
        }
    }

    #[test]
    fn folded_mask_matches_reference() {
        let mut t = [0u8; 256];
        for (i, b) in t.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(167).wrapping_add(13);
        }
        let kt = KeyTable(t);
        for s in 0..4u8 {
            let seed: [u8; 16] = core::array::from_fn(|i| if i % 3 == 0 { 0 } else { (i as u8).wrapping_mul(31).wrapping_add(s) });
            let original: Vec<u8> = (0..300u32).map(|x| (x * 7 + s as u32) as u8).collect();
            let mut a = original.clone();
            let mut b = original.clone();
            kt.decrypt_head(&seed, &mut a);
            reference_decrypt(&t, &seed, &mut b);
            assert_eq!(a, b);
            assert_eq!(&a[0x100..], &original[0x100..]);
        }
    }

    #[test]
    fn rejects_wrong_table() {
        let t: [u8; 256] = core::array::from_fn(|i| i as u8);
        assert!(KeyTable::from_bytes(&t).is_err());
        assert!(KeyTable::find_in(&t).is_none());
    }

    #[test]
    fn plain_entry_round_trips() {
        let t: [u8; 256] = core::array::from_fn(|i| i as u8);
        let kt = KeyTable(t);
        let mut raw = Vec::new();
        for v in [5i32, 0, -2, 0] {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        raw.extend_from_slice(b"hello");
        assert_eq!(decode_entry(&kt, &raw).unwrap(), b"hello");
        assert!(decode_entry(&kt, &raw[..18]).is_err());
    }

    /// A plain entry with HD assets decodes back to the same file and assets, with Panacea's
    /// offsets (16-byte header, 0x30 per asset header, data and each asset padded to 16).
    #[test]
    fn plain_entry_with_assets_round_trips() {
        let kt = KeyTable(core::array::from_fn(|i| i as u8));
        let data: Vec<u8> = (0..37u8).collect();
        let assets = vec![
            HdAsset { name: "-c0.cvbl".into(), orig_offset: -1, data: vec![1; 20] },
            HdAsset { name: "-1c0.dds".into(), orig_offset: 0x1C0, data: vec![2; 16] },
        ];
        let e = build_plain_entry(&data, &assets, 7).unwrap();
        let (d, a) = decode_entry_full(&kt, &e).unwrap();
        assert_eq!(d, data);
        assert_eq!(a, assets);
        let first = 16 + 2 * HD_HEADER_SIZE + 48;
        assert_eq!(Bytes(&e).i32(16 + 0x20).unwrap(), first as i32);
        assert_eq!(Bytes(&e).i32(16 + HD_HEADER_SIZE + 0x20).unwrap(), (first + 32) as i32);
        assert_eq!(e.len(), first + 32 + 16);
        assert!(build_plain_entry(&data, &[HdAsset { name: "x".repeat(32), orig_offset: 0, data: vec![] }], 0).is_err());
    }

    /// The three files and their SHA-256 (of the user's own copies).
    const FILES: [(&str, &str); 3] = [
        ("btltbl.bin", "b70db229ce1a0ae26a9353f88f75818145f3c947d0340808a2d06528e60daf0e"),
        ("xa_ex_0010.mset", "60d43ee9b2e2744fd4c2c62cdf98b1e412aa28e48f2e6c0201bcb04a932dbb7b"),
        ("xa_ex_0010.mdls", "697fd74bf2dec0a5b0410436e88b1b3131cfa7c122f1f180fd5f05af161bf4cf"),
    ];

    fn keys() -> Option<KeyTable> {
        let p = testdata::dir()?.join("key.exe");
        if !p.is_file() {
            eprintln!("skipping: {} not found", p.display());
            return None;
        }
        Some(KeyTable::from_executable(p).expect("key table in key.exe"))
    }

    /// Raw PKG entries (`pkg/<name>.pkgent`) decode to the extracted files.
    #[test]
    fn pkg_entries_decode() {
        let Some(keys) = keys() else { return };
        for (name, sha) in FILES {
            let Some(raw) = testdata::read(&format!("pkg/{name}.pkgent")) else { continue };
            let out = decode_entry(&keys, &raw).unwrap();
            assert!(sha256_hex(&out).starts_with(sha), "{name}");
            if let Some(plain) = testdata::read(name) {
                assert_eq!(out, plain, "{name}");
            }
        }
    }

    /// `hed/kh1_*.hed`: the names hash to entries in kh1_first, with the extracted sizes; an
    /// `Install` built in a temp folder (sparse pkg holding the cut entries) reads them back.
    #[test]
    fn install_reads_by_name() {
        let Some(dir) = testdata::dir() else { return };
        let Some(keys) = keys() else { return };
        let hed_dir = dir.join("hed");
        if !hed_dir.join("kh1_first.hed").is_file() {
            eprintln!("skipping: no hed/kh1_first.hed");
            return;
        }
        let first = Archive::open(hed_dir.join("kh1_first.hed")).unwrap();
        assert_eq!(first.entries.len(), 601);
        for (name, _) in FILES {
            let e = first.find(name).unwrap_or_else(|| panic!("{name} in kh1_first"));
            if let Some(plain) = testdata::read(name) {
                assert_eq!(e.real_len as usize, plain.len(), "{name}");
            }
        }
        assert_eq!(first.find("btltbl.bin").unwrap().stored_len, 131584);

        // A temp game folder: every hed, and a sparse kh1_first.pkg with the cut entries at
        // their offsets.
        let root = std::env::temp_dir().join(format!("kh1-formats-install-{}", std::process::id()));
        let dt = root.join("Image").join("dt");
        std::fs::create_dir_all(&dt).unwrap();
        for stem in KH1_ARCHIVES {
            let src = hed_dir.join(format!("{stem}.hed"));
            if src.is_file() {
                std::fs::copy(&src, dt.join(format!("{stem}.hed"))).unwrap();
            }
        }
        {
            use std::io::Write;
            let mut pkg = File::create(dt.join("kh1_first.pkg")).unwrap();
            for (name, _) in FILES {
                let Some(raw) = testdata::read(&format!("pkg/{name}.pkgent")) else { continue };
                let e = first.find(name).unwrap();
                assert_eq!(raw.len() as u32, e.stored_len, "{name}");
                pkg.seek(SeekFrom::Start(e.offset)).unwrap();
                pkg.write_all(&raw).unwrap();
            }
        }
        let inst = Install::with_keys(&root, keys).unwrap();
        assert_eq!(inst.archives.len(), 5);
        // KH1's own fifth.hed carries one junk entry (skipped).
        assert!(inst.archives.iter().all(|a| !a.entries.is_empty()));
        for (name, sha) in FILES {
            let (a, _) = inst.find(name).unwrap();
            assert!(a.hed_path.ends_with("kh1_first.hed"), "{name}");
            if testdata::read(&format!("pkg/{name}.pkgent")).is_some() {
                assert!(sha256_hex(&inst.read(name).unwrap()).starts_with(sha), "{name}");
            }
        }
        assert!(matches!(inst.read("no such file.bin"), Err(Error::NotFound(_))));
        let _ = std::fs::remove_dir_all(&root);
    }
}
