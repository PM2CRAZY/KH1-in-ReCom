//! A new hair on Re:CoM Riku's head, and nothing else of the source model (the asset side of the hair swap; fork owner, 2026-11-24:
//! "o cabelo começa sim pelo do Riku do KH2"; the DDD hair this was first written for was removed by Stage 363). The source is a
//! [`HairSource`] (today KH2 Riku's, [`crate::kh2_hair`]).
//!
//! What it does, for Riku's default attire `PLAYER/0101/PL0101.MDL`:
//!
//! 1. **Re:CoM's hair triangles** are found per triangle in part 3 (the textured body) on texture id 1 (`n_zz110_01`, a
//!    1024 x 1024 atlas of face, hair, skin and trousers). The atlas is cut into UV islands (triangles that share a
//!    corner's UV); an island is hair when one of its triangles touches a hair bone (`*hair*`), and the triangles of a hair
//!    island that touch the head bone or a hair bone are removed. This is exact on the atlas; no colour heuristic.
//!    Checked (files: `PL0101.MDL`): 716 triangles touch the head or hair bones, 366 of them are on hair islands, the
//!    other 350 are the face island (310), the ear / neck pieces; no triangle in a hair island touches another bone.
//! 2. Part 2 (untextured, positions only) is **not** part 3's twin: it holds 1074 triangles at other positions (a low-poly
//!    head and hair proxy). It has no UVs, so its hair is the triangles that touch a hair bone; the head-bone skull stays.
//! 3. The source's triangles come with a Re:CoM bone each and a position in Re:CoM's frame; a corner's bind-pose position is
//!    put in that bone's space with the inverse of the Re:CoM bone's rest world. Re:CoM has one bone per vertex.
//! 4. The atlas is rebuilt: Re:CoM's pixels are kept only under the kept triangles (dilated), the rest is the gutter
//!    colour, and the source's hair islands are copied into the free space with a per-island translation and, when the
//!    rectangles do not fit at full size, one common scale below 1. The new atlas has its own texture name
//!    ([`HairTarget::new_texture`]): the game looks texture names up globally, so the original `n_zz110_01` must not be
//!    shadowed. Written as the HD DDS and as the 256-colour TIM2 ([`crate::keyblade::texture_entry`]).
//! 5. The remaining triangles are written again as strips in packets (as `keyblade::packets` does), no packet over the
//!    vertex count Square's own packets reach in that part, and the per-bone bounds are recomputed. Each source triangle is
//!    written twice, with the two facing flags, so that one copy passes a facing cull from any side (0.186.1).
//!
//! The facing flag of a drawing vertex (0 or 0x20) is `0x20` when `cross(b - a, c - a)` of the strip-order corners points
//! away from the head's centre. Checked (files: 96.6% of PL0101's head-bone-only textured triangles, 394 of 408).

use std::collections::{BTreeMap, HashMap, HashSet};

use kh1_formats::egs::HdAsset;

use crate::ann::{cstr, name16};
use crate::math::Aff;
use crate::mdl::{join_parts, split_parts, MdlPart, MeshPacket, MeshVertex, RecomSkeleton, FMT_PLAIN, FMT_TEXTURED, MAX_PACKET_QW};
use crate::texture::{decode_dds, Image};
use crate::{bad, Error, Result};

/// Re:CoM's head bone.
pub const HEAD_BONE: &str = "head";
/// Where the head's centre is relative to the `head` bone's rest origin (metres): used only to decide which way a
/// triangle faces. Checked (files): with this point 394 of 408 head-bone-only textured triangles of PL0101 follow the rule.
pub const HEAD_CENTRE: [f64; 3] = [0.0, 0.07, 0.04];

/// Which Re:CoM texture the new atlas replaces, and the name of the new one.
#[derive(Clone, Copy, Debug)]
pub struct HairTarget {
    /// The atlas's texture name in the model (`n_zz110_01`).
    pub texture: &'static str,
    /// The new atlas's name (at most 15 bytes; the RTM entry is `<name>.tm2`). Unique in the game: Re:CoM looks names up
    /// globally.
    pub new_texture: &'static str,
    /// 0.198.7: a second texture of the model with hair on it (Dark Riku's `PL0201`: `n_zz120_02`, 91 hair-bone triangles beside
    /// the 187 on `n_zz120_01`, Checked (files)); its hair triangles are removed the same way, nothing is kept from it for the fit.
    pub also: Option<&'static str>,
    /// 0.198.33: the open back of the neck ([`in_neck_skirt_window`]) is joined to the hair by a skirt (Dark Riku's model with BBS's
    /// hair, see [`NECK_SKIRT_Y`]).
    pub neck_skirt: bool,
}

/// 0.198.33 (fork owner, 2026-11-27, on the battle's Dark Riku with BBS kid Riku's hair: "percebi que o cabelo do bbs tem alguns
/// espaços vazios no modo dark mode, testei pulando e atacando"). Checked (files, tests `riku_skin::tests::dark_mode_neck_open_edges`
/// / `dark_mode_hair_hole_scan`): with Re:CoM's hair and its nape base ([`ALSO_NAPE`]) gone, Dark Riku's neck ends in an open edge
/// behind (`neck1` / `neck2` 0.8 cm under the head bone and 6.3 cm behind it, `head` / `neck1` 1.8 cm over and 4.4 cm behind, `head`
/// 3.9 cm over at 7.3 cm out, the face's back edge 6.2 cm over at 9.2 cm out), 1.4 to 2.9 cm from BBS's nearest hair corner. Under the
/// short hair that opening shows the inside of the neck, which the game culls: the background, worst when his own jumps and swings
/// turn the head on the neck (`neck1` 3-7 cm off its rest place in the head's frame). KH2's hair (207 / 208) hangs over it. Each open
/// edge of the kept `head` / `neck*` triangles in this window (head-relative, metres: height range, behind [`NECK_SKIRT_Z`], within
/// [`NECK_SKIRT_X`]) is joined to its corners' nearest hair corners by two-sided triangles, as the skirt (0.196.15): the bottom follows
/// the neck, the top the hair. They wear the edge's own texture and texels (the coat's on the neck, the skin's behind the ear), not
/// the hair's: in his air finisher, swings and jump the band webs out in front of his neck (sides up to 11.2 cm), where it reads as
/// the neck.
pub const NECK_SKIRT_Y: [f64; 2] = [-0.03, 0.075];
/// See [`NECK_SKIRT_Y`].
pub const NECK_SKIRT_Z: f64 = -0.02;
/// See [`NECK_SKIRT_Y`].
pub const NECK_SKIRT_X: f64 = 0.105;
/// See [`NECK_SKIRT_Y`]: the farthest a corner of the edge may be from its hair corner (metres).
pub const NECK_SKIRT_REACH: f64 = 0.04;
/// See [`NECK_SKIRT_Y`]: how much nearer the neck's axis than the edge's corner its hair corner may be (metres).
pub const NECK_SKIRT_IN: f64 = 0.005;
/// 0.198.35: how far (metres) the skirt's top corners sit in under the hair corners they end at, toward the head's centre
/// ([`HEAD_CENTRE`]), on `head`.
pub const NECK_SKIRT_TUCK: f64 = 0.03;

/// Whether an open edge with midpoint `p` (world rest; `head` = the head bone's rest origin) is in [`NECK_SKIRT_Y`]'s window.
pub fn in_neck_skirt_window(p: [f64; 3], head: [f64; 3]) -> bool {
    let r = [p[0] - head[0], p[1] - head[1], p[2] - head[2]];
    (NECK_SKIRT_Y[0]..=NECK_SKIRT_Y[1]).contains(&r[1]) && r[2] < NECK_SKIRT_Z && r[0].abs() < NECK_SKIRT_X
}

/// Riku's default attire (`PL0101`) with KH2 Riku's hair ([`crate::kh2_hair`]): the new atlas's name.
pub const RIKU_0101_KH2: HairTarget = HairTarget { texture: "n_zz110_01", new_texture: "rk_hair_kh2", also: None, neck_skirt: false };

/// 0.197.2: Riku's Halloween Town attire (`PL0111`, the same skeleton as `PL0101`, Checked (files)) with KH2 Riku's hair. The hair's
/// texture keeps one name for every group (Re:CoM looks names up globally; the picture is the same).
pub const RIKU_0111_KH2: HairTarget = HairTarget { texture: "n_zz110_11", new_texture: "rk_hair_kh2", also: None, neck_skirt: false };

/// 0.197.2: the Halloween Town attire with BBS kid Riku's hair.
/// 0.198.13: its own texture name. The BBS hair's picture holds each model's temple patches ([`HairSource::painted_temple_window`]),
/// so it differs per model, and Re:CoM looks names up over every loaded model: with one name, the Dark Form's temples read Riku's
/// default picture in game (a yellow-orange triangle, the kid's vest; fork owner: "costeleta ficou até pior...").
pub const RIKU_0111_BBS: HairTarget = HairTarget { texture: "n_zz110_11", new_texture: "rk_hair_bbs_ht", also: None, neck_skirt: false };

/// The same attire with BBS kid Riku's hair ([`crate::bbs_hair`]).
pub const RIKU_0101_BBS: HairTarget = HairTarget { texture: "n_zz110_01", new_texture: "rk_hair_bbs", also: None, neck_skirt: false };

/// 0.198.7 (fork owner, 2026-11-26: "agora implementa todos os cabelos para o a Skin do riku Darkmode tbm"): Dark Riku's model (`PL0201`,
/// the Dark Form skin's) with KH2 Riku's hair: his hair is on `n_zz120_01` (with the face) and `n_zz120_02` (with the coat). Checked (files).
pub const RIKU_0201_KH2: HairTarget = HairTarget { texture: "n_zz120_01", new_texture: "rk_hair_kh2", also: Some("n_zz120_02"), neck_skirt: false };
/// The same on his Halloween Town model (`PL0211`).
pub const RIKU_0211_KH2: HairTarget = HairTarget { texture: "n_zz120_11", new_texture: "rk_hair_kh2", also: Some("n_zz120_12"), neck_skirt: false };
/// Dark Riku's model with BBS kid Riku's hair.
/// (0.198.13: its own texture name, see [`RIKU_0111_BBS`].)
pub const RIKU_0201_BBS: HairTarget = HairTarget { texture: "n_zz120_01", new_texture: "rk_hair_bbs_df", also: Some("n_zz120_02"), neck_skirt: true };
/// The same on his Halloween Town model.
pub const RIKU_0211_BBS: HairTarget = HairTarget { texture: "n_zz120_11", new_texture: "rk_hair_bbs_dh", also: Some("n_zz120_12"), neck_skirt: true };

/// 0.198.7: the window (head-relative, metres: half-width, lowest and highest height, the most forward depth) in which whole head / neck
/// triangles of the second hair texture ([`HairTarget::also`]) go with the old hair: the base of Dark Riku's hair over his nape.
pub const ALSO_NAPE: [f64; 4] = [0.04, -0.06, 0.0, -0.05];

/// One corner of a source hair triangle, already in Re:CoM's frame.
#[derive(Clone, Debug)]
pub struct SrcVert {
    /// Bind-pose position in the Re:CoM skeleton's frame (metres).
    pub pos: [f64; 3],
    /// Texture coordinates in the source atlas (unwrapped, may lie outside 0..1: the atlas repeats).
    pub uv: [f64; 2],
    /// The Re:CoM bone the corner follows.
    pub bone: String,
}

/// One source hair triangle.
#[derive(Clone, Debug)]
pub struct SrcTri {
    /// The corners, in strip order.
    pub v: [SrcVert; 3],
    /// UV island id in the source atlas.
    pub island: usize,
    /// True when this triangle continues the previous one's strip (only its third corner is new).
    pub cont: bool,
    /// A lining triangle (a copy of a hair triangle pulled toward the head's centre, BBS 0.194.2, KH2 0.197.1).
    pub lining: bool,
}

/// A hair ready for the swap, whatever its game.
pub struct SrcHair {
    /// The hair triangles.
    pub tris: Vec<SrcTri>,
    /// Number of UV islands (ids `0..islands`).
    pub islands: usize,
    /// Skin triangles left out (report only).
    pub skin: usize,
    /// The source atlas (same size as Re:CoM's).
    pub atlas: Image,
    /// One-sided (0.194.3, BBS): each triangle is written once, in the source's own winding, all with the facing flag that
    /// [`facing`] gives the majority of them (the source's outside); otherwise two-sided (DDD, KH2).
    pub one_sided: bool,
}

