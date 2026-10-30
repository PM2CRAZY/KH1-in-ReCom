//! "KH1 motions test": re-points rows of Sora's action table (`motions::SWAPS`) to the KH1 clips
//! in the extended `PL0001.ANN`. Re:CoM's `stateInitial` (`0x1401FA6E0`) resolves the names on
//! the next player set-up (room load) and its motion request plays them.
//!
//! Only the name pointers change. The index field is left alone: it is -1 in the exe's data until
//! the first set-up, and every set-up rewrites all of them from the names, so a reset is not
//! needed, and live it would make a playing Sora request slot -1.

use super::{hook, log, va};
use crate::motions::{self, RowPatch};
use std::path::Path;
use std::sync::Mutex;

struct Row {
    addr: usize,
    recom: u64,
    kh1: u64,
    what: &'static str,
}

/// The planned rows (empty when the plan failed).
static ROWS: Mutex<Vec<Row>> = Mutex::new(Vec::new());

/// Reads a NUL-terminated string at `p` if it lies inside the image (at most 32 bytes).
unsafe fn image_str(base: usize, size: usize, p: u64) -> Option<String> {
    let p = p as usize;
    if p < base || p + 32 > base + size {
        return None;
    }
    let b = std::slice::from_raw_parts(p as *const u8, 32);
    let n = b.iter().position(|&c| c == 0)?;
    Some(String::from_utf8_lossy(&b[..n]).into_owned())
}

fn read_ann_header(path: &Path) -> Result<Vec<(u32, String)>, String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut c = [0u8; 4];
    f.read_exact(&mut c).map_err(|e| e.to_string())?;
    let count = u32::from_le_bytes(c);
    if count > 4096 {
        return Err(format!("{}: slot count {count} is not plausible", path.display()));
    }
    let mut h = c.to_vec();
    h.resize(motions::ann_header_len(count), 0);
    f.read_exact(&mut h[4..]).map_err(|e| e.to_string())?;
    motions::ann_header(&h)
}

fn first_existing(cands: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    cands.iter().find(|p| p.is_file()).cloned()
}

/// Plans the swap at start. `mod_dir` is Panacea's mod folder (`mod_path\Recom`).
pub unsafe fn prepare(base: usize, size: usize, mod_dir: &Path, game_dir: &Path) {
    let table = va(base, motions::SORA_TABLE);
    let char1 = *(va(base, motions::CHAR_TABLES + 8) as *const u64) as usize;
    if char1 != table {
        log(&format!("kh1 motions: character table 1 is {char1:#x}, expected Sora's {table:#x}; switch disabled"));
        return;
    }
    let mut names = Vec::with_capacity(motions::SORA_ROWS);
    for r in 0..motions::SORA_ROWS {
        let p = *((table + r * 16) as *const u64);
        names.push(image_str(base, size, p));
    }
    let manifest_path = first_existing(&[
        mod_dir.join("kh1-motions.json"),
        mod_dir.join("PLAYER").join("0001").join("kh1-motions.json"),
        game_dir.join("kh1-motions.json"),
    ]);
    let ann_path = first_existing(&[
        mod_dir.join("PLAYER").join("0001").join("PL0001.ANN"),
        mod_dir.join("raw").join("PLAYER").join("0001").join("PL0001.ANN"),
    ]);
    let (Some(mp), Some(ap)) = (manifest_path, ann_path) else {
        log(&format!(
            "kh1 motions: no kh1-motions.json and/or PLAYER\\0001\\PL0001.ANN under {} (asset build step not run?); switch has no effect",
            mod_dir.display()
        ));
        return;
    };
    let plan = std::fs::read_to_string(&mp)
        .map_err(|e| format!("{}: {e}", mp.display()))
        .and_then(|t| motions::parse_manifest(&t))
        .and_then(|m| Ok((m, read_ann_header(&ap)?)))
        .and_then(|(m, a)| motions::plan(&m, &names, &a));
    let plan: Vec<RowPatch> = match plan {
        Ok(p) => p,
        Err(e) => {
            log(&format!("kh1 motions: {e}; switch has no effect"));
            return;
        }
    };
    log(&format!("kh1 motions: manifest {}, motion pack {}", mp.display(), ap.display()));
    let mut rows = Vec::new();
    for p in plan {
        let addr = table + p.row as usize * 16;
        let recom = *(addr as *const u64);
        // Our name lives for the whole process (Re:CoM keeps the pointer).
        let mut s = p.kh1_name.clone().into_bytes();
        s.push(0);
        let kh1 = Box::leak(s.into_boxed_slice()).as_ptr() as u64;
        log(&format!(
            "kh1 motions: row {:#04x} {} {} -> {} (slot {}, KH1 entry {})",
            p.row,
            p.what,
            p.recom_name,
            p.kh1_name,
            p.slot,
            p.entry.map(|e| e.to_string()).unwrap_or_else(|| "?".into())
        ));
        rows.push(Row { addr, recom, kh1, what: p.what });
    }
    if let Ok(mut g) = ROWS.lock() {
        *g = rows;
    }
}

/// Points the rows at the KH1 clips (`on`) or back at Re:CoM's. Returns false if the switch
/// can't apply (no plan) or a row was not as expected.
pub fn apply(on: bool) -> bool {
    let Ok(rows) = ROWS.lock() else { return false };
    if rows.is_empty() {
        return false;
    }
    let mut ok = true;
    for r in rows.iter() {
        let (from, to) = if on { (r.recom, r.kh1) } else { (r.kh1, r.recom) };
        // SAFETY: `addr` is a row of Sora's action table in the exe's .data, checked at prepare.
        let now = unsafe { *(r.addr as *const u64) };
        if now == to {
            continue;
        }
        if !unsafe { hook::patch_u64(r.addr, from, to, r.what) } {
            ok = false;
        }
    }
    log(&format!("kh1 motions: {} (Re:CoM resolves the rows on the next room load)", if on { "on" } else { "off" }));
    ok
}
