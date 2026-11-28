//! The AI script runtime and the helper routines KH1's enemy scripts share (M11).
//!
//! KH1 runs an enemy's script once per frame from the enemy update `0x1402B7260`, only while no
//! "action" motion plays (actor flag 4, set by a request with bit 24): an attack or a reaction
//! pauses the script, which then carries on after the instruction that yielded (VM `0x1402CC620`:
//! `misc0` yields, `misc8` ends a thread, `misc3` returns; threads `0x1402BCEC0` / `0x1402BD640`,
//! gate `0x140297870`). Checked (decomp). The port keeps that shape with Rust `async` code polled
//! once per frame; the host doesn't poll while its action runs, which is KH1's pause.
//!
//! The helpers are the routines the scripts share. Each names its offset in the Shadow's
//! `ex_2020.bd` and the Soldier's `ex_2010.bd`; "same bytes" means the two copies are byte for
//! byte identical (their branches and calls are relative), Checked (files).

use super::body::Body;
use super::{Engine, Numbers};
use crate::math::{wrap_angle, Vec3};
use crate::rng::Kh1Rand;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

/// What the AI asks of the host this frame. `A` names an attack (the enemy's own enum; the host
/// sees its index).
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd<A> {
    /// A plain motion request (KH1's `0x14029F7C0` / `0x14029F980` without bit 24): played now
    /// unless it is the one playing.
    Motion { request: u16, blend: f32 },
    /// The motion queued after the current one (`0x14029FA30`).
    Queue { request: u16, blend: f32 },
    /// [`Cmd::Queue`] played from `start` KH1 frames when it comes (`sys 14`'s fourth word: `0x1402B9FB0` writes it to the
    /// queued entry's start `+4`, as `sys 13`'s; Checked (decomp)). M11, the Yellow Opera's Thunder (`ex_2130.bd`
    /// `0x4A8..0x4E4`: 206 queued from frame 2, 207 from frame 3). KH1's request list holds several entries
    /// (`0x14029FA30` appends, `+0x5E` counts); the host's one queued slot takes the next when the behaviour sees the
    /// previous start.
    QueueAt { request: u16, blend: f32, start: f32 },
    /// An action (bit 24): the AI is not polled again until the host says it ended. `attack`:
    /// the attack objects made with it. `then`: the queued request.
    Action { request: u16, attack: Option<A>, then: u16, blend: f32 },
    /// The facing, now (`sys 11`).
    Yaw(f32),
    /// A jump (`sys 50` → `0x1402B7F20`): hop height and air velocity.
    Jump { height: f32, velocity: Vec3 },
    /// Underground on / off (the Shadow: the script's `wp+0x3C` and its kind-2 point masks): for
    /// the log. What can hit it is [`Cmd::Hittable`].
    Underground(bool),
    /// KH1's actor flag `0x40` cleared (true) or set (false): with it set, KH1's attack-vs-hurt
    /// test (`0x1402B6AF0`, gated by `0x140291920`: `flags & 0x8048 == 0x8000`) and the automatic
    /// target search (`0x1402B0A00`, gated by `0x140292070`, same test) skip the actor.
    Hittable(bool),
    /// The model's height scale a script ramps (the Shadow's sink 1 → 0, rise 0 → 1): actor
    /// `+0x44` (Assumption: the y of the scale `+0x40..`, from the layout).
    Height(f32),
    /// A fade-in over n frames (`0x2570` / `0x2010`: alpha `+0x60` = 0, `+0x64` = 1 / n).
    FadeIn(f32),
    /// Target set (`sys 49` with the target) or cleared.
    Target(bool),
    /// The state object changed (the enemy's own state numbers): for the log.
    State(u8),
    /// The action just requested carries KH1's request bit 23 (actor flag `0x1000`,
    /// [`Body::held`]): its root motion moves the body, KH1's air step does nothing until it ends.
    /// Not used by the Shadow.
    Held,
    /// A plain motion request (no bit 24) made with attack rows (M11 fliers): KH1's `sys 23`
    /// objects of mode 2 live while the request they first saw plays (`0x1402B47A0`: `+0x58` takes
    /// the owner's request `+0x164` at the first tick, a change ends them; they hit while its HIT
    /// track's slot is on, `+0x1CC & 0x10000 << sub`; Checked (decomp)), and the script is not
    /// paused (no action flag 4): it keeps steering the body while the attack plays. The host runs
    /// it in the class's Re:CoM attack state; it ends when another motion replaces it or its motion
    /// has played.
    Attack { request: u16, attack: A, blend: f32 },
    /// [`Cmd::Attack`] played from `start` KH1 frames (`sys 13`'s fourth word, as [`Cmd::MotionAt`]): the attack's hit
    /// windows are its motion's frames, so they come `start` frames sooner. M11, the Blue Rhapsody's melee (`ex_2120.bd`
    /// `0x4BA`: 215 at blend 8 from frame 8).
    AttackAt { request: u16, attack: A, blend: f32, start: f32 },
    /// The model's scale (actor `+0x40..+0x48`, all three axes; the Darkball's appearance and
    /// warps ramp it, M11): a factor on the model's own scale.
    Scale(f32),
    /// KH1's actor flag `0x1` (the model drawn: `0x14029F421`, Checked (decomp, Epic notes
    /// re-read on Steam: the draw skips an actor without it)): shown (true) or hidden (false).
    Visible(bool),
    /// A plain motion request from a later frame (`sys 13`'s fourth word, `0x1402B9F80`: the start
    /// time `+4` of the request entry `0x14029F980` writes, added when the motion switches,
    /// `0x14029FC40`; Checked (decomp)): played from `start` KH1 frames, unless it is the one playing
    /// ([`Cmd::Motion`]'s rule). M11 (the Wyvern's idle from frame 16, `0x2FD4` / `0x3022`).
    MotionAt { request: u16, blend: f32, start: f32 },
    /// A guard-type row (`crate::enemy::guard`, the shared enemy-guard step, M11) made with the
    /// request just asked for: `Some((row, request))` (the row's index in the enemy's guard list; it
    /// lives while that request plays and its HIT slot is on, `0x1402B47A0`), or `None`: the
    /// enemy's guard rows end now.
    Guard(Option<(u8, u16)>),
    /// `sys 94`: the enemy throws its thrown object `n` (its child script,
    /// `crate::enemy::thrown`; the Bandit's scimitar, M11).
    Throw(u8),
    /// A spell child strikes an area once at `at` (KH1 units, y down: the floor point it marked; M11, the Yellow Opera's
    /// Thunder `ex_2130_00.bd`, `crate::caster::Strike`): the host's strike (a Re:CoM bullet placed by KH1's numbers).
    Strike { child: u8, at: Vec3 },
    /// A strike child marked its place `at` (KH1 units, y down: the floor point; M11, the Yellow Opera's Thunder
    /// `ex_2130_00.bd` `0x376`: KH1's warning cloud, its model's effect 14, `sys 74` 200 above the mark, for the 30 frames
    /// before its [`Cmd::Strike`]): the host's stand-in, one of Re:CoM's own effects there until that child strikes
    /// (owner 2026-11-03).
    Warn { child: u8, at: Vec3 },
    /// Something KH1's script does here that the port doesn't build (an owner question or a later
    /// milestone): for the log only.
    Note(&'static str),
    /// KH1's stats-block write `+0x3C = +0x40` (HP = max HP; the Search Ghost's `0x2ADE`, M11): the
    /// host restores the enemy's own HP to its max (Re:CoM's HP, decision M10).
    HealFull,
    /// KH1's `sys 102` for a support spell child (`0x1402BC0B0` → `0x1402B80A0`; M11, the Green Requiem's Cure, `ex_2140.bd`
    /// `0x33C`): the child's target among the caster's allies, picked with this frame's draw (the same `rand` as `sys 25`)
    /// by KH1's rule ([`crate::greenrequiem::pick`]) from the host's list of them (Re:CoM's battle enemies); the caster itself
    /// when none qualifies. The host keeps it for the child's [`Cmd::Throw`].
    Support { child: u8, draw: f32 },
    /// `sys 13` (`0x1402B9F80` → `0x14029F980`) of the request already playing: KH1 writes the request entry
    /// whatever plays (`+0x5E` = 1, the start time `+0x61`), so the motion starts again from `start` KH1
    /// frames (Checked (decomp)); [`Cmd::Motion`] / [`Cmd::MotionAt`] leave a playing request alone (`sys 12`'s
    /// rule, `0x14029F7C0`). M11, the Fat Bandit's fireballs (`ex_2060.bd` `0x16FC` / `0x166A`: 201 again while
    /// 201 plays).
    Restart { request: u16, blend: f32, start: f32 },
    /// KH1's `sys 194` (`0x1402BC9A0`: an EXP bonus for the party) and `sys 122` with a prize table
    /// (`0x1402ABC40`) from a script handler (M11, the Search Ghost's hit during its drain's hold).
    /// The host carries it out on the enemy's own EXP and prizes ([`Reward`]).
    Reward(Reward),
}

/// A script's reward (M11, the Search Ghost's `0x28A6`): KH1's EXP bonus `EXP / exp_div + exp_add`
/// on the enemy's EXP (KH1's stats `+0x68`, the record's EXP word r7, `0x1402A58E0`; integer
/// division, VM op 1 sub 0 imm 3), and the prize counts of a KH1 drop table by KH1 prize kind
/// (`0x1402ABC40`'s seven counts; the kinds are `0x1402AC080`'s switch: 0..2 munny 1 / 5 / 20,
/// 3 / 4 HP 1 / 5, 5 / 6 MP 2 / 15). Checked (decomp). Every number comes from the player's script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reward {
    pub exp_div: i32,
    pub exp_add: i32,
    /// The bonus is at least this (the Fat Bandit's stagger `0x6B8..0x746`: `sys 195(point, 1)` when
    /// `EXP / 5 − 1 < 0`, else `EXP / 5`; 0 for a script with no floor). M11.
    pub exp_min: i32,
    pub prizes: [u8; 7],
}