/// Where a hair comes from: it builds its triangles in Re:CoM's frame given Re:CoM's bone names and rest world matrices.
pub trait HairSource {
    /// The hair for a model with these bones (`names[i]`, `world[i]` = its rest world matrix).
    fn hair(&self, names: &[String], world: &[Aff]) -> Result<SrcHair>;
    /// The model before the swap: a source that moves Re:CoM's hair bones its own way re-poses them here (0.195.0, KH2).
    fn prepare_mdl(&self, mdl: &[u8]) -> Result<Vec<u8>> {
        Ok(mdl.to_vec())
    }
    /// Re:CoM's face as it stays after the swap (world rest triangles), given before [`HairSource::hair`] (0.195.4: the KH2 hair
    /// fits itself to it).
    fn set_face(&self, _face: &[[[f64; 3]; 3]]) {}
    /// Moves a vertex of Re:CoM's face that stays (world rest position; the head bone's at rest is `head`) (0.196.1: the KH2 hair
    /// flattens the ear its own model has no ear for).
    fn adjust_face(&self, p: [f64; 3], _head: [f64; 3]) -> [f64; 3] {
        p
    }
    /// Whether the plain copy of one of the source's hair triangles (part 2) is left out, given the triangle and the head bone's rest
    /// position (0.196.4: the flat copy of a head-bound KH2 triangle at the ear showed as a flat polygon).
    fn drop_plain_hair_copy(&self, _t: &SrcTri, _head: [f64; 3]) -> bool {
        false
    }
    /// Whether the open head edge with midpoint `p` (world rest; the head bone's at rest is `head`) gets a skirt to the source's hair
    /// (0.196.15).
    fn skirt_edge(&self, _p: [f64; 3], _head: [f64; 3]) -> bool {
        false
    }
    /// Whether a head-bone triangle of the plain proxy (part 2: low-poly, flat, drawn without a facing cull, made to hide under Re:CoM's
    /// own bulky hair), centroid `p` (world rest; the head bone's at rest is `head`), is dropped (0.196.4: under the KH2 hair it shows
    /// through the gaps as a flat lilac polygon).
    fn drop_proxy_triangle(&self, _p: [f64; 3], _head: [f64; 3]) -> bool {
        false
    }
    /// Whether a kept head-bone triangle (centroid `p`, world rest; the head bone's at rest is `head`) is written twice, with
    /// both facing flags, like the source's hair triangles (0.196.3: the KH2 hair opens the ear's window, so the temple's
    /// single-copy triangles must pass the game's facing cull from either side).
    fn two_sided_face(&self, _p: [f64; 3], _head: [f64; 3]) -> bool {
        false
    }
    /// Whether the hair is packed into Re:CoM's atlas (renamed to the target's name) as until 0.196.x, instead of its own
    /// texture at the source's size (0.197.0, the default: the KH2 hair's islands were packed at 0.4 of their size).
    fn packed_atlas(&self) -> bool {
        false
    }
    /// Whether the lining's triangles also get their flat copy in the plain proxy (part 2), as every hair triangle does (BBS, as
    /// 0.194.2 was seen in game); KH2's do not (0.197.1: the proxy's flat copies show as flat polygons, 0.196.4).
    fn lining_in_plain(&self) -> bool {
        true
    }
    /// Whether the open head edge with midpoint `p` (world rest; the head bone's at rest is `head`) gets a curtain of its own skin down
    /// to the returned height under `head` (metres, negative), its bottom corners on the returned bone (0.198.2: KH2's nape).
    fn nape_curtain(&self, _p: [f64; 3], _head: [f64; 3]) -> Option<(f64, &'static str)> {
        None
    }
    /// 0.198.6: the nape curtain wears the nearest hair vertex's texel (and the hair's texture) instead of the skin's, and its bottom
    /// corners are pulled this far (metres) toward the bottom bone's origin, level, so they sit inside what covers that bone (the
    /// collar). `None`: skin, straight down (0.198.2).
    fn nape_curtain_hair(&self) -> Option<f64> {
        None
    }
    /// 0.198.13 (fork owner on the BBS hair: "tem a do cabelo mesmo e tem a da cabeça base ao lado, ficou estranho"): Re:CoM's face
    /// atlas has a lock of hair painted on the skin of each temple, in front of the ear (hidden under Re:CoM's own hair until the
    /// swap). The window (head-relative, metres: least and most |x|, lowest and highest y, least and most z) of the kept head
    /// triangles that [`unpaint_temples`] moves onto clean skin of the same atlas. `None`: the face's texture as it is.
    fn painted_temple_window(&self) -> Option<[f64; 6]> {
        None
    }
    /// The look's own motion pack, made from Re:CoM's (`ann`) for the original model `mdl` (none: Re:CoM's pack as it is).
    fn bake_ann(&self, _ann: &[u8], _mdl: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
}

/// What a swap did (for logs and tests).
#[derive(Clone, Debug, Default)]
pub struct HairReport {
    /// Part 3's triangles on the atlas texture / removed from it / kept from it.
    pub riku_on_atlas: usize,
    /// Removed from part 3.
    pub removed: usize,
    /// Removed from part 2 (the triangles touching a hair bone).
    pub removed_plain: usize,
    /// Part 3's triangles on other textures (kept untouched).
    pub other_texture: usize,
    /// Triangles in a hair island that touch neither the head nor a hair bone (kept).
    pub stray_in_hair_islands: usize,
    /// Open head edges given the nape curtain (0.198.2).
    pub nape: usize,
    /// Triangles removed from the second hair texture ([`HairTarget::also`], 0.198.7).
    pub also_removed: usize,
    /// The source's hair triangles added (DDD's, or KH2's).
    pub ddd_hair: usize,
    /// The source's skin triangles left out (DDD only).
    pub ddd_skin: usize,
    /// The source's hair UV islands.
    pub ddd_islands: usize,
    /// Rectangles copied into the atlas (islands inside another's rectangle share it).
    pub atlas_rects: usize,
    /// The common scale of the copied islands.
    pub atlas_scale: f64,
    /// Plain copies of the source's hair triangles left out ([`HairSource::drop_plain_hair_copy`]).
    pub plain_hair_dropped: usize,
    /// Skirt triangles added between the head's open edges and the source's hair ([`HairSource::skirt_edge`]).
    pub skirt: usize,
    /// Head-bone triangles of the plain proxy dropped ([`HairSource::drop_proxy_triangle`]).
    pub proxy_dropped: usize,
    /// Kept head-bone triangles written a second time with the other facing ([`HairSource::two_sided_face`]).
    pub face_two_sided: usize,
    /// Kept head triangles moved off the temples' painted lock ([`HairSource::painted_temple_window`], 0.198.13), and the lock's
    /// texels filled with skin in their copy.
    pub temple_unpainted: (usize, usize),
    /// Packets in part 3 / part 2 before and after.
    pub packets: ([usize; 2], [usize; 2]),
}

fn skirt_segs_push(segs: &mut Vec<Seg>, tex: u32, a: MeshVertex, b: MeshVertex, c: MeshVertex) {
    segs.push(Seg { tex, verts: vec![a, b, c] });
}

/// The swap's result.
pub struct HairSwap {
    /// The new `.MDL`: parts 0 and 1 (the weapon) as they were, parts 2 and 3 rebuilt.
    pub mdl: Vec<u8>,
    /// The new `.RTM`: `<new_texture>.tm2` added (0.197.0; with [`HairSource::packed_atlas`]: the atlas entry replaced by it).
    pub rtm: Vec<u8>,
    /// The RTM's HD assets: `<new_texture>_tm2.dds` added (packed: the old atlas's DDS replaced by it).
    pub hd: Vec<HdAsset>,
    /// The new texture's DDS (the one in `hd`): the hair's own picture (packed: the new atlas).
    pub dds: Vec<u8>,
    /// Part 3's texture index of the hair's triangles (packed: the atlas's, 1).
    pub hair_tex: u32,
    /// Riku's atlas's DDS as it was (debug renders of the face).
    pub atlas_dds: Vec<u8>,
    /// Counts.
    pub report: HairReport,
}

// ---------------------------------------------------------------------------------------------
// Small helpers.
// ---------------------------------------------------------------------------------------------

const F32_ONE: u32 = 0x3F80_0000;
const START_Q0: [u32; 4] = [0xFFFF, F32_ONE, 0, 0xA000];
/// Pixels around a kept triangle's footprint that keep Re:CoM's colours (bilinear filtering and the quantiser's 4x
/// downscale read a little outside the triangle).
const KEEP_DILATE: usize = 6;
/// Pixels around a DDD island's UV box that are copied with it.
const PAD: i32 = 6;
/// The common scales tried for DDD's hair islands, largest first.
const SCALES: [f64; 16] = [1.0, 0.98, 0.96, 0.94, 0.92, 0.9, 0.85, 0.8, 0.7, 0.6, 0.55, 0.5, 0.45, 0.4, 0.35, 0.3];

struct Dsu(Vec<usize>);

impl Dsu {
    fn new() -> Dsu {
        Dsu(Vec::new())
    }
    fn make(&mut self) -> usize {
        self.0.push(self.0.len());
        self.0.len() - 1
    }
    fn find(&mut self, mut x: usize) -> usize {
        while self.0[x] != x {
            self.0[x] = self.0[self.0[x]];
            x = self.0[x];
        }
        x
    }
    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.0[a.max(b)] = a.min(b);
        }
    }
}

/// UV islands: `corners[t]` are triangle `t`'s corner keys; returns each triangle's dense island id (in order of first
/// appearance) and the island count.
pub fn islands<K: std::hash::Hash + Eq + Copy>(corners: &[[K; 3]]) -> (Vec<usize>, usize) {
    let mut ids: HashMap<K, usize> = HashMap::new();
    let mut d = Dsu::new();
    let mut idx = |k: K, d: &mut Dsu| *ids.entry(k).or_insert_with(|| d.make());
    let mut first = Vec::with_capacity(corners.len());
    for c in corners {
        let a = idx(c[0], &mut d);
        let b = idx(c[1], &mut d);
        let e = idx(c[2], &mut d);
        d.union(a, b);
        d.union(a, e);
        first.push(a);
    }
    let mut dense: BTreeMap<usize, usize> = BTreeMap::new();
    let mut out = Vec::with_capacity(corners.len());
    for a in first {
        let r = d.find(a);
        let n = dense.len();
        out.push(*dense.entry(r).or_insert(n));
    }
    (out, dense.len())
}

fn inv3(m: &[[f64; 3]; 3]) -> Result<[[f64; 3]; 3]> {
    let c = |i: usize, j: usize| -> f64 {
        let (r0, r1) = ((i + 1) % 3, (i + 2) % 3);
        let (c0, c1) = ((j + 1) % 3, (j + 2) % 3);
        m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0]
    };
    let det = m[0][0] * c(0, 0) + m[0][1] * c(0, 1) + m[0][2] * c(0, 2);
    if det.abs() < 1e-12 {
        return bad("hair: a rest matrix is singular");
    }
    let mut o = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            o[i][j] = c(j, i) / det;
        }
    }
    Ok(o)
}

/// A world point in a bone's space (row vectors: `world = local · r + t`).
fn to_local(w: &Aff, inv_r: &[[f64; 3]; 3], p: [f64; 3]) -> [f64; 3] {
    let d = [p[0] - w.t[0], p[1] - w.t[1], p[2] - w.t[2]];
    crate::math::vm(d, inv_r)
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The facing flag of the strip-order triangle `p` (world points) for a head centred at `centre` (see the module docs).
pub fn facing(p: [[f64; 3]; 3], centre: [f64; 3]) -> u32 {
    let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
    let c = [(p[0][0] + p[1][0] + p[2][0]) / 3.0, (p[0][1] + p[1][1] + p[2][1]) / 3.0, (p[0][2] + p[1][2] + p[2][2]) / 3.0];
    if dot(n, sub(c, centre)) > 0.0 {
        0x20
    } else {
        0
    }
}

// ---------------------------------------------------------------------------------------------
// Re:CoM's side.
// ---------------------------------------------------------------------------------------------

/// One triangle of a Re:CoM part: its three vertices in strip order, and where it came from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RTri {
    pub(crate) v: [MeshVertex; 3],
    strip: usize,
    pos: usize,
    pub(crate) tex: u32,
}

fn same_vertex(a: &MeshVertex, b: &MeshVertex) -> bool {
    a.q1 == b.q1 && a.q2 == b.q2
}

/// The triangles of a draw list: two starting vertices, then one triangle per drawing vertex. Square's packets are
/// exactly that; anything else is refused.
pub(crate) fn extract(list: &[MeshPacket]) -> Result<Vec<RTri>> {
    let mut out = Vec::new();
    let mut strip = 0usize;
    for pk in list {
        let (mut starts, mut pos) = (0, 0);
        for (i, v) in pk.verts.iter().enumerate() {
            if !v.draws() {
                if v.q0 != START_Q0 {
                    return bad("hair: a strip's starting vertex is not as Square writes it");
                }
                if i == 0 || pk.verts[i - 1].draws() {
                    strip += 1;
                    starts = 1;
                    pos = 0;
                } else {
                    starts += 1;
                }
                if starts > 2 {
                    return bad("hair: three starting vertices in a row");
                }
            } else {
                if starts != 2 {
                    return bad("hair: a drawing vertex without two before it");
                }
                out.push(RTri { v: [pk.verts[i - 2], pk.verts[i - 1], *v], strip, pos, tex: v.q2.map_or(0, |q| q[3]) });
                pos += 1;
            }
        }
    }
    Ok(out)
}