impl Reward {
    /// KH1's bonus on an enemy EXP `exp` (`sys 194`'s argument). A zero divisor gives the addend.
    pub fn exp_bonus(&self, exp: i32) -> i32 {
        if self.exp_div == 0 {
            return self.exp_add.max(self.exp_min);
        }
        (exp / self.exp_div + self.exp_add).max(self.exp_min)
    }
}

impl<A: Copy> Cmd<A> {
    /// The same command with the attack mapped (the host's index).
    pub fn map<B>(&self, f: impl Fn(A) -> B) -> Cmd<B> {
        match *self {
            Cmd::Motion { request, blend } => Cmd::Motion { request, blend },
            Cmd::Queue { request, blend } => Cmd::Queue { request, blend },
            Cmd::QueueAt { request, blend, start } => Cmd::QueueAt { request, blend, start },
            Cmd::Action { request, attack, then, blend } => Cmd::Action { request, attack: attack.map(f), then, blend },
            Cmd::Yaw(y) => Cmd::Yaw(y),
            Cmd::Jump { height, velocity } => Cmd::Jump { height, velocity },
            Cmd::Underground(u) => Cmd::Underground(u),
            Cmd::Hittable(h) => Cmd::Hittable(h),
            Cmd::Height(h) => Cmd::Height(h),
            Cmd::FadeIn(n) => Cmd::FadeIn(n),
            Cmd::Target(t) => Cmd::Target(t),
            Cmd::State(s) => Cmd::State(s),
            Cmd::Held => Cmd::Held,
            Cmd::Attack { request, attack, blend } => Cmd::Attack { request, attack: f(attack), blend },
            Cmd::AttackAt { request, attack, blend, start } => Cmd::AttackAt { request, attack: f(attack), blend, start },
            Cmd::Scale(k) => Cmd::Scale(k),
            Cmd::Visible(v) => Cmd::Visible(v),
            Cmd::MotionAt { request, blend, start } => Cmd::MotionAt { request, blend, start },
            Cmd::Guard(g) => Cmd::Guard(g),
            Cmd::Throw(n) => Cmd::Throw(n),
            Cmd::Strike { child, at } => Cmd::Strike { child, at },
            Cmd::Warn { child, at } => Cmd::Warn { child, at },
            Cmd::Note(n) => Cmd::Note(n),
            Cmd::HealFull => Cmd::HealFull,
            Cmd::Support { child, draw } => Cmd::Support { child, draw },
            Cmd::Restart { request, blend, start } => Cmd::Restart { request, blend, start },
            Cmd::Reward(r) => Cmd::Reward(r),
        }
    }
}

/// What the host tells the AI each frame (KH1 units, y down, 60 Hz frames).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct World {
    pub step: f32,
    pub pos: Vec3,
    /// Sora's position (the scripts' target `actor+0x74` is always Sora here: Re:CoM has no party
    /// in battle yet), or none.
    pub target: Option<Vec3>,
    /// Movement state 0 (on the ground, not in an arc).
    pub grounded: bool,
    /// Movement state 2 (in an arc).
    pub airborne: bool,
    /// Last frame's displacement length (`+0x10 − +0x490`, the stuck test).
    pub moved: f32,
    /// The KH1 request playing.
    pub motion: Option<u16>,
}

/// One frame's random draw, for the log (`sys 25` = `0x14031BA30`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draw {
    pub value: f32,
    pub what: &'static str,
}

/// The script's world: the constants, the motion lengths, this frame's world, the body, the
/// random stream and the work area every script has (home `wp+0x10`, target `actor+0x74`, the
/// state object `wp+0x20`); `X` holds the enemy's own work area.
pub struct Core<C, A, X> {
    pub c: C,
    pub frames: Vec<(u16, f32)>,
    pub w: World,
    pub body: Body,
    pub rng: Kh1Rand,
    pub draws: Vec<Draw>,
    pub out: Vec<Cmd<A>>,
    pub home: Vec3,
    pub has_target: bool,
    /// The state object (`wp+0x20`): value, changed.
    pub state: (u8, bool),
    /// The air steering this frame (`+0x20` × `+0x2C`).
    pub steer: Option<Vec3>,
    /// The last walk / run request (the host plays it while moving).
    pub moving: Option<u16>,
    /// The walk / run motion blend ([`super::MOVE_BLEND`]; the Shadow's port passes its script's
    /// 8, the same number).
    pub move_blend: f32,
    /// The floor's height (KH1 y, down) under the body, from the host (`sys 89`'s `+0x3D4` =
    /// floor − y, `0x1402999A0`; M11 fliers), or none.
    pub floor: Option<f32>,
    /// The request playing and the frames it has played ([`tick_clock`], [`motion_done`]).
    pub clock: (Option<u16>, f32),
    pub x: X,
}