/// A strip: the vertices, the first two as they are drawn after a restart.
struct Seg {
    tex: u32,
    verts: Vec<MeshVertex>,
}

fn start_of(v: MeshVertex) -> MeshVertex {
    MeshVertex { q0: START_Q0, ..v }
}

/// Runs of consecutive triangles of one source strip become one strip each.
fn segments(tris: &[&RTri]) -> Vec<Seg> {
    let mut segs: Vec<Seg> = Vec::new();
    let mut last: Option<&RTri> = None;
    for &t in tris {
        let cont = last.is_some_and(|l| l.strip == t.strip && l.pos + 1 == t.pos && l.tex == t.tex && same_vertex(&l.v[1], &t.v[0]) && same_vertex(&l.v[2], &t.v[1]));
        if cont {
            segs.last_mut().unwrap().verts.push(t.v[2]);
        } else {
            segs.push(Seg { tex: t.tex, verts: vec![start_of(t.v[0]), start_of(t.v[1]), t.v[2]] });
        }
        last = Some(t);
    }
    segs
}

/// Packs strips into packets of one texture and at most `max_v` vertices (a split strip continues from its last two
/// vertices), as `keyblade::packets` does.
fn pack(segs: &[Seg], fmt: u16, gif: u32, regs: u32, max_v: usize) -> Vec<MeshPacket> {
    let textured = fmt == FMT_TEXTURED;
    let mut out: Vec<MeshPacket> = Vec::new();
    let mut cur_tex = u32::MAX;
    for seg in segs {
        let tex = if textured { seg.tex } else { 0 };
        let mut i = 0;
        while i + 2 < seg.verts.len() {
            let need_new = match out.last() {
                None => true,
                Some(p) => tex != cur_tex || p.verts.len() + 3 > max_v,
            };
            if need_new {
                out.push(MeshPacket { fmt, gif, regs, verts: Vec::new(), count: None });
                cur_tex = tex;
            }
            let p = out.last_mut().unwrap();
            let take = (seg.verts.len() - i).min(max_v - p.verts.len());
            for (k, v) in seg.verts[i..i + take].iter().enumerate() {
                let mut v = if k < 2 { start_of(*v) } else { *v };
                if !textured {
                    v.q2 = None;
                }
                p.verts.push(v);
            }
            i += take;
            if i < seg.verts.len() {
                i -= 2;
            }
        }
    }
    out
}

fn max_vertices(list: &[MeshPacket], stride: usize) -> usize {
    let limit = (MAX_PACKET_QW - 2) / stride;
    list.iter().map(|p| p.verts.len()).max().map_or(limit, |m| m.clamp(3, limit))
}

fn uv_key(v: &MeshVertex) -> (i64, i64) {
    let uv = v.uv().unwrap_or([0.0, 0.0]);
    ((uv[0] as f64 * 1e5).round() as i64, (uv[1] as f64 * 1e5).round() as i64)
}

/// What [`classify_riku`] found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RikuSplit {
    /// Triangles of the part's body list on the atlas texture.
    pub on_atlas: usize,
    /// Marked for removal.
    pub removed: usize,
    /// Triangles on other textures.
    pub other_texture: usize,
    /// Triangles in hair islands that touch neither head nor hair bones (kept).
    pub stray: usize,
}

/// Marks Re:CoM's hair triangles on texture `atlas` (see the module docs): `remove[i]` for `tris[i]`.
fn classify_riku(tris: &[RTri], atlas: u32, head: usize, hair_bones: &HashSet<usize>) -> (Vec<bool>, RikuSplit) {
    let on: Vec<usize> = (0..tris.len()).filter(|&i| tris[i].tex == atlas).collect();
    let corners: Vec<[(i64, i64); 3]> = on.iter().map(|&i| tris[i].v.each_ref().map(uv_key)).collect();
    let (isl, n) = islands(&corners);
    let touches = |t: &RTri, f: &dyn Fn(usize) -> bool| t.v.iter().any(|v| f(v.bone()));
    let mut hair_island = vec![false; n];
    for (k, &i) in on.iter().enumerate() {
        if touches(&tris[i], &|b| hair_bones.contains(&b)) {
            hair_island[isl[k]] = true;
        }
    }
    let mut remove = vec![false; tris.len()];
    let mut s = RikuSplit { on_atlas: on.len(), other_texture: tris.len() - on.len(), ..Default::default() };
    for (k, &i) in on.iter().enumerate() {
        if !hair_island[isl[k]] {
            continue;
        }
        if touches(&tris[i], &|b| b == head || hair_bones.contains(&b)) {
            remove[i] = true;
            s.removed += 1;
        } else {
            s.stray += 1;
        }
    }
    (remove, s)
}

// ---------------------------------------------------------------------------------------------
// The atlas.
// ---------------------------------------------------------------------------------------------

/// Marks the pixels a UV triangle (pixel coordinates) covers, with `slack` pixels of tolerance.
fn raster(mask: &mut [bool], w: usize, h: usize, t: [[f64; 2]; 3], slack: f64) {
    let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in t {
        x0 = x0.min(p[0]);
        x1 = x1.max(p[0]);
        y0 = y0.min(p[1]);
        y1 = y1.max(p[1]);
    }
    let (xa, xb) = ((x0 - slack).floor().max(0.0) as usize, ((x1 + slack).ceil().max(0.0) as usize).min(w - 1));
    let (ya, yb) = ((y0 - slack).floor().max(0.0) as usize, ((y1 + slack).ceil().max(0.0) as usize).min(h - 1));
    let area = (t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0]);
    let sg = if area < 0.0 { -1.0 } else { 1.0 };
    for y in ya..=yb {
        for x in xa..=xb {
            let p = [x as f64 + 0.5, y as f64 + 0.5];
            let mut inside = true;
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
                let d = if len < 1e-9 { 0.0 } else { sg * ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])) / len };
                if d < -slack {
                    inside = false;
                    break;
                }
            }
            if inside {
                mask[y * w + x] = true;
            }
        }
    }
}

/// Square dilation by `r` pixels.
fn dilate(mask: &[bool], w: usize, h: usize, r: usize) -> Vec<bool> {
    let pass = |src: &[bool], horizontal: bool| -> Vec<bool> {
        let mut out = vec![false; w * h];
        let (outer, inner) = if horizontal { (h, w) } else { (w, h) };
        for o in 0..outer {
            let at = |i: usize| if horizontal { o * w + i } else { i * w + o };
            let mut pre = vec![0u32; inner + 1];
            for i in 0..inner {
                pre[i + 1] = pre[i] + src[at(i)] as u32;
            }
            for i in 0..inner {
                let (a, b) = (i.saturating_sub(r), (i + r + 1).min(inner));
                out[at(i)] = pre[b] > pre[a];
            }
        }
        out
    };
    pass(&pass(mask, true), false)
}

/// A summed-area table over a boolean grid, for "is this rectangle free" in O(1).
struct Sat {
    w: usize,
    h: usize,
    s: Vec<u32>,
}

impl Sat {
    fn new(occ: &[bool], w: usize, h: usize) -> Sat {
        let mut s = vec![0u32; (w + 1) * (h + 1)];
        for y in 0..h {
            let mut row = 0;
            for x in 0..w {
                row += occ[y * w + x] as u32;
                s[(y + 1) * (w + 1) + x + 1] = s[y * (w + 1) + x + 1] + row;
            }
        }
        Sat { w, h, s }
    }
    fn free(&self, x: usize, y: usize, rw: usize, rh: usize) -> bool {
        if x + rw > self.w || y + rh > self.h {
            return false;
        }
        let i = |xx: usize, yy: usize| self.s[yy * (self.w + 1) + xx];
        i(x + rw, y + rh) + i(x, y) == i(x + rw, y) + i(x, y + rh)
    }
}

/// One rectangle of DDD's atlas copied into the new one.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    /// Source rectangle in DDD's pixels (may lie outside 0..w: the atlas repeats).
    sx: i32,
    sy: i32,
    sw: i32,
    sh: i32,
    /// Destination top-left, and the scale.
    dx: i32,
    dy: i32,
}

/// Source rectangles of the hair islands (box of the unwrapped UVs times the size, padded), and which rectangle serves
/// which island (an island inside another's rectangle shares its copy).
fn island_rects(hair: &SrcHair, size: (usize, usize)) -> (Vec<[i32; 4]>, Vec<usize>) {
    let n = hair.islands;
    let mut bb = vec![[f64::MAX, f64::MAX, f64::MIN, f64::MIN]; n];
    let mut used = vec![false; n];
    for t in &hair.tris {
        used[t.island] = true;
        for v in &t.v {
            let b = &mut bb[t.island];
            b[0] = b[0].min(v.uv[0] * size.0 as f64);
            b[1] = b[1].min(v.uv[1] * size.1 as f64);
            b[2] = b[2].max(v.uv[0] * size.0 as f64);
            b[3] = b[3].max(v.uv[1] * size.1 as f64);
        }
    }
    let mut order: Vec<usize> = (0..n).filter(|&i| used[i]).collect();
    let rect = |i: usize| [bb[i][0].floor() as i32 - PAD, bb[i][1].floor() as i32 - PAD, bb[i][2].ceil() as i32 + PAD, bb[i][3].ceil() as i32 + PAD];
    let area = |r: [i32; 4]| (r[2] - r[0]) as i64 * (r[3] - r[1]) as i64;
    order.sort_by(|&a, &b| area(rect(b)).cmp(&area(rect(a))).then(a.cmp(&b)));
    let mut rects: Vec<[i32; 4]> = Vec::new();
    let mut of = vec![usize::MAX; n];
    for i in order {
        let r = rect(i);
        match rects.iter().position(|q| q[0] <= r[0] && q[1] <= r[1] && q[2] >= r[2] && q[3] >= r[3]) {
            Some(k) => of[i] = k,
            None => {
                of[i] = rects.len();
                rects.push(r);
            }
        }
    }
    (rects, of)
}

/// Finds a place for every rectangle at `scale` in the grid `occ`, first fit, top to bottom, biggest first.
fn place(rects: &[[i32; 4]], scale: f64, occ: &[bool], w: usize, h: usize) -> Option<Vec<(i32, i32)>> {
    let mut occ = occ.to_vec();
    let mut out = Vec::with_capacity(rects.len());
    for r in rects {
        let rw = (((r[2] - r[0]) as f64 * scale).ceil() as usize).max(1);
        let rh = (((r[3] - r[1]) as f64 * scale).ceil() as usize).max(1);
        let sat = Sat::new(&occ, w, h);
        let mut found = None;
        'scan: for y in 0..h.saturating_sub(rh) + 1 {
            for x in 0..w.saturating_sub(rw) + 1 {
                if sat.free(x, y, rw, rh) {
                    found = Some((x, y));
                    break 'scan;
                }
            }
        }
        let (x, y) = found?;
        for yy in y..y + rh {
            for xx in x..x + rw {
                occ[yy * w + xx] = true;
            }
        }
        out.push((x as i32, y as i32));
    }
    Some(out)
}

fn texel_wrap(img: &Image, x: i64, y: i64) -> [f64; 4] {
    let p = img.px[(y.rem_euclid(img.h as i64) as usize) * img.w + x.rem_euclid(img.w as i64) as usize];
    p.map(|c| c as f64)
}

fn sample(img: &Image, x: f64, y: f64) -> [u8; 4] {
    let (fx, fy) = (x - 0.5, y - 0.5);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (ax, ay) = (fx - x0, fy - y0);
    let (x0, y0) = (x0 as i64, y0 as i64);
    let (a, b, c, d) = (texel_wrap(img, x0, y0), texel_wrap(img, x0 + 1, y0), texel_wrap(img, x0, y0 + 1), texel_wrap(img, x0 + 1, y0 + 1));
    core::array::from_fn(|k| {
        let top = a[k] * (1.0 - ax) + b[k] * ax;
        let bot = c[k] * (1.0 - ax) + d[k] * ax;
        (top * (1.0 - ay) + bot * ay).round().clamp(0.0, 255.0) as u8
    })
}

/// The gutter colour of Re:CoM's atlas: the commonest colour away from every triangle.
fn gutter(old: &Image, covered: &[bool]) -> [u8; 4] {
    let mut hist: BTreeMap<[u8; 3], u32> = BTreeMap::new();
    for (p, &c) in old.px.iter().zip(covered) {
        if !c {
            *hist.entry([p[0], p[1], p[2]]).or_default() += 1;
        }
    }
    hist.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).map_or([0x3B, 0x37, 0x44, 255], |(c, _)| [c[0], c[1], c[2], 255])
}

/// The new atlas, and where each of DDD's hair islands went: `(image, per island (rectangle, scale))`.
struct Atlas {
    img: Image,
    rects: Vec<Rect>,
    of_island: Vec<usize>,
    scale: f64,
}