pub type Cx<C, A, X> = Rc<RefCell<Core<C, A, X>>>;
pub type Task = Pin<Box<dyn Future<Output = ()>>>;

impl<C, A, X> Core<C, A, X> {
    pub fn new(c: C, frames: Vec<(u16, f32)>, seed: u32, yaw: f32, move_blend: f32, x: X) -> Core<C, A, X> {
        Core {
            c,
            frames,
            w: World::default(),
            body: Body { yaw, yaw_target: yaw, ..Default::default() },
            rng: Kh1Rand::new(seed),
            draws: Vec::new(),
            out: Vec::new(),
            home: Vec3::ZERO,
            has_target: false,
            state: (0, false),
            steer: None,
            moving: None,
            move_blend,
            floor: None,
            clock: (None, 0.0),
            x,
        }
    }
}

pub struct Yield(bool);

impl Future for Yield {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            Poll::Pending
        }
    }
}

/// `misc0`: the rest of the frame.
pub fn frame() -> Yield {
    Yield(false)
}

fn waker() -> Waker {
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VT)
    }
    fn noop(_: *const ()) {}
    static VT: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    // SAFETY: the vtable's functions ignore the data pointer.
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VT)) }
}

/// Polls a thread once; a finished thread is dropped (`misc8`).
pub fn poll(t: &mut Option<Task>) {
    if let Some(f) = t.as_mut() {
        let w = waker();
        let mut cx = Context::from_waker(&w);
        if f.as_mut().poll(&mut cx).is_ready() {
            *t = None;
        }
    }
}

/// `sys 25`: KH1's random float (logged).
pub fn rand<C, A, X>(cx: &Cx<C, A, X>, what: &'static str) -> f32 {
    let mut io = cx.borrow_mut();
    let v = io.rng.next_f32();
    io.draws.push(Draw { value: v, what });
    v
}

/// Shadow `0x1FB6`, Soldier `0x1A70` (same bytes): rand × (b − a) + a.
pub fn rand_range<C, A, X>(cx: &Cx<C, A, X>, a: f32, b: f32, what: &'static str) -> f32 {
    rand(cx, what) * (b - a) + a
}

/// Shadow `0x1F70`, Soldier `0x1A2A` (same bytes): a random offset of length [a, b) in a random
/// direction (`sys 10` rotates (len, 0, 0) by rand × 360°; the rotation's sign is an Assumption
/// that changes nothing: the angle is uniform).
pub fn random_offset<C, A, X>(cx: &Cx<C, A, X>, a: f32, b: f32) -> Vec3 {
    let len = rand_range(cx, a, b, "offset length");
    let deg = rand(cx, "offset angle") * 360.0;
    let r = deg.to_radians();
    Vec3::new(len * r.cos(), 0.0, -len * r.sin())
}

pub fn push<C, A, X>(cx: &Cx<C, A, X>, c: Cmd<A>) {
    cx.borrow_mut().out.push(c);
}

pub fn pos<C, A, X>(cx: &Cx<C, A, X>) -> Vec3 {
    cx.borrow().w.pos
}

pub fn target<C, A, X>(cx: &Cx<C, A, X>) -> Option<Vec3> {
    let io = cx.borrow();
    io.w.target.filter(|_| io.has_target)
}

/// Shadow `0x2102` / `0x1D5A`, Soldier `0x1BBC` / `0x17D2`: the distance to the target (`sys 6`,
/// 3D), or a large value with none.
pub fn dist_to_target<C, A, X>(cx: &Cx<C, A, X>) -> f32 {
    target(cx).map_or(1.0e8, |t| (t - pos(cx)).length())
}

/// `sys 49`: the target set or cleared (Soldier `0x1D7C` / `0x1D6A`).
pub fn set_target<C, A, X>(cx: &Cx<C, A, X>, on: bool) {
    let changed = {
        let mut io = cx.borrow_mut();
        let ch = io.has_target != on;
        io.has_target = on;
        ch
    };
    if changed {
        push(cx, Cmd::Target(on));
    }
}

/// Shadow `0x2302`, Soldier `0x1DD8`: the target yaw toward the target (`sys 36`: +0x150 only,
/// the engine turns).
pub fn face_target<C, A, X>(cx: &Cx<C, A, X>) {
    if let Some(t) = target(cx) {
        let d = t - pos(cx);
        cx.borrow_mut().body.yaw_target = d.x.atan2(d.z);
    }
}