fn build_atlas(old: &Image, ddd_img: &Image, hair: &SrcHair, keep_uv: &[[[f64; 2]; 3]], all_uv: &[[[f64; 2]; 3]]) -> Result<Atlas> {
    let (w, h) = (old.w, old.h);
    if (ddd_img.w, ddd_img.h) != (w, h) {
        return bad(format!("hair: DDD's atlas is {}x{}, Re:CoM's {w}x{h}", ddd_img.w, ddd_img.h));
    }
    let foot = |tris: &[[[f64; 2]; 3]], slack: f64| -> Vec<bool> {
        let mut m = vec![false; w * h];
        for t in tris {
            let px = t.map(|p| [p[0] * w as f64, p[1] * h as f64]);
            raster(&mut m, w, h, px, slack);
            for p in px {
                let (x, y) = ((p[0].floor().max(0.0) as usize).min(w - 1), (p[1].floor().max(0.0) as usize).min(h - 1));
                m[y * w + x] = true;
            }
        }
        m
    };
    let keep = dilate(&foot(keep_uv, 0.75), w, h, KEEP_DILATE);
    let covered = dilate(&foot(all_uv, 0.75), w, h, 10);
    let bg = gutter(old, &covered);
    let (src_rects, of_island) = island_rects(hair, (w, h));
    let mut chosen = None;
    for s in SCALES {
        if let Some(p) = place(&src_rects, s, &keep, w, h) {
            chosen = Some((s, p));
            break;
        }
    }
    let Some((scale, dst)) = chosen else { return bad("hair: DDD's hair islands do not fit in the free space of Re:CoM's atlas") };
    let mut img = Image { w, h, px: old.px.iter().zip(&keep).map(|(p, &k)| if k { *p } else { bg }).collect() };
    let mut rects = Vec::new();
    for (r, &(dx, dy)) in src_rects.iter().zip(&dst) {
        let rect = Rect { sx: r[0], sy: r[1], sw: r[2] - r[0], sh: r[3] - r[1], dx, dy };
        let (rw, rh) = (((rect.sw as f64 * scale).ceil()) as i32, ((rect.sh as f64 * scale).ceil()) as i32);
        for y in 0..rh {
            for x in 0..rw {
                // Below 0.5 a destination pixel covers 2 x 2 or more source pixels: average a k x k grid of samples across it
                // (k = 1 above 0.5, so the DDD atlas, which fits at about 1, is sampled as it always was).
                let k = ((1.0 / scale).floor() as usize).max(1);
                let mut acc = [0.0f64; 3];
                for j in 0..k {
                    for i in 0..k {
                        let (sx, sy) = (rect.sx as f64 + (x as f64 + (i as f64 + 0.5) / k as f64) / scale, rect.sy as f64 + (y as f64 + (j as f64 + 0.5) / k as f64) / scale);
                        let q = sample(ddd_img, sx, sy);
                        for c in 0..3 {
                            acc[c] += q[c] as f64;
                        }
                    }
                }
                let n = (k * k) as f64;
                let p = [(acc[0] / n).round() as u8, (acc[1] / n).round() as u8, (acc[2] / n).round() as u8, 255];
                img.px[(dy + y) as usize * w + (dx + x) as usize] = p;
            }
        }
        rects.push(rect);
    }
    Ok(Atlas { img, rects, of_island, scale })
}

/// A DDS of `img` with `like`'s header (A8R8G8B8, no mipmaps, same size).
fn encode_dds(like: &[u8], img: &Image) -> Result<Vec<u8>> {
    let u = |o: usize| u32::from_le_bytes(like[o..o + 4].try_into().unwrap());
    if like.len() < 128 || &like[..4] != b"DDS " || u(12) as usize != img.h || u(16) as usize != img.w || u(88) != 32 || [u(92), u(96), u(100), u(104)] != [0xFF_0000, 0xFF00, 0xFF, 0xFF00_0000] || u(80) & 0x41 != 0x41 {
        return bad("hair: the atlas's DDS is not 32-bit A8R8G8B8 of the expected size");
    }
    let mut out = like[..128].to_vec();
    out.reserve(4 * img.px.len());
    for p in &img.px {
        out.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
    }
    Ok(out)
}

/// A DDS of `img`: A8R8G8B8, no mipmaps (as Re:CoM's own 32-bit HD textures, e.g. `n_zz110_01`).
pub fn dds_argb(img: &Image) -> Vec<u8> {
    let mut h = vec![0u8; 128];
    let mut put = |o: usize, v: u32| h[o..o + 4].copy_from_slice(&v.to_le_bytes());
    put(0, u32::from_le_bytes(*b"DDS "));
    put(4, 124);
    put(8, 0x100F); // caps, height, width, pitch, pixel format
    put(12, img.h as u32);
    put(16, img.w as u32);
    put(20, 4 * img.w as u32);
    put(76, 32);
    put(80, 0x41); // RGB, alpha pixels
    put(88, 32);
    put(92, 0xFF_0000);
    put(96, 0xFF00);
    put(100, 0xFF);
    put(104, 0xFF00_0000);
    put(108, 0x1000); // texture
    let mut out = h;
    out.reserve(4 * img.px.len());
    for p in &img.px {
        out.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The swap.
// ---------------------------------------------------------------------------------------------

fn part_bones(p: &MdlPart) -> Vec<String> {
    p.bones.iter().map(|b| cstr(&b.name)).collect()
}

/// A texel of Re:CoM's painted hair (grey-lavender, as [`crate::bbs_hair::is_hair_texel`]'s rule would see it, or the atlas's dark
/// gutter): not skin. Skin is orange-brown: red over green over blue, blue well under red.
fn not_skin(p: [u8; 4]) -> bool {
    let (r, g, b) = (p[0] as f64, p[1] as f64, p[2] as f64);
    !(r >= g && g >= b && r > 40.0 && (r - b) > 0.3 * r)
}

/// The atlas's texel at `uv` (wrapped, nearest).
fn texel_at(img: &Image, uv: [f64; 2]) -> [u8; 4] {
    let x = ((uv[0].rem_euclid(1.0) * img.w as f64) as usize).min(img.w - 1);
    let y = ((uv[1].rem_euclid(1.0) * img.h as f64) as usize).min(img.h - 1);
    img.px[y * img.w + x]
}

/// Samples of a UV triangle (barycentric grid, edges included) and the texels one pixel around each.
fn uv_samples(uv: [[f64; 2]; 3], img: &Image) -> Vec<[f64; 2]> {
    let (px, py) = (1.0 / img.w as f64, 1.0 / img.h as f64);
    let mut out = vec![];
    const N: usize = 10;
    for i in 0..=N {
        for j in 0..=(N - i) {
            let (a, b) = (i as f64 / N as f64, j as f64 / N as f64);
            let c = 1.0 - a - b;
            let p = [a * uv[0][0] + b * uv[1][0] + c * uv[2][0], a * uv[0][1] + b * uv[1][1] + c * uv[2][1]];
            for (dx, dy) in [(0.0, 0.0), (-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                out.push([p[0] + dx * px, p[1] + dy * py]);
            }
        }
    }
    out
}

/// A texel of the painted lock: grey-lavender (saturation under 0.3) and not dark (the ear's outline and the atlas's gutter are).
fn painted_hair(p: [u8; 4]) -> bool {
    let (mx, mn) = (p[0].max(p[1]).max(p[2]) as f64, p[0].min(p[1]).min(p[2]) as f64);
    mx >= 60.0 && (mx - mn) / mx < 0.3
}

/// Texels around the painted lock (its soft edge and the shadow painted along it) filled with it (0.198.13).
const LOCK_EDGE: usize = 8;
/// A skin texel is a source of the lock's fill when its red channel is this part of the patch's median skin red or more (0.198.13; on
/// `PL0201` the cheek beside the lock is 150-185 and the shadow painted at the jaw over the collar under 90; the Halloween attires'
/// skin is darker as a whole, Checked (files: the patches' dumps)).
const FILL_FROM: f64 = 0.65;

/// 0.198.13 ([`HairSource::painted_temple_window`]): the kept head triangles in the window (both sides) that show the painted lock are
/// given a clean copy of their own skin. Re:CoM's atlas cannot change (its name must stay, and the game looks names up globally), so
/// each group of them that shares UV corners gets its rectangle of the atlas copied into a free place of the hair's own picture `pic`
/// (no hair triangle's texels there), the lock's texels (and [`LOCK_EDGE`] around them) in the copy filled from the skin texels around them (repeated means of the
/// skin neighbours: the skin's own gradient goes on through), and its triangles point there (texture `pic_tex`). Every texel outside
/// the lock is Re:CoM's own, so the edges with the cheek and the ear do not change. Returns the triangles moved and the lock texels
/// filled.
fn unpaint_temples(tris: &mut [RTri], img: &Image, atlas: u32, head: usize, world: &[Aff], win: [f64; 6], pic: &mut Image, pic_tex: u32, hair_uv: &[[[f64; 2]; 3]]) -> (usize, usize) {
    let uv_of = |t: &RTri| t.v.map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64]));
    let picked: Vec<usize> = (0..tris.len())
        .filter(|&i| {
            let t = &tris[i];
            if t.tex != atlas || !t.v.iter().all(|v| v.bone() == head) {
                return false;
            }
            let w = t.v.map(|v| world[head].point(v.pos().map(|x| x as f64)));
            let c = [0, 1, 2].map(|k| (w[0][k] + w[1][k] + w[2][k]) / 3.0 - world[head].t[k]);
            (win[0]..=win[1]).contains(&c[0].abs()) && (win[2]..=win[3]).contains(&c[1]) && (win[4]..=win[5]).contains(&c[2])
                && uv_samples(uv_of(t), img).into_iter().any(|p| painted_hair(texel_at(img, p)))
        })
        .collect();
    if picked.is_empty() {
        return (0, 0);
    }
    // where the hair's triangles read `pic` (wrapped as the hair's UVs are), grown by a margin: not free
    let (pw, ph) = (pic.w, pic.h);
    let mut used = vec![false; pw * ph];
    for t in hair_uv {
        let (fu, fv) = (((t[0][0] + t[1][0] + t[2][0]) / 3.0).floor(), ((t[0][1] + t[1][1] + t[2][1]) / 3.0).floor());
        raster(&mut used, pw, ph, t.map(|p| [(p[0] - fu) * pw as f64, (p[1] - fv) * ph as f64]), 1.0);
    }
    let used = dilate(&used, pw, ph, 8);
    let free = Sat::new(&used, pw, ph);
    let corners: Vec<[(i64, i64); 3]> = picked.iter().map(|&i| tris[i].v.each_ref().map(uv_key)).collect();
    let (group, n) = islands(&corners);
    let (mut moved, mut filled) = (0, 0);
    for g in 0..n {
        let members: Vec<usize> = picked.iter().zip(&group).filter(|(_, &k)| k == g).map(|(&i, _)| i).collect();
        // the group's rectangle in the atlas's pixels, with a margin for the filtering
        let pad = 4i64;
        let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        for &i in &members {
            for p in uv_of(&tris[i]) {
                let (x, y) = ((p[0] * img.w as f64).floor() as i64, (p[1] * img.h as f64).floor() as i64);
                (x0, y0, x1, y1) = (x0.min(x - pad), y0.min(y - pad), x1.max(x + pad + 1), y1.max(y + pad + 1));
            }
        }
        let (rw, rh) = ((x1 - x0) as usize, (y1 - y0) as usize);
        // the first free place of `pic`, scanning rows from the bottom (the kid's body and clothes, which the hair does not use)
        let mut at = None;
        'scan: for y in (0..ph.saturating_sub(rh)).rev().step_by(4) {
            for x in (0..pw.saturating_sub(rw)).step_by(4) {
                if free.free(x, y, rw, rh) {
                    at = Some((x, y));
                    break 'scan;
                }
            }
        }
        let Some((ox, oy)) = at else { continue };
        let src_px = |x: i64, y: i64| img.px[(y.rem_euclid(img.h as i64) as usize) * img.w + x.rem_euclid(img.w as i64) as usize];
        let mut patch: Vec<[u8; 4]> = (0..rw * rh).map(|i| src_px(x0 + (i % rw) as i64, y0 + (i / rw) as i64)).collect();
        // fill the lock: its texels take the mean of their skin (or already filled) neighbours, pass after pass
        // (with its soft edge and the shadow painted beside it: the texels within [`LOCK_EDGE`] of it go too)
        // (the dark texels stay, but those in the lock's own shading, within 2 of it: Re:CoM paints the collar's shadow on the face's lower triangles, model-reviewer-style check of the
        // first try: filled, it showed as a skin sliver over the collar)
        // (black: Dark Riku's collar is painted 0-25 on the jaw's triangles; the gutter, 32-56, and the Halloween lock's dark shading are
        // filled, Checked (files: the patches' dumps))
        let dark = |p: [u8; 4]| p[0].max(p[1]).max(p[2]) < 28;
        let grey = |p: [u8; 4]| { let (mx, mn) = (p[0].max(p[1]).max(p[2]) as f64, p[0].min(p[1]).min(p[2]) as f64); mx >= 28.0 && (mx - mn) / mx < 0.3 };
        let lock: Vec<bool> = patch.iter().map(|&p| grey(p)).collect();
        let near = dilate(&lock, rw, rh, 2);
        let mut todo: Vec<bool> = dilate(&lock, rw, rh, LOCK_EDGE).into_iter().zip(&patch).zip(&near).map(|((t, &p), &n)| t && (n || !dark(p))).collect();
        // sources: lit skin only (the collar's shadow painted on the jaw, filled from, grew a dark brown blob at the lock's tip)
        let lit = {
            let mut r: Vec<u8> = patch.iter().zip(&todo).filter(|(&p, &t)| !t && !not_skin(p)).map(|(p, _)| p[0]).collect();
            r.sort_unstable();
            r.get(r.len() / 2).map_or(0.0, |&m| m as f64 * FILL_FROM)
        };
        let mut good: Vec<bool> = patch.iter().zip(&todo).map(|(&p, &t)| !t && !not_skin(p) && p[0] as f64 >= lit).collect();
        filled += todo.iter().filter(|&&t| t).count();
        for _ in 0..rw.max(rh) {
            if !todo.iter().any(|&t| t) {
                break;
            }
            let mut next = patch.clone();
            let mut done = vec![];
            for i in (0..rw * rh).filter(|&i| todo[i]) {
                let (x, y) = ((i % rw) as i64, (i / rw) as i64);
                let (mut sum, mut k) = ([0.0f64; 3], 0.0);
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= rw as i64 || ny >= rh as i64 {
                        continue;
                    }
                    let j = ny as usize * rw + nx as usize;
                    if good[j] {
                        for c in 0..3 {
                            sum[c] += patch[j][c] as f64;
                        }
                        k += 1.0;
                    }
                }
                if k > 0.0 {
                    next[i] = [(sum[0] / k).round() as u8, (sum[1] / k).round() as u8, (sum[2] / k).round() as u8, 255];
                    done.push(i);
                }
            }
            for i in done {
                todo[i] = false;
                good[i] = true;
            }
            patch = next;
        }
        // debug (tests only): `SB_PATCH_DEBUG=<dir>` writes each patch before / after the fill as raw RGB
        if let Some(d) = std::env::var("SB_PATCH_DEBUG").ok().filter(|_| cfg!(test)) {
            let orig: Vec<u8> = (0..rw * rh).flat_map(|i| { let p = src_px(x0 + (i % rw) as i64, y0 + (i / rw) as i64); [p[0], p[1], p[2]] }).collect();
            let now: Vec<u8> = patch.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
            let _ = std::fs::write(format!("{d}/patch_{}x{}_{g}_orig.rgb", rw, rh), orig);
            let _ = std::fs::write(format!("{d}/patch_{}x{}_{g}_now.rgb", rw, rh), now);
        }
        for y in 0..rh {
            for x in 0..rw {
                pic.px[(oy + y) * pw + ox + x] = patch[y * rw + x];
            }
        }
        // the triangles read the copy: the same texel, now in `pic`
        for &i in &members {
            for v in tris[i].v.iter_mut() {
                if let Some(q) = v.q2.as_mut() {
                    let (u, w) = (f32::from_bits(q[0]) as f64, f32::from_bits(q[1]) as f64);
                    let (x, y) = (u * img.w as f64 - x0 as f64 + ox as f64, w * img.h as f64 - y0 as f64 + oy as f64);
                    q[0] = ((x / pw as f64) as f32).to_bits();
                    q[1] = ((y / ph as f64) as f32).to_bits();
                    q[3] = pic_tex;
                }
            }
            tris[i].tex = pic_tex;
        }
        moved += members.len();
    }
    (moved, filled)
}