/// Soldier `0x1E02`: face the target now (`sys 11`: +0x150 and the facing, `0x140297E10`).
pub fn face_target_now<C, A, X>(cx: &Cx<C, A, X>) {
    if let Some(t) = target(cx) {
        let d = t - pos(cx);
        let y = d.x.atan2(d.z);
        let mut io = cx.borrow_mut();
        io.body.yaw = y;
        io.body.yaw_target = y;
        io.out.push(Cmd::Yaw(y));
    }
}

/// The state object changes (Soldier `0x19EA`; the Shadow's `wp+0x20`): the supervisor restarts
/// the behaviour thread.
pub fn set_state<C, A, X>(cx: &Cx<C, A, X>, s: u8) {
    cx.borrow_mut().state = (s, true);
}

/// `sys 43`: a motion's length (Assumption: its frame count).
pub fn motion_frames<C, A, X>(cx: &Cx<C, A, X>, r: u16) -> f32 {
    cx.borrow().frames.iter().find(|x| x.0 == r).map_or(0.0, |x| x.1)
}

/// Shadow `0x2D42`, Soldier `0x23EA`: wait `n` frames (counted down by the step `sys 5`).
pub async fn wait_frames<C, A, X>(cx: &Cx<C, A, X>, n: f32) {
    let mut t = n;
    while t > 0.0 {
        let s = cx.borrow().w.step;
        t -= s;
        frame().await;
    }
}

/// Shadow `0x1D74`, Soldier `0x182E` (same bytes): wait until on the ground (movement state 0).
pub async fn wait_grounded<C, A, X>(cx: &Cx<C, A, X>) {
    while !cx.borrow().w.grounded {
        frame().await;
    }
}

/// Soldier `0x998` / `0xA82` / `0xB6A` / `0x174E`: wait while in the air (movement state 2).
pub async fn wait_airborne<C, A, X>(cx: &Cx<C, A, X>) {
    while cx.borrow().w.airborne {
        frame().await;
    }
}

/// `sys 40` → `0x140299D60(actor, goal, mode)`: walk (mode 0, `+4`), run (1, `+8`) or glide (2,
/// `+0xC`, no motion) toward `goal`: the facing turns by the angle to the goal, divided by
/// `dist / speed × 0.3` when that is above 1; the velocity this frame is the facing × min(speed,
/// dist); returns 0 once within one frame's travel, else the distance. The walk / run request
/// (1 / 2, blend 8) plays while moving. Not ported: the floor probe ahead (`0x140193650`; a drop
/// makes it hop with `+0x10` and fall): Re:CoM's arenas are flat (Assumption).
pub fn move_toward<C: Numbers + Clone, A, X>(cx: &Cx<C, A, X>, goal: Vec3, mode: u8, speed2: f32) -> f32 {
    let mut io = cx.borrow_mut();
    let speed = match mode {
        0 => io.c.walk_speed(),
        1 => io.c.run_speed(),
        _ => speed2,
    };
    let to = (goal - io.w.pos).flat();
    let (dir, dist) = to.normalize_len();
    let cur = io.body.yaw;
    let mut turn = wrap_angle(dir.x.atan2(dir.z) - cur);
    let k = dist / speed * 0.3;
    if k > 1.0 {
        turn /= k;
    }
    let y = wrap_angle(cur + turn);
    io.body.yaw = y;
    io.body.yaw_target = y;
    let v = Vec3::from_yaw(y) * dist.min(speed);
    io.out.push(Cmd::Yaw(y));
    let step = io.w.step;
    if dist < step * speed {
        return 0.0;
    }
    io.body.walk = v;
    if mode <= 1 {
        let r = if mode == 0 { super::react::req::WALK } else { super::react::req::RUN };
        io.moving = Some(r);
        let blend = io.move_blend;
        io.out.push(Cmd::Motion { request: r, blend });
    }
    dist
}

/// Shadow `0x2440`, Soldier `0x1F16` (same bytes): move each frame until arrived on the ground or
/// the time is out.
pub async fn walk_to<C: Numbers + Clone, A, X>(cx: &Cx<C, A, X>, goal: Vec3, mode: u8, timeout: f32) -> f32 {
    let mut t = timeout;
    let mut r = 0.0;
    while t > 0.0 {
        r = move_toward(cx, goal, mode, 0.0);
        frame().await;
        if r <= 0.0 && cx.borrow().w.grounded {
            break;
        }
        t -= cx.borrow().w.step;
    }
    r
}

/// Shadow `0x2374`, Soldier `0x1E4A` (the same routine; the Soldier's copy also calls `sys 1`,
/// a debug print): chase the target (its position re-read every `retarget` frames) until within
/// `dist` or the time is out.
pub async fn approach<C: Numbers + Clone, A, X>(cx: &Cx<C, A, X>, mode: u8, timeout: f32, retarget: f32, dist: f32) -> f32 {
    let mut t = timeout;
    let mut rt = 0.0;
    let mut goal = pos(cx);
    let mut r = 0.0;
    while t > 0.0 {
        if rt <= 0.0 {
            if let Some(tg) = target(cx) {
                goal = tg;
            }
            rt = retarget;
        }
        r = move_toward(cx, goal, mode, 0.0);
        if r - dist < 0.0 {
            break;
        }
        let s = cx.borrow().w.step;
        t -= s;
        rt -= s;
        frame().await;
    }
    r
}

/// Soldier `0x15BA`, Shadow `0x1BC4` (the same routine): wander. Two random points around the
/// actor's **current** position (`sys 7` adds `actor+0x10`; lengths [inner − band, inner) and
/// [outer − band, outer), band 100), walk to each (timeout), sometimes stand (the idle motion,
/// wait [idle_min, idle_max)), and walk home when farther than `home_max`.
#[allow(clippy::too_many_arguments)]
pub async fn wander<C: Numbers + Clone, A, X>(cx: &Cx<C, A, X>, inner: f32, outer: f32, band: f32, timeout: f32, idle_chance: f32, idle_min: f32, idle_max: f32, home_max: f32, idle_blend: f32) {
    let home = cx.borrow().home;
    let p = pos(cx);
    let a = p + random_offset(cx, inner - band, inner);
    let b = p + random_offset(cx, outer - band, outer);
    walk_to(cx, a, 0, timeout).await;
    walk_to(cx, b, 0, timeout).await;
    if rand(cx, "wander idle") - idle_chance < 0.0 {
        push(cx, Cmd::Motion { request: super::react::req::IDLE, blend: idle_blend });
        let n = rand_range(cx, idle_min, idle_max, "wander idle time");
        wait_frames(cx, n).await;
    }
    if (pos(cx) - home).length() - home_max > 0.0 {
        walk_to(cx, home, 0, timeout).await;
        frame().await;
    }
}

/// Soldier `0x1BFC`, Shadow `0x2142`: the jump start (request 3 as an action, 4 queued with
/// `0x40010004`), a yield (the action plays), then the jump (`sys 50`: hop `height`, air velocity
/// `v`).
pub async fn jump_at<C: Numbers + Clone, A, X>(cx: &Cx<C, A, X>, height: f32, v: Vec3) {
    push(cx, Cmd::Action { request: super::react::req::JUMP, attack: None, then: super::react::req::FALL, blend: 0.0 });
    frame().await;
    let y = pos(cx).y;
    {
        let mut io = cx.borrow_mut();
        let c = io.c.clone();
        io.body.jump(height, Vec3::new(v.x, 0.0, v.z), y, &c);
    }
    push(cx, Cmd::Jump { height, velocity: v });
}

/// The supervisor's state-machine helpers (Soldier `0x1996` restart, `0x19B4` take the request,
/// `0x19EA` request, `0x1A06` clear): the request taken this frame, if any.
pub fn take_state<C, A, X>(cx: &Cx<C, A, X>) -> Option<u8> {
    let mut io = cx.borrow_mut();
    let ch = io.state.1;
    io.state.1 = false;
    ch.then_some(io.state.0)
}