/// Riku's `.MDL` with a new hair (see the module docs): `mdl` is `PLAYER/0101/PL0101.MDL`, `rtm` its raw `.RTM` (the data part of
/// the archive entry) and `hd` that entry's HD assets. The caller writes the result as `build_blade` does
/// (`egs::build_plain_entry(&swap.rtm, &swap.hd, date)`).
pub fn with_hair(mdl: &[u8], rtm: &[u8], hd: &[HdAsset], target: &HairTarget, src: &dyn HairSource) -> Result<HairSwap> {
    let mut parts = split_parts(mdl)?;
    if parts.len() != 4 {
        return bad(format!("hair: the model has {} parts, not 4", parts.len()));
    }
    let p2 = MdlPart::parse_bytes(&parts[2])?;
    let p3 = MdlPart::parse_bytes(&parts[3])?;
    let skel = RecomSkeleton::parse(mdl)?;
    let names = part_bones(&p3);
    if names != part_bones(&p2) || names != skel.bones.iter().map(|b| b.name.clone()).collect::<Vec<_>>() {
        return bad("hair: the parts do not share one skeleton");
    }
    // 0.198.7: part 3's list B is kept as it is when it has no hair (Dark Riku's `PL0201`: the coat tails, `*_skt*` / `hip` / `waist`
    // on `n_zz120_02`, Checked (files)); part 2's must be empty
    let names_b: Vec<String> = skel.bones.iter().map(|b| b.name.clone()).collect();
    if p3.list_b.as_ref().is_some_and(|l| l.iter().any(|pk| pk.verts.iter().any(|v| names_b.get(v.bone()).is_some_and(|n| n.contains("hair") || n == HEAD_BONE)))) || p2.list_b.is_some() {
        return bad("hair: the body's list B has hair or head triangles, or the proxy has a list B");
    }
    if !p3.list_a.iter().all(|p| p.fmt == FMT_TEXTURED) || !p2.list_a.iter().all(|p| p.fmt == FMT_PLAIN) {
        return bad("hair: part 3 is not textured or part 2 is not plain");
    }
    let atlas_tex = p3.texture_names().iter().position(|n| n == target.texture).ok_or_else(|| Error::Format(format!("hair: part 3 has no texture {}", target.texture)))? as u32;
    let head = names.iter().position(|n| n == HEAD_BONE).ok_or_else(|| Error::Format("hair: no head bone".into()))?;
    let hair_bones: HashSet<usize> = names.iter().enumerate().filter(|(_, n)| n.contains("hair")).map(|(i, _)| i).collect();
    let world = skel.rest_world();

    // Re:CoM's triangles, part 3 then its plain twin.
    let tris3 = extract(&p3.list_a)?;
    let tris2 = extract(&p2.list_a)?;
    let (mut remove3, split) = classify_riku(&tris3, atlas_tex, head, &hair_bones);
    let mut also_removed = 0usize;
    // 0.198.7: the second hair texture ([`HairTarget::also`]): its triangles in hair islands that touch the head or a hair bone go too
    // (as on the atlas; Dark Riku's has hair pieces on `head` there: left, they showed as dark spikes above the ears, render `kh2_cam`)
    if let Some(also) = target.also {
        let t2 = p3.texture_names().iter().position(|n| n == also).ok_or_else(|| Error::Format(format!("hair: part 3 has no texture {also}")))? as u32;
        let (mut r2, s2) = classify_riku(&tris3, t2, head, &hair_bones);
        also_removed = s2.removed;
        // the base of the old hair over the nape: whole triangles on `head` / `neck*` within [`ALSO_NAPE`] of the head bone (Dark Riku's
        // 8, Checked (files, test `dark_probe`): |x| ≤ 2.8 cm, 0.8-4.5 cm under the head bone, 6-9 cm behind it; model-reviewer of
        // 0.198.7: a flat grey dome between the new hair and the collar)
        for (t, r) in tris3.iter().zip(r2.iter_mut()) {
            if *r || t.tex != t2 || !t.v.iter().all(|v| v.bone() == head || names[v.bone()].starts_with("neck")) {
                continue;
            }
            let w = t.v.map(|v| world[v.bone()].point(v.pos().map(|x| x as f64)));
            let c = [0, 1, 2].map(|k| (w[0][k] + w[1][k] + w[2][k]) / 3.0 - world[head].t[k]);
            if c[0].abs() <= ALSO_NAPE[0] && (ALSO_NAPE[1]..=ALSO_NAPE[2]).contains(&c[1]) && c[2] <= ALSO_NAPE[3] {
                *r = true;
                also_removed += 1;
            }
        }
        for (r, x) in remove3.iter_mut().zip(r2) {
            *r |= x;
        }
    }
    // Part 2 is not part 3's twin (1074 triangles, other positions): a low-poly head / hair proxy. Its hair is the
    // triangles that touch a hair bone; its head-bone triangles (the skull) stay.
    let remove2: Vec<bool> = tris2
        .iter()
        .map(|t| {
            if t.v.iter().any(|v| hair_bones.contains(&v.bone())) {
                return true;
            }
            // The skull stays, except the head-bone triangles the source asks to drop (0.196.4).
            t.v.iter().all(|v| v.bone() == head) && {
                let w = t.v.map(|v| world[v.bone()].point(v.pos().map(|x| x as f64)));
                let c = [0, 1, 2].map(|k| (w[0][k] + w[1][k] + w[2][k]) / 3.0);
                src.drop_proxy_triangle(c, world[head].t)
            }
        })
        .collect();
    let proxy_dropped = tris2.iter().zip(&remove2).filter(|(t, &r)| r && t.v.iter().all(|v| v.bone() == head)).count();
    let removed2 = remove2.iter().filter(|&&r| r).count();

    // The face that stays (head-bone triangles not removed), for sources that fit to it.
    let face: Vec<[[f64; 3]; 3]> = tris3
        .iter()
        .zip(&remove3)
        .filter(|(t, &r)| !r && t.v.iter().all(|v| v.bone() == head))
        .map(|(t, _)| t.v.map(|v| src.adjust_face(world[v.bone()].point(v.pos().map(|x| x as f64)), world[head].t)))
        .collect();
    src.set_face(&face);
    // The source's hair, in Re:CoM's frame.
    let hair = src.hair(&names, &world)?;

    // The atlas.
    let old_asset = hd.iter().find(|a| a.name == crate::keyblade::hd_name(target.texture)).ok_or_else(|| Error::Format(format!("hair: no HD asset for {}", target.texture)))?;
    let old_img = decode_dds(&old_asset.data)?;
    let uv_of = |t: &RTri| t.v.map(|v| v.uv().map_or([0.0; 2], |u| [u[0] as f64, u[1] as f64]));
    let keep_uv: Vec<_> = tris3.iter().zip(&remove3).filter(|(t, &r)| !r && t.tex == atlas_tex).map(|(t, _)| uv_of(t)).collect();
    let all_uv: Vec<_> = tris3.iter().filter(|t| t.tex == atlas_tex).map(uv_of).collect();
    // 0.197.0: the hair on its own texture (`target.new_texture`, the source's picture at its own size), Riku's atlas left as it is
    // (its name too: his `.TXA` animates the eyes and lips on `n_zz110_01.tm2` by name, Checked (files)); a source may still ask
    // for the old packing into Re:CoM's atlas ([`HairSource::packed_atlas`]).
    let atlas = if src.packed_atlas() { Some(build_atlas(&old_img, &hair.atlas, &hair, &keep_uv, &all_uv)?) } else { None };
    let hair_tex = if atlas.is_some() { atlas_tex } else { p3.textures.len() as u32 };

    // The source's triangles as Re:CoM vertices.
    let inv: Vec<[[f64; 3]; 3]> = world.iter().map(|w| inv3(&w.r)).collect::<Result<_>>()?;
    let centre = {
        let h = world[head].t;
        [h[0] + HEAD_CENTRE[0], h[1] + HEAD_CENTRE[1], h[2] + HEAD_CENTRE[2]]
    };
    // One-sided sources: the winding most triangles have away from the head's centre is the source's front.
    let one_sided_flag = hair.one_sided.then(|| {
        let out = hair.tris.iter().filter(|t| facing(t.v.clone().map(|v| v.pos), centre) == 0x20).count();
        if out * 2 >= hair.tris.len() { 0x20 } else { 0 }
    });
    let mut ddd_segs: Vec<Seg> = Vec::new();
    for t in &hair.tris {
        let mk = |v: &SrcVert, draw_facing: Option<u32>| -> Result<MeshVertex> {
            let b = names.iter().position(|n| *n == v.bone).ok_or_else(|| Error::Format(format!("hair: Re:CoM has no bone {}", v.bone)))?;
            let p = to_local(&world[b], &inv[b], v.pos);
            let (u, w) = match &atlas {
                Some(atlas) => {
                    let rect = atlas.rects[atlas.of_island[t.island]];
                    let (iw, ih) = (atlas.img.w as f64, atlas.img.h as f64);
                    ((rect.dx as f64 + (v.uv[0] * iw - rect.sx as f64) * atlas.scale) / iw, (rect.dy as f64 + (v.uv[1] * ih - rect.sy as f64) * atlas.scale) / ih)
                }
                None => (v.uv[0], v.uv[1]),
            };
            let q0 = match draw_facing {
                None => START_Q0,
                Some(f) => [f, F32_ONE, 0, 0x2000],
            };
            let bits = |x: f64| (x as f32).to_bits();
            Ok(MeshVertex { q0, q1: [bits(p[0]), bits(p[1]), bits(p[2]), ((b as u32) << 16) | (b as u32 * 4)], q2: Some([(u as f32).to_bits(), (w as f32).to_bits(), F32_ONE, hair_tex]) })
        };
        let f = match one_sided_flag {
            Some(f) => f,
            None => facing(t.v.clone().map(|v| v.pos), centre),
        };
        if t.cont && !ddd_segs.is_empty() {
            ddd_segs.last_mut().unwrap().verts.push(mk(&t.v[2], Some(f))?);
        } else {
            ddd_segs.push(Seg { tex: hair_tex, verts: vec![mk(&t.v[0], None)?, mk(&t.v[1], None)?, mk(&t.v[2], Some(f))?] });
        }
    }

    // The lists.
    // The face that stays, as the source wants it (head-bone vertices moved in world space, back to their bone's frame).
    let mut adjusted3: Vec<RTri> = tris3
        .iter()
        .zip(&remove3)
        .filter(|(_, &r)| !r)
        .map(|(t, _)| {
            let mut t = t.clone();
            for v in t.v.iter_mut() {
                let b = v.bone();
                if b != head {
                    continue;
                }
                let w0 = world[b].point(v.pos().map(|x| x as f64));
                let w1 = src.adjust_face(w0, world[head].t);
                if w1 != w0 {
                    let l = to_local(&world[b], &inv3(&world[b].r).unwrap_or(crate::math::I3), w1);
                    v.q1 = [(l[0] as f32).to_bits(), (l[1] as f32).to_bits(), (l[2] as f32).to_bits(), v.q1[3]];
                }
            }
            t
        })
        .collect();
    // 0.198.13: the temples' painted lock, on the hair's own picture (not with a packed atlas: the hair is on Re:CoM's there)
    let mut pic = hair.atlas.clone();
    let temple_unpainted = match src.painted_temple_window() {
        Some(win) if atlas.is_none() => {
            let hair_uv: Vec<[[f64; 2]; 3]> = hair.tris.iter().map(|t| t.v.clone().map(|v| v.uv)).collect();
            unpaint_temples(&mut adjusted3, &old_img, atlas_tex, head, &world, win, &mut pic, hair_tex, &hair_uv)
        }
        _ => (0, 0),
    };
    let kept3: Vec<&RTri> = adjusted3.iter().collect();
    let kept2: Vec<&RTri> = tris2.iter().zip(&remove2).filter(|(_, &r)| !r).map(|(t, _)| t).collect();
    let nb = p3.bones.len();
    let (g3, r3) = p3.list_a.first().map_or((0, 0), |p| (p.gif, p.regs));
    let (g2, r2) = p2.list_a.first().map_or((0, 0), |p| (p.gif, p.regs));
    let mut segs3 = segments(&kept3);
    // The kept face's triangles the source asks to be two-sided: the same vertices again with the other facing flag (the flag
    // is the third corner's, as for the hair's `back` copies below).
    let face_back: Vec<Seg> = kept3
        .iter()
        .filter(|t| {
            t.v.iter().all(|v| v.bone() == head) && {
                let w = t.v.map(|v| world[head].point(v.pos().map(|x| x as f64)));
                let c = [0, 1, 2].map(|k| (w[0][k] + w[1][k] + w[2][k]) / 3.0);
                src.two_sided_face(c, world[head].t)
            }
        })
        .map(|t| Seg { tex: t.tex, verts: vec![start_of(t.v[0]), start_of(t.v[1]), MeshVertex { q0: [t.v[2].q0[0] ^ 0x20, t.v[2].q0[1], t.v[2].q0[2], t.v[2].q0[3]], ..t.v[2] }] })
        .collect();
    let face_two_sided = face_back.len();
    segs3.extend(face_back);
    // The skirt (0.196.15; fork owner: "eu acho que é pq a cabeça base acaba antes da cabeça original e dai vem o cabelo e fica o vão"):
    // the head's open edges (an edge of one kept triangle only) that the source selects are joined to the source's nearest hair vertices by
    // two-sided triangles that wear the hair vertex's texel and follow its bone, so no space is left between where the head's skin ends and
    // where the hair starts. Checked (files): 19 of the 24 side edges of the 0.196.3 head were over 5 mm from any hair triangle, 14 over 10 mm.
    let mut skirt = 0usize;
    {
        let key3 = |p: [f64; 3]| [(p[0] * 1e4).round() as i64, (p[1] * 1e4).round() as i64, (p[2] * 1e4).round() as i64];
        let mut edges: HashMap<[i64; 6], (usize, [f64; 3], [f64; 3], MeshVertex, MeshVertex)> = HashMap::new();
        for t in &kept3 {
            let w = t.v.map(|v| world[v.bone()].point(v.pos().map(|x| x as f64)));
            for (x, y) in [(0usize, 1usize), (1, 2), (2, 0)] {
                let (ka, kb) = (key3(w[x]), key3(w[y]));
                let (lo, hi) = if ka <= kb { (ka, kb) } else { (kb, ka) };
                let e = edges.entry([lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]]).or_insert((0, w[x], w[y], t.v[x], t.v[y]));
                e.0 += 1;
            }
        }
        // the source's hair vertices: world position and the vertex as written
        let hv: Vec<([f64; 3], MeshVertex)> = hair
            .tris
            .iter()
            .zip(&ddd_segs)
            .filter(|(t, s)| !t.cont && s.verts.len() == 3)
            .flat_map(|(t, s)| (0..3).map(move |j| (t.v[j].pos, s.verts[j])))
            .collect();
        let nearest = |p: [f64; 3]| -> Option<(f64, usize)> {
            hv.iter().enumerate().map(|(i, h)| ((0..3).map(|k| (h.0[k] - p[k]).powi(2)).sum::<f64>().sqrt(), i)).min_by(|a, b| a.0.total_cmp(&b.0))
        };
        // a skirt vertex: the head's vertex `v` wearing the texel (and texture) of hair vertex `h`
        let on_hair = |v: MeshVertex, h: &MeshVertex| MeshVertex { q0: START_Q0, q2: match (v.q2, h.q2) { (Some(a), Some(b)) => Some([b[0], b[1], a[2], b[3]]), (a, _) => a }, ..v };
        let mut keys: Vec<_> = edges.keys().copied().collect();
        keys.sort();
        for k in keys {
            let (n, pa, pb, va, vb) = edges[&k];
            let mid = [0, 1, 2].map(|i| (pa[i] + pb[i]) / 2.0);
            // 0.198.33: the neck's open back ([`NECK_SKIRT_Y`])
            let on_neck = |v: &MeshVertex| v.bone() == head || names[v.bone()].starts_with("neck");
            let nape = n == 1 && target.neck_skirt && on_neck(&va) && on_neck(&vb) && in_neck_skirt_window(mid, world[head].t);
            if !nape && (n != 1 || va.bone() != head || vb.bone() != head || !src.skirt_edge(mid, world[head].t)) {
                continue;
            }
            // the nape's: the nearest hair corner not inside the neck (nearer its axis, `neck1`'s, than the edge's corner): a skirt
            // in to the hair's lowest layer, which dips into the neck, came out through the neck's side as the head turned on it
            let axis = names.iter().position(|x| x == "neck1").map_or(world[head].t, |i| world[i].t);
            let radial = |p: [f64; 3]| ((p[0] - axis[0]).powi(2) + (p[2] - axis[2]).powi(2)).sqrt();
            let nearest_out = |p: [f64; 3]| -> Option<(f64, usize)> {
                hv.iter().enumerate().filter(|(_, h)| radial(h.0) >= radial(p) - NECK_SKIRT_IN).map(|(i, h)| (crate::math::dist(h.0, p), i)).min_by(|a, b| a.0.total_cmp(&b.0))
            };
            let (Some((da, ia)), Some((db, ib))) = (if nape { (nearest_out(pa), nearest_out(pb)) } else { (nearest(pa), nearest(pb)) }) else { continue };
            if nape {
                if da.max(db) > NECK_SKIRT_REACH {
                    continue;
                }
            } else {
                if !(0.003..=0.030).contains(&da.max(db)) && !(0.003..=0.030).contains(&da.min(db)) {
                    continue;
                }
                if da > 0.030 || db > 0.030 {
                    continue;
                }
            }
            let area = |p: [[f64; 3]; 3]| -> f64 {
                let (u, v) = ([p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]], [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]]);
                let c = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
                (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt() / 2.0
            };
            let mut tris: Vec<(MeshVertex, MeshVertex, MeshVertex, [[f64; 3]; 3])> = vec![];
            let (ha, hb) = (hv[ia].clone(), hv[ib].clone());
            // 0.198.33 (fork owner, offered the neck skirt in the coat's texture where it webs in front of the neck: "2"): the nape's
            // skirt wears the edge's own texture and texels (the neck's coat, the face's skin behind the ear), its top corners at the hair
            // corners with the texel of the edge corner under them, so where it crosses in front of the neck it reads as the neck
            // 0.198.35 (model-reviewer of 0.198.33: a coat triangle rose ~4 cm into the hair over the middle nape spike and thin dark
            // slivers poked through the hair, the skirt ending on the hair's outer corners): its top corners are tucked
            // [`NECK_SKIRT_TUCK`] in under the hair, toward the head's centre, rigid on `head`
            let centre = [0, 1, 2].map(|k| world[head].t[k] + HEAD_CENTRE[k]);
            let tucked = |hp: [f64; 3]| -> [f64; 3] {
                let d = [centre[0] - hp[0], centre[1] - hp[1], centre[2] - hp[2]];
                let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-9);
                // tests only: `NECK_TUCK=<metres>` (tuning)
                let t = if cfg!(test) { std::env::var("NECK_TUCK").ok().and_then(|x| x.parse().ok()).unwrap_or(NECK_SKIRT_TUCK) } else { NECK_SKIRT_TUCK };
                let k = t.min(l) / l;
                [hp[0] + d[0] * k, hp[1] + d[1] * k, hp[2] + d[2] * k]
            };
            // the bone the top corners follow (tests only: `NECK_TOP=edge` puts each on its edge corner's bone, for tuning)
            let top_on_edge = cfg!(test) && std::env::var("NECK_TOP").is_ok_and(|x| x == "edge");
            let on_edge = |hp: [f64; 3], v: MeshVertex| {
                let nb = if top_on_edge { v.bone() } else { head };
                let q = to_local(&world[nb], &inv[nb], tucked(hp));
                let bits = |x: f64| (x as f32).to_bits();
                MeshVertex { q1: [bits(q[0]), bits(q[1]), bits(q[2]), ((nb as u32) << 16) | (nb as u32 * 4)], ..v }
            };
            let tex = if nape { va.q2.map_or(hair_tex, |q| q[3]) } else { hair_tex };
            if nape {
                tris.push((va, vb, on_edge(hb.0, vb), [pa, pb, tucked(hb.0)]));
                if ia != ib {
                    tris.push((va, on_edge(hb.0, vb), on_edge(ha.0, va), [pa, tucked(hb.0), tucked(ha.0)]));
                }
            } else {
                tris.push((on_hair(va, &hb.1), on_hair(vb, &hb.1), hb.1, [pa, pb, hb.0]));
                if ia != ib {
                    tris.push((on_hair(va, &ha.1), hb.1, ha.1, [pa, hb.0, ha.0]));
                }
            }
            for (a, b, c, p) in tris {
                if area(p) < 1e-7 {
                    continue;
                }
                for f in [0u32, 0x20] {
                    skirt_segs_push(&mut segs3, tex, start_of(a), start_of(b), MeshVertex { q0: [f, F32_ONE, 0, 0x2000], ..c });
                }
                skirt += 1;
            }
        }
    }
    // The nape curtain (0.198.2): open head / neck edges the source selects get a two-sided strip of their own skin straight down to
    // the neck's base, its bottom corners on the source's bone, so the skull's open back is closed when the hair lifts off it.
    let mut nape = 0usize;
    // the hair's vertices (world rest, as written), for a curtain on the hair's texels
    let hv: Vec<([f64; 3], MeshVertex)> = hair
        .tris
        .iter()
        .zip(&ddd_segs)
        .filter(|(t, s)| !t.cont && !t.lining && s.verts.len() == 3)
        .flat_map(|(t, s)| (0..3).map(move |j| (t.v[j].pos, s.verts[j])))
        .collect();
    {
        let key3 = |p: [f64; 3]| [(p[0] * 1e4).round() as i64, (p[1] * 1e4).round() as i64, (p[2] * 1e4).round() as i64];
        let mut edges: HashMap<[i64; 6], (usize, [f64; 3], [f64; 3], MeshVertex, MeshVertex)> = HashMap::new();
        for t in &kept3 {
            if t.tex != atlas_tex || !t.v.iter().all(|v| v.bone() == head || names[v.bone()].starts_with("neck")) {
                continue;
            }
            let w = t.v.map(|v| world[v.bone()].point(v.pos().map(|x| x as f64)));
            for (x, y) in [(0usize, 1usize), (1, 2), (2, 0)] {
                let (ka, kb) = (key3(w[x]), key3(w[y]));
                let (lo, hi) = if ka <= kb { (ka, kb) } else { (kb, ka) };
                let e = edges.entry([lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]]).or_insert((0, w[x], w[y], t.v[x], t.v[y]));
                e.0 += 1;
            }
        }
        let mut keys: Vec<_> = edges.keys().copied().collect();
        keys.sort();
        for k in keys {
            let (n, pa, pb, va, vb) = edges[&k];
            let mid = [0, 1, 2].map(|i| (pa[i] + pb[i]) / 2.0);
            let Some((bottom, bone)) = (if n == 1 { src.nape_curtain(mid, world[head].t) } else { None }) else { continue };
            let Some(nb) = names.iter().position(|x| x == bone) else { continue };
            let tuck = src.nape_curtain_hair();
            let low = |p: [f64; 3], v: MeshVertex| -> MeshVertex {
                let mut p = [p[0], world[head].t[1] + bottom, p[2]];
                if let Some(d) = tuck {
                    let o = world[nb].t;
                    let (dx, dz) = (o[0] - p[0], o[2] - p[2]);
                    let l = (dx * dx + dz * dz).sqrt().max(1e-9);
                    let k = d.min(l) / l;
                    p = [p[0] + dx * k, p[1], p[2] + dz * k];
                }
                let q = to_local(&world[nb], &inv[nb], p);
                let bits = |x: f64| (x as f32).to_bits();
                MeshVertex { q0: START_Q0, q1: [bits(q[0]), bits(q[1]), bits(q[2]), ((nb as u32) << 16) | (nb as u32 * 4)], ..v }
            };
            let (la, lb) = (low(pa, va), low(pb, vb));
            // 0.198.6: on the hair's texel (the nearest hair vertex's), not the skin's
            let (mut va, mut vb, mut la, mut lb, mut tex) = (va, vb, la, lb, atlas_tex);
            if tuck.is_some() {
                let near = |p: [f64; 3]| hv.iter().min_by(|a, b| crate::math::dist(a.0, p).total_cmp(&crate::math::dist(b.0, p))).map(|h| h.1);
                let wear = |v: MeshVertex, h: Option<MeshVertex>| match (v.q2, h.and_then(|h| h.q2)) { (Some(a), Some(b)) => MeshVertex { q2: Some([b[0], b[1], a[2], b[3]]), ..v }, _ => v };
                // one texel for the whole strip (the hair vertex nearest its middle): two texels from different islands stretched
                // the picture between them (model-reviewer of 0.198.6e: a light stripe beside the neck from the front)
                let h = near(mid);
                va = wear(va, h);
                vb = wear(vb, h);
                la = wear(la, h);
                lb = wear(lb, h);
                tex = hair_tex;
            }
            for (a, b, c) in [(va, vb, lb), (va, lb, la)] {
                for f in [0u32, 0x20] {
                    skirt_segs_push(&mut segs3, tex, start_of(a), start_of(b), MeshVertex { q0: [f, F32_ONE, 0, 0x2000], ..c });
                }
            }
            nape += 1;
        }
    }
    let segs2 = segments(&kept2);
    let mut plain_segs = segs2;
    // The source's hair is written into the plain proxy too (one copy, flat, drawn without a facing cull), except the triangles the source
    // asks to leave out (0.196.4). `ddd_segs[i]` is `hair.tris[i]`'s strip while no triangle continues another's strip.
    let mut plain_hair_dropped = 0;
    for (i, s) in ddd_segs.iter().enumerate() {
        if hair.tris.get(i).is_some_and(|t| t.lining && !src.lining_in_plain()) {
            continue;
        }
        if hair.tris.get(i).is_some_and(|t| !t.cont && src.drop_plain_hair_copy(t, world[head].t)) {
            plain_hair_dropped += 1;
            continue;
        }
        plain_segs.push(Seg { tex: 0, verts: s.verts.clone() });
    }
    // Two-sided: DDD's hair is layered shells and thin shards, some of them seen from the side its winding calls the back
    // (the game culls by the facing flag: a one-sided copy left see-through bands from behind and above, Checked (game,
    // 0.186.0)). Each triangle is written again with the other facing, same vertices, so exactly one copy passes the cull
    // from any side. Part 2 (the proxy) keeps one copy.
    let back: Vec<Seg> = ddd_segs
        .iter()
        .map(|s| Seg { tex: s.tex, verts: s.verts.iter().enumerate().map(|(i, v)| if i < 2 { *v } else { MeshVertex { q0: [v.q0[0] ^ 0x20, v.q0[1], v.q0[2], v.q0[3]], ..*v } }).collect() })
        .collect();
    segs3.extend(ddd_segs);
    if !hair.one_sided {
        segs3.extend(back);
    }
    let list3 = pack(&segs3, FMT_TEXTURED, g3, r3, max_vertices(&p3.list_a, 3));
    let list2 = pack(&plain_segs, FMT_PLAIN, g2, r2, max_vertices(&p2.list_a, 2));
    let mut textures = p3.textures.clone();
    if atlas.is_some() {
        textures[atlas_tex as usize][..16].copy_from_slice(&name16(target.new_texture));
    } else {
        let mut n = [0u8; 32];
        n[..16].copy_from_slice(&name16(target.new_texture));
        textures.push(n);
    }
    let (np3, np2) = ([p3.list_a.len(), p2.list_a.len()], [list3.len(), list2.len()]);
    let new3 = {
        let mut lists: Vec<&[MeshPacket]> = vec![&list3];
        if let Some(lb) = p3.list_b.as_ref() {
            lists.push(lb);
        }
        MdlPart { bounds: crate::keyblade::part_bounds(nb, &lists), textures, list_a: list3.clone(), ..p3.clone() }
    };
    let new2 = MdlPart { bounds: crate::keyblade::part_bounds(nb, &[&list2]), list_a: list2, ..p2.clone() };
    parts[2] = new2.write_bytes()?;
    parts[3] = new3.write_bytes()?;

    // The texture.
    let mut pack_rtm = crate::rtm::Rtm::parse(rtm)?;
    let (dds, hd_out) = match &atlas {
        Some(atlas) => {
            let dds = encode_dds(&old_asset.data, &atlas.img)?;
            let (entry, new_hd, _) = crate::keyblade::texture_entry(target.new_texture, &dds)?;
            let old_entry = format!("{}.tm2", target.texture);
            let at = pack_rtm.entries.iter().position(|e| e.name_str() == old_entry).ok_or_else(|| Error::Format(format!("hair: the RTM has no {old_entry}")))?;
            pack_rtm.entries[at] = entry;
            let mut hd_out: Vec<HdAsset> = hd.iter().filter(|a| a.name != old_asset.name && a.name != new_hd.name).cloned().collect();
            hd_out.push(new_hd);
            (dds, hd_out)
        }
        None => {
            // opaque, as the packed atlas was written
            let dds = dds_argb(&Image { w: pic.w, h: pic.h, px: pic.px.iter().map(|p| [p[0], p[1], p[2], 255]).collect() });
            let (entry, new_hd, _) = crate::keyblade::texture_entry(target.new_texture, &dds)?;
            let new_entry = format!("{}.tm2", target.new_texture);
            pack_rtm.entries.retain(|e| e.name_str() != new_entry);
            pack_rtm.entries.push(entry);
            let mut hd_out: Vec<HdAsset> = hd.iter().filter(|a| a.name != new_hd.name).cloned().collect();
            hd_out.push(new_hd);
            (dds, hd_out)
        }
    };

    let report = HairReport {
        riku_on_atlas: split.on_atlas,
        removed: split.removed,
        removed_plain: removed2,
        other_texture: split.other_texture,
        stray_in_hair_islands: split.stray,
        face_two_sided,
        temple_unpainted,
        proxy_dropped,
        skirt,
        nape,
        also_removed,
        plain_hair_dropped,
        ddd_hair: hair.tris.len(),
        ddd_skin: hair.skin,
        ddd_islands: hair.tris.iter().map(|t| t.island).collect::<HashSet<_>>().len(),
        atlas_rects: atlas.as_ref().map_or(0, |a| a.rects.len()),
        atlas_scale: atlas.as_ref().map_or(1.0, |a| a.scale),
        packets: ([np3[0], np3[1]], np2),
    };
    Ok(HairSwap { mdl: join_parts(&parts), rtm: pack_rtm.write(), hd: hd_out, dds, hair_tex, atlas_dds: old_asset.data.clone(), report })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::install;
    use std::path::{Path, PathBuf};

    fn game() -> Option<PathBuf> {
        std::env::var_os("RECOM_GAME").map(Into::into)
    }

    struct Files {
        mdl: Vec<u8>,
    }

    fn files() -> Option<Files> {
        let g = game()?;
        let keys = install::keys(&g).unwrap();
        let arc = install::recom_archive(&g).unwrap();
        let mdl = install::read_recom(&arc, &keys, "PLAYER/0101/PL0101.MDL").unwrap();
        Some(Files { mdl })
    }

    pub(crate) fn out_dir() -> PathBuf {
        let p = PathBuf::from(format!("{}/local/assets/recom/riku-hair/out", crate::kh2_motion::tests::workspace()));
        let _ = std::fs::create_dir_all(&p);
        p
    }

    #[test]
    fn islands_group_by_shared_corners() {
        let (id, n) = islands(&[[1, 2, 3], [3, 4, 5], [7, 8, 9], [9, 10, 11], [20, 21, 22]]);
        assert_eq!((id, n), (vec![0, 0, 1, 1, 2], 3));
    }

    #[test]
    fn pack_keeps_the_cap_and_continues_split_strips() {
        let v = |draw: bool, n: u32| MeshVertex { q0: if draw { [0, F32_ONE, 0, 0x2000] } else { START_Q0 }, q1: [n, 0, 0, 0], q2: None };
        let mut verts = vec![v(false, 0), v(false, 1)];
        verts.extend((2..12).map(|i| v(true, i)));
        let packets = pack(&[Seg { tex: 0, verts }], FMT_PLAIN, 1, 2, 5);
        assert!(packets.iter().all(|p| p.verts.len() <= 5 && !p.verts[0].draws() && !p.verts[1].draws()));
        // every triangle of the original strip is drawn exactly once, in order
        let seen: Vec<[u32; 3]> = packets.iter().flat_map(|p| (2..p.verts.len()).map(move |i| [p.verts[i - 2].q1[0], p.verts[i - 1].q1[0], p.verts[i].q1[0]])).collect();
        assert_eq!(seen, (0..10).map(|i| [i, i + 1, i + 2]).collect::<Vec<_>>());
    }

    #[test]
    fn raster_covers_a_triangle_and_dilate_grows_it() {
        let mut m = vec![false; 16 * 16];
        raster(&mut m, 16, 16, [[2.0, 2.0], [10.0, 2.0], [2.0, 10.0]], 0.0);
        assert!(m[3 * 16 + 3] && !m[12 * 16 + 12]);
        let d = dilate(&m, 16, 16, 2);
        assert!(d.iter().filter(|&&x| x).count() > m.iter().filter(|&&x| x).count());
    }

    /// Re:CoM's PL0101: the classifier removes the hair islands' head / hair-bone triangles and nothing else, parts 2 and 3
    /// the same; the facing rule fits the face triangles.
    #[test]
    fn classifies_rikus_hair() {
        let Some(f) = files() else { return };
        let parts = split_parts(&f.mdl).unwrap();
        let p3 = MdlPart::parse_bytes(&parts[3]).unwrap();
        let names = part_bones(&p3);
        let head = names.iter().position(|n| n == HEAD_BONE).unwrap();
        let hair_bones: HashSet<usize> = names.iter().enumerate().filter(|(_, n)| n.contains("hair")).map(|(i, _)| i).collect();
        let tris = extract(&p3.list_a).unwrap();
        let (remove, s) = classify_riku(&tris, 1, head, &hair_bones);
        eprintln!("Re:CoM part 3: {} triangles, {s:?}", tris.len());
        assert_eq!(tris.len(), 2728);
        assert_eq!((s.on_atlas, s.stray), (1336, 0));
        assert_eq!(s.removed, 366);
        let touching = tris.iter().filter(|t| t.tex == 1 && t.v.iter().any(|v| hair_bones.contains(&v.bone()))).count();
        assert_eq!(touching, 278);
        assert!(tris.iter().zip(&remove).filter(|(t, _)| t.tex == 1 && t.v.iter().any(|v| hair_bones.contains(&v.bone()))).all(|(_, &r)| r), "every hair-bone triangle is removed");
        assert!(tris.iter().zip(&remove).filter(|(t, _)| t.tex != 1).all(|(t, &r)| !r && !t.v.iter().any(|v| hair_bones.contains(&v.bone()))), "no hair bone on the other texture");
        // facing rule on the face triangles (head bone alone)
        let sk = RecomSkeleton::parse(&f.mdl).unwrap();
        let w = sk.rest_world();
        let head_w = w[head].t;
        let centre = [head_w[0] + HEAD_CENTRE[0], head_w[1] + HEAD_CENTRE[1], head_w[2] + HEAD_CENTRE[2]];
        let (mut ok, mut all) = (0, 0);
        for t in tris.iter().filter(|t| t.tex == 1 && t.v.iter().all(|v| v.bone() == head)) {
            let p = t.v.map(|v| w[head].point(v.pos().map(|x| x as f64)));
            all += 1;
            ok += (facing(p, centre) == t.v[2].q0[0]) as usize;
        }
        eprintln!("facing rule: {ok}/{all}");
        assert!(ok * 100 >= all * 95, "{ok}/{all}");
        // the bounds of the original part are what part_bounds computes (so the rebuilt ones can be trusted)
        assert_eq!(p3.bounds, crate::keyblade::part_bounds(p3.bones.len(), &[&p3.list_a]));
        assert_eq!(join_parts(&parts), f.mdl);
    }

    /// Part 3 of a swap and one picture for the debug renders (0.197.0): with the hair on its own texture, Riku's atlas (left) and
    /// the hair's texture (right, scaled to the atlas's height) side by side, every UV moved onto it and the hair's texture index
    /// made the atlas's (1), so the renders that read texture 1 with one picture still show both. A packed swap is returned as it is.
    pub(crate) fn test_view(swap: &HairSwap) -> (MdlPart, Image) {
        let mut p3 = MdlPart::parse_bytes(&split_parts(&swap.mdl).unwrap()[3]).unwrap();
        let hair = decode_dds(&swap.dds).unwrap();
        if swap.hair_tex == 1 {
            return (p3, hair);
        }
        let atlas = decode_dds(&swap.atlas_dds).unwrap();
        let hw = hair.w * atlas.h / hair.h;
        let (w, h) = (atlas.w + hw, atlas.h);
        let mut px = vec![[0u8, 0, 0, 255]; w * h];
        for y in 0..h {
            for x in 0..w {
                px[y * w + x] = if x < atlas.w { atlas.px[y * atlas.w + x] } else { hair.px[(y * hair.h / h) * hair.w + (x - atlas.w) * hair.w / hw] };
            }
        }
        let (fa, fh) = (atlas.w as f32 / w as f32, hw as f32 / w as f32);
        for k in p3.list_a.iter_mut() {
            for v in k.verts.iter_mut() {
                if let Some(q) = v.q2.as_mut() {
                    let u = f32::from_bits(q[0]);
                    if q[3] == swap.hair_tex {
                        q[0] = (fa + u * fh).to_bits();
                        q[3] = 1;
                    } else if q[3] == 1 {
                        q[0] = (u * fa).to_bits();
                    }
                }
            }
        }
        (p3, Image { w, h, px })
    }

    /// Bounds (world, metres) of PL0101's kept head-bone-only skin triangles at z > 0 (debug).
    pub(crate) fn head_front_bounds(mdl: &[u8]) -> ([f64; 3], [f64; 3]) {
        let parts = split_parts(mdl).unwrap();
        let p3 = MdlPart::parse_bytes(&parts[3]).unwrap();
        let w = RecomSkeleton::parse(mdl).unwrap().rest_world();
        let hb = part_bones(&p3).iter().position(|n| n == HEAD_BONE).unwrap();
        let (mut lo, mut hi) = ([9.0f64; 3], [-9.0f64; 3]);
        for t in collect(&p3) {
            if t.tex == 1 && t.v.iter().all(|v| v.bone() == hb) {
                for c in t.v.map(|v| w[hb].point(v.pos().map(|x| x as f64))) {
                    if c[2] > 0.0 {
                        for k in 0..3 {
                            lo[k] = lo[k].min(c[k]);
                            hi[k] = hi[k].max(c[k]);
                        }
                    }
                }
            }
        }
        (lo, hi)
    }

    /// A list's triangles (0.198.8: part 3's list B too).
    pub(crate) fn extract_list(list: &[MeshPacket]) -> Vec<RTri> {
        extract(list).unwrap()
    }

    pub(crate) fn collect(part: &MdlPart) -> Vec<RTri> {
        extract(&part.list_a).unwrap()
    }

    /// Top-down rays (x, z) through the head: the cells inside the outline that no head / hair / neck triangle covers
    /// (two-sided), as `(x, z)` in metres. Diagnostic for the crown hole.
    pub(crate) fn top_holes(p3: &MdlPart, world: &[Aff]) -> Vec<(f64, f64)> {
        let names = part_bones(p3);
        let want = |b: usize| names[b] == "head" || names[b].contains("hair") || names[b].starts_with("neck");
        let tris: Vec<[[f64; 2]; 3]> = collect(p3).into_iter().filter(|t| t.tex == 1 && t.v.iter().all(|v| want(v.bone()))).map(|t| t.v.map(|v| {
            let w = world[v.bone()].point(v.pos().map(|x| x as f64));
            [w[0], w[2]]
        })).collect();
        let step = 0.004;
        let n = 100i64;
        let covered = |u: f64, v: f64| {
            tris.iter().any(|t| {
                let det = (t[1][1] - t[2][1]) * (t[0][0] - t[2][0]) + (t[2][0] - t[1][0]) * (t[0][1] - t[2][1]);
                if det.abs() < 1e-12 {
                    return false;
                }
                let l0 = ((t[1][1] - t[2][1]) * (u - t[2][0]) + (t[2][0] - t[1][0]) * (v - t[2][1])) / det;
                let l1 = ((t[2][1] - t[0][1]) * (u - t[2][0]) + (t[0][0] - t[2][0]) * (v - t[2][1])) / det;
                l0 >= 0.0 && l1 >= 0.0 && l0 + l1 <= 1.0
            })
        };
        let grid: Vec<Vec<bool>> = (-n..n).map(|i| (-n..n).map(|j| covered(i as f64 * step, j as f64 * step)).collect()).collect();
        let mut out = Vec::new();
        for i in 0..2 * n as usize {
            for j in 0..2 * n as usize {
                if grid[i][j] {
                    continue;
                }
                let row_l = grid[i][..j].iter().any(|&c| c);
                let row_r = grid[i][j..].iter().any(|&c| c);
                let col_u = grid[..i].iter().any(|r| r[j]);
                let col_d = grid[i..].iter().any(|r| r[j]);
                if row_l && row_r && col_u && col_d {
                    out.push(((i as i64 - n) as f64 * step, (j as i64 - n) as f64 * step));
                }
            }
        }
        out
    }

    fn names_hair(p: &MdlPart) -> HashSet<usize> {
        part_bones(p).iter().enumerate().filter(|(_, n)| n.contains("hair")).map(|(i, _)| i).collect()
    }

    // ----- debug rendering (tests only) -----

    fn crc32(d: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in d {
            c ^= b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    }

    pub(crate) fn png(w: usize, h: usize, rgb: &[u8]) -> Vec<u8> {
        let mut raw = Vec::with_capacity((3 * w + 1) * h);
        for y in 0..h {
            raw.push(0);
            raw.extend_from_slice(&rgb[3 * w * y..3 * w * (y + 1)]);
        }
        let chunk = |out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]| {
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            let mut body = kind.to_vec();
            body.extend_from_slice(data);
            out.extend_from_slice(&body);
            out.extend_from_slice(&crc32(&body).to_be_bytes());
        };
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&(w as u32).to_be_bytes());
        ihdr.extend_from_slice(&(h as u32).to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &ihdr);
        chunk(&mut out, b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6));
        chunk(&mut out, b"IEND", &[]);
        out
    }

    /// Textured painter's-algorithm render of the head, hair and neck of part 3 (front: x / y, side: z / y).
    pub(crate) fn render(p3: &MdlPart, world: &[Aff], atlas: &Image, dir: &Path) {
        let names = part_bones(p3);
        let want = |b: usize| names[b] == "head" || names[b].contains("hair") || names[b].starts_with("neck");
        let mut tris: Vec<([[f64; 3]; 3], [[f64; 2]; 3], f64)> = Vec::new();
        for t in collect(p3) {
            if t.tex != 1 || !t.v.iter().all(|v| want(v.bone())) {
                continue;
            }
            let p = t.v.map(|v| world[v.bone()].point(v.pos().map(|x| x as f64)));
            let uv = t.v.map(|v| {
                let u = v.uv().unwrap();
                [u[0] as f64, u[1] as f64]
            });
            tris.push((p, uv, if t.v[2].q0[0] == 0x20 { 1.0 } else { -1.0 }));
        }
        // (name, horizontal axis and sign, vertical axis and sign, depth axis and sign: larger = nearer)
        type View = (&'static str, (usize, f64), (usize, f64), (usize, f64));
        let views: [View; 5] = [
            ("front", (0, 1.0), (1, 1.0), (2, 1.0)),
            ("side", (2, 1.0), (1, 1.0), (0, -1.0)),
            ("back", (0, -1.0), (1, 1.0), (2, -1.0)),
            ("top", (0, 1.0), (2, -1.0), (1, 1.0)),
            ("bottom_back", (0, -1.0), (2, 1.0), (1, -1.0)),
        ];
        for (cull, (name, h, v, dp)) in views.into_iter().flat_map(|v| [(false, v), (true, v)]) {
            let tx = |q: &[f64; 3]| [h.1 * q[h.0], v.1 * q[v.0], dp.1 * q[dp.0]];
            let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
            for (p, _, _) in &tris {
                for q in p {
                    let t = tx(q);
                    for k in 0..3 {
                        lo[k] = lo[k].min(t[k]);
                        hi[k] = hi[k].max(t[k]);
                    }
                }
            }
            let size = 1000usize;
            let s = 900.0 / (hi[0] - lo[0]).max(hi[1] - lo[1]);
            let mut rgb = vec![30u8; size * size * 3];
            let mut order: Vec<usize> = (0..tris.len()).collect();
            order.sort_by(|&a, &b| {
                let d = |i: usize| tris[i].0.iter().map(|q| tx(q)[2]).sum::<f64>();
                d(a).partial_cmp(&d(b)).unwrap()
            });
            for i in order {
                let (p, uv, fl) = &tris[i];
                if cull {
                    let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
                    let mut dir = [0.0; 3];
                    dir[dp.0] = dp.1;
                    if fl * dot(n, dir) <= 0.0 {
                        continue;
                    }
                }
                let sc: Vec<[f64; 2]> = p.iter().map(|q| { let t = tx(q); [50.0 + (t[0] - lo[0]) * s, 950.0 - (t[1] - lo[1]) * s] }).collect();
                let (x0, x1) = (sc.iter().map(|q| q[0]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (sc.iter().map(|q| q[0]).fold(f64::MIN, f64::max).ceil() as usize).min(size - 1));
                let (y0, y1) = (sc.iter().map(|q| q[1]).fold(f64::MAX, f64::min).floor().max(0.0) as usize, (sc.iter().map(|q| q[1]).fold(f64::MIN, f64::max).ceil() as usize).min(size - 1));
                let det = (sc[1][1] - sc[2][1]) * (sc[0][0] - sc[2][0]) + (sc[2][0] - sc[1][0]) * (sc[0][1] - sc[2][1]);
                if det.abs() < 1e-9 {
                    continue;
                }
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                        let l0 = ((sc[1][1] - sc[2][1]) * (px - sc[2][0]) + (sc[2][0] - sc[1][0]) * (py - sc[2][1])) / det;
                        let l1 = ((sc[2][1] - sc[0][1]) * (px - sc[2][0]) + (sc[0][0] - sc[2][0]) * (py - sc[2][1])) / det;
                        let l2 = 1.0 - l0 - l1;
                        if l0 < -0.01 || l1 < -0.01 || l2 < -0.01 {
                            continue;
                        }
                        let u = l0 * uv[0][0] + l1 * uv[1][0] + l2 * uv[2][0];
                        let v = l0 * uv[0][1] + l1 * uv[1][1] + l2 * uv[2][1];
                        let c = atlas.px[((v * atlas.h as f64) as usize).min(atlas.h - 1) * atlas.w + ((u * atlas.w as f64) as usize).min(atlas.w - 1)];
                        rgb[3 * (y * size + x)..3 * (y * size + x) + 3].copy_from_slice(&c[..3]);
                    }
                }
            }
            std::fs::write(dir.join(format!("hair_{name}{}.png", if cull { "_cull" } else { "" })), png(size, size, &rgb)).unwrap();
        }
    }
}