/// The flying enemies' steering (M11 fliers): the script writes the actor's `+0x20` (a unit
/// direction, `y` included) and `+0x2C` (a speed) for this frame; movement state 4 follows it
/// ([`Body::flying`]).
pub fn steer<C, A, X>(cx: &Cx<C, A, X>, dir: Vec3, speed: f32) {
    cx.borrow_mut().body.walk = dir * speed;
}

/// `sys 135`: movement state 4 (flight) with the flight velocity `v` ([`Body::fly`]).
pub fn fly<C, A, X>(cx: &Cx<C, A, X>, v: Vec3) {
    cx.borrow_mut().body.fly(v);
}

/// `sys 89` (`0x1402BA5D0`): the height above the floor, `+0x3D4` = floor y − y (KH1 y down;
/// `0x1402999A0`, Checked (decomp)); 0 without a floor (as KH1's when no floor was found).
pub fn height<C, A, X>(cx: &Cx<C, A, X>) -> f32 {
    let io = cx.borrow();
    io.floor.map_or(0.0, |f| f - io.w.pos.y)
}

/// A script's write of the stats block's `+0x14` (the flight's easing, `0x1402CC000`'s `a`; the
/// Darkball's script sets it, M11): [`Body::accel`].
pub fn set_air_accel<C, A, X>(cx: &Cx<C, A, X>, a: f32) {
    cx.borrow_mut().body.accel = Some(a);
}

/// `sys 86` (`0x1402BA560` → `0x140292BC0`): the actor moved to `to` (KH1 units) with the facing
/// `yaw` (the vector's fourth word, wrapped by `0x1402A8280`), its flight velocity `+0x3C0..+0x3D0`
/// and speed `+0x2C` zeroed (Checked (decomp)). The host moves the body there this frame
/// ([`Body::warp`]).
pub fn warp<C, A, X>(cx: &Cx<C, A, X>, to: Vec3, yaw: f32) {
    let mut io = cx.borrow_mut();
    let d = to - io.w.pos;
    io.body.warp(d, crate::math::wrap_angle(yaw));
    let y = io.body.yaw;
    io.out.push(Cmd::Yaw(y));
}

/// `sys 125` (`0x1402BABA0`): the turn rate, in degrees a frame.
pub fn set_turn_rate<C, A, X>(cx: &Cx<C, A, X>, deg: f32) {
    cx.borrow_mut().body.turn = Some(deg / 180.0 * std::f32::consts::PI);
}

/// `sys 172` (`0x14029EFC0`): the motion has played to its end (`+0x280 ≤ step × +0x284 +
/// +0x16C`: its length against its time plus this frame's step) with no blend running and nothing
/// queued. The time is [`Core::clock`] (the frames the request playing has played, counted by the
/// behaviour from the host's world each frame: [`tick_clock`]); the blend and the queue are not
/// read (Assumption: the scripts that wait on it queue nothing then).
pub fn motion_done<C, A, X>(cx: &Cx<C, A, X>) -> bool {
    let io = cx.borrow();
    let Some(r) = io.clock.0 else { return true };
    let len = io.frames.iter().find(|x| x.0 == r).map_or(0.0, |x| x.1);
    len <= io.w.step + io.clock.1
}

/// Counts the frames the host's current request has played ([`Core::clock`]), once a frame
/// before the script runs. A request seen for the first time has played one step (KH1 advances it
/// in the motion update of the frame that requested it, after the script; Assumption ±1 frame).
pub fn tick_clock<C, A, X>(io: &mut Core<C, A, X>) {
    if io.clock.0 == io.w.motion && io.clock.0.is_some() {
        io.clock.1 += io.w.step;
    } else {
        io.clock = (io.w.motion, io.w.step);
    }
}
