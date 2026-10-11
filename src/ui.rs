//! UI: loading splash (kaleidoscope shader), title menu, HUD, radar, overlays.
use crate::audio;
use crate::players::*;
use crate::enemies::{Enemy, EnemyKind};
use crate::game::*;
use crate::settings::Cvars;
use crate::story::{ending_text, Story};
use crate::vehicle::Vehicle;
use crate::world::Chunks;
use bevy::asset::weak_handle;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::ui::{MaterialNode, UiMaterial, UiMaterialPlugin};

pub const LOAD_SHADER: Handle<Shader> = weak_handle!("4b1d2e2e-7c53-4c0e-9f1a-0a5e1c3d9b03");
pub const TUNNEL_SHADER: Handle<Shader> = weak_handle!("4b1d2e2e-7c53-4c0e-9f1a-0a5e1c3d9b04");
pub const CITY_SHADER: Handle<Shader> = weak_handle!("4b1d2e2e-7c53-4c0e-9f1a-0a5e1c3d9b05");
/// Tunable "PI" fed to the loading shader (user asked for 3.6 or 4.1).
pub const LOAD_PI: f32 = 3.6;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LoadMat {
    #[uniform(0)]
    pub p: Vec4,
}
impl UiMaterial for LoadMat {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(LOAD_SHADER)
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TunnelMat {
    #[uniform(0)]
    pub p: Vec4,
}
impl UiMaterial for TunnelMat {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(TUNNEL_SHADER)
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CityMat {
    #[uniform(0)]
    pub p: Vec4,
}
impl UiMaterial for CityMat {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(CITY_SHADER)
    }
}
#[derive(Component)]
struct IntroRoot;
#[derive(Component)]
pub struct HudRoot(pub usize);
#[derive(Component)]
struct HudSlot(usize);
#[derive(Component)]
struct Tagged;
#[derive(Component)]
struct GradLetter {
    i: usize,
    n: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum G {
    Intro,
    Loading,
    Menu,
    Hud,
    Pause,
    Dead,
    Ending,
}
#[derive(Component)]
struct Group(G);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum HudText {
    Speed,
    Hp,
    Ammo,
    Chapter,
    Objective,
    Compass,
    Score,
    Fps,
    DlgWho,
    DlgText,
    Toast(usize),
    Boss,
    Hint,
    LoadStatus,
    LoadPct,
    LoadTip,
    Prompt,
    EndText,
    IntroLog,
}
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Bar {
    Hp,
    Boost,
    Hack,
    Boss,
    Load,
}
#[derive(Component)]
struct BarBox(Bar);
#[derive(Component)]
struct Vignette;
#[derive(Component)]
struct DlgBox;
#[derive(Component)]
struct RadarDot(usize);
#[derive(Component)]
struct LoadRoot;

pub struct UiPlugin;
impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::load_internal_asset!(app, LOAD_SHADER, "shaders/loading.wgsl", Shader::from_wgsl);
        bevy::asset::load_internal_asset!(app, TUNNEL_SHADER, "shaders/tunnel.wgsl", Shader::from_wgsl);
        bevy::asset::load_internal_asset!(app, CITY_SHADER, "shaders/city.wgsl", Shader::from_wgsl);
        app.add_plugins((UiMaterialPlugin::<LoadMat>::default(), UiMaterialPlugin::<TunnelMat>::default(), UiMaterialPlugin::<CityMat>::default()))
            .add_systems(Startup, setup_ui)
            .add_systems(Update, (loading_update, groups, hud_texts, hud_bars, radar, overlays, fx_ui_update, grad_letters, tag_hud));
    }
}

fn t(s: &str, size: f32, c: Color) -> impl Bundle {
    (Text::new(s), TextFont { font: crate::fonts::term(), font_size: size * 1.3, ..default() }, TextColor(c))
}

fn full() -> Node {
    Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }
}

const CYAN: Color = Color::srgb(0.1, 0.95, 1.0);
const MAG: Color = Color::srgb(1.0, 0.15, 0.8);
const ORANGE: Color = Color::srgb(1.0, 0.6, 0.15);
const DIM: Color = Color::srgba(0.8, 0.7, 1.0, 0.7);

fn grad(k: f32) -> Color {
    let (a, b, c) = (Vec3::new(0.1, 0.95, 1.0), Vec3::new(1.0, 0.15, 0.8), Vec3::new(1.0, 0.6, 0.15));
    let v = if k < 0.5 { a.lerp(b, k * 2.0) } else { b.lerp(c, (k - 0.5) * 2.0) };
    Color::srgb(v.x, v.y, v.z)
}

fn grad_wrap(k: f32) -> Color {
    let (a, b, c) = (Vec3::new(0.1, 0.95, 1.0), Vec3::new(1.0, 0.15, 0.8), Vec3::new(1.0, 0.6, 0.15));
    let x = k.rem_euclid(1.0) * 3.0;
    let v = if x < 1.0 { a.lerp(b, x) } else if x < 2.0 { b.lerp(c, x - 1.0) } else { c.lerp(a, x - 2.0) };
    Color::srgb(v.x, v.y, v.z)
}

fn bar(p: &mut ChildSpawnerCommands, kind: Bar, w: f32, h: f32, c: Color) {
    p.spawn((Node { width: Val::Px(w), height: Val::Px(h), border: UiRect::all(Val::Px(1.0)), ..default() }, BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.35)), BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)), BarBox(kind)))
        .with_children(|b| {
            b.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(c), kind));
        });
}

fn title_row(p: &mut ChildSpawnerCommands, s: &str, size: f32, font: Handle<Font>) {
    p.spawn(Node { flex_direction: FlexDirection::Row, ..default() }).with_children(|r| {
        let n = s.chars().count();
        for (i, ch) in s.chars().enumerate() {
            if ch == ' ' {
                r.spawn(Node { width: Val::Px(size * 0.4), ..default() });
                continue;
            }
            r.spawn((
                Text::new(ch.to_string()),
                TextFont { font: font.clone(), font_size: size, ..default() },
                TextColor(grad(i as f32 / (n - 1).max(1) as f32)),
                TextShadow { offset: Vec2::new(size * 0.04, size * 0.04), color: Color::srgba(0.8, 0.0, 0.6, 0.9) },
                GradLetter { i, n },
            ));
        }
    });
}

/// Spawn one player's HUD. `cam` = the camera entity it renders on (None = the default UI camera).
pub fn spawn_hud(commands: &mut Commands, slot: usize, cam: Option<Entity>) {
    let mut e = commands.spawn((Node { flex_direction: FlexDirection::Column, ..full() }, Group(G::Hud), HudRoot(slot), Visibility::default()));
    if let Some(c) = cam {
        e.insert(UiTargetCamera(c));
    }
    e.with_children(|root| {
            // vignette
            root.spawn((full(), BackgroundColor(Color::srgba(1.0, 0.0, 0.0, 0.0)), Vignette));
            // top-left objective
            root.spawn(Node { position_type: PositionType::Absolute, left: Val::Px(24.0), top: Val::Px(18.0), flex_direction: FlexDirection::Column, ..default() })
                .with_children(|p| {
                    p.spawn((t("", 14.0, DIM), HudText::Chapter));
                    p.spawn((t("", 22.0, CYAN), HudText::Objective));
                    p.spawn((t("", 16.0, ORANGE), HudText::Compass));
                });
            // top-right score + radar
            root.spawn(Node { position_type: PositionType::Absolute, right: Val::Px(24.0), top: Val::Px(18.0), flex_direction: FlexDirection::Column, align_items: AlignItems::FlexEnd, ..default() })
                .with_children(|p| {
                    p.spawn((t("", 20.0, MAG), HudText::Score));
                    p.spawn((t("", 12.0, DIM), HudText::Fps));
                    p.spawn((
                        Node { width: Val::Px(150.0), height: Val::Px(150.0), margin: UiRect::top(Val::Px(8.0)), border: UiRect::all(Val::Px(2.0)), ..default() },
                        BorderRadius::MAX,
                        BorderColor(Color::srgba(0.1, 0.95, 1.0, 0.6)),
                        BackgroundColor(Color::srgba(0.02, 0.0, 0.1, 0.55)),
                    ))
                    .with_children(|r| {
                        r.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(70.0), top: Val::Px(70.0), width: Val::Px(6.0), height: Val::Px(6.0), ..default() }, BackgroundColor(CYAN), BorderRadius::MAX));
                        for i in 0..24 {
                            r.spawn((
                                Node { position_type: PositionType::Absolute, width: Val::Px(6.0), height: Val::Px(6.0), display: Display::None, ..default() },
                                BackgroundColor(Color::srgb(1.0, 0.2, 0.3)),
                                BorderRadius::MAX,
                                RadarDot(i),
                            ));
                        }
                    });
                });
            // toasts
            root.spawn(Node { position_type: PositionType::Absolute, top: Val::Px(70.0), width: Val::Percent(100.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() })
                .with_children(|p| {
                    for i in 0..3 {
                        p.spawn((t("", 20.0, Color::WHITE), HudText::Toast(i)));
                    }
                    p.spawn((t("", 18.0, Color::srgb(1.0, 0.3, 0.3)), HudText::Boss));
                    bar(p, Bar::Boss, 420.0, 10.0, Color::srgb(1.0, 0.25, 0.2));
                });
            // crosshair + hint
            root.spawn(Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, flex_direction: FlexDirection::Column, ..default() })
                .with_children(|p| {
                    p.spawn(t("+", 26.0, Color::srgba(0.1, 0.95, 1.0, 0.85)));
                    p.spawn((Node { margin: UiRect::top(Val::Px(120.0)), ..default() }, t("", 22.0, Color::WHITE), HudText::Hint));
                });
            // dialogue
            root.spawn((
                Node { position_type: PositionType::Absolute, bottom: Val::Px(130.0), left: Val::Percent(20.0), width: Val::Percent(60.0), padding: UiRect::all(Val::Px(12.0)), flex_direction: FlexDirection::Column, border: UiRect::all(Val::Px(1.0)), display: Display::None, ..default() },
                BackgroundColor(Color::srgba(0.03, 0.0, 0.1, 0.75)),
                BorderColor(Color::srgba(1.0, 0.15, 0.8, 0.6)),
                DlgBox,
            ))
            .with_children(|p| {
                p.spawn((t("", 16.0, ORANGE), HudText::DlgWho));
                p.spawn((t("", 20.0, Color::WHITE), HudText::DlgText));
            });
            // bottom-left: hp + weapon
            root.spawn(Node { position_type: PositionType::Absolute, left: Val::Px(24.0), bottom: Val::Px(24.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), ..default() })
                .with_children(|p| {
                    p.spawn((t("", 22.0, ORANGE), HudText::Ammo));
                    p.spawn((t("", 16.0, Color::srgb(0.4, 1.0, 0.6)), HudText::Hp));
                    bar(p, Bar::Hp, 300.0, 14.0, Color::srgb(0.2, 1.0, 0.5));
                });
            // bottom-right: speed + boost
            root.spawn(Node { position_type: PositionType::Absolute, right: Val::Px(24.0), bottom: Val::Px(24.0), flex_direction: FlexDirection::Column, align_items: AlignItems::FlexEnd, row_gap: Val::Px(6.0), ..default() })
                .with_children(|p| {
                    p.spawn((t("", 46.0, CYAN), HudText::Speed, TextShadow { offset: Vec2::new(2.0, 2.0), color: Color::srgba(1.0, 0.1, 0.7, 0.8) }));
                    bar(p, Bar::Boost, 260.0, 10.0, Color::srgb(0.1, 0.8, 1.0));
                    bar(p, Bar::Hack, 260.0, 10.0, Color::srgb(0.2, 1.0, 0.4));
                });
        });
}

fn setup_ui(mut commands: Commands, mut mats: ResMut<Assets<LoadMat>>, mut tmats: ResMut<Assets<TunnelMat>>, mut cmats: ResMut<Assets<CityMat>>) {
    spawn_hud(&mut commands, 0, None);

    // ---------------- Menu ----------------
    commands
        .spawn((Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, row_gap: Val::Px(10.0), ..full() }, Group(G::Menu), Visibility::default()))
        .with_children(|p| {
            // reflective neon-city water band behind the menu text
            p.spawn((
                Node { position_type: PositionType::Absolute, left: Val::Px(0.0), bottom: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(34.0), ..default() },
                MaterialNode(cmats.add(CityMat { p: Vec4::new(0.0, 1.0, 0.0, 0.0) })),
            ));
            p.spawn(Node { margin: UiRect::bottom(Val::Px(0.0)), ..default() }).with_children(|r| title_row(r, "NEON DUST", 128.0, crate::fonts::logo()));
            p.spawn(t("A SYNTHWAVE ROAD-RAGE ODYSSEY THROUGH THE SKY OF A DEAD MACHINE", 18.0, DIM));
            p.spawn((Node { margin: UiRect::vertical(Val::Px(26.0)), ..default() }, t("PRESS ENTER", 34.0, Color::WHITE), HudText::Prompt));
            p.spawn(t("WASD drive/move   SHIFT boost/sprint   SPACE drift/jump   LMB shoot   RMB rockets/aim", 16.0, CYAN));
            p.spawn(t("F enter/exit car   1-4 weapons   E hack   ESC pause   ` console", 16.0, CYAN));
            p.spawn((Node { margin: UiRect::top(Val::Px(26.0)), max_width: Val::Px(760.0), ..default() }, t("You are XZ. You woke up inside your computer. The only way out is the Port at the edge of the system. ORACLE says do not stop moving.", 17.0, Color::srgba(1.0, 0.8, 0.95, 0.85))));
        });

    // ---------------- Pause ----------------
    commands
        .spawn((Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, row_gap: Val::Px(14.0), ..full() }, BackgroundColor(Color::srgba(0.02, 0.0, 0.08, 0.7)), Group(G::Pause), Visibility::default()))
        .with_children(|p| {
            title_row(p, "PAUSED", 80.0, crate::fonts::shade());
            p.spawn(t("ESC / ENTER  resume", 22.0, Color::WHITE));
            p.spawn(t("` open console (cheats on  ->  god, noclip, give all ...)", 16.0, DIM));
        });

    // ---------------- Dead ----------------
    commands
        .spawn((Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, row_gap: Val::Px(14.0), ..full() }, BackgroundColor(Color::srgba(0.15, 0.0, 0.02, 0.6)), Group(G::Dead), Visibility::default()))
        .with_children(|p| {
            p.spawn((Text::new("KERNEL PANIC"), TextFont { font: crate::fonts::shade(), font_size: 90.0, ..default() }, TextColor(Color::srgb(1.0, 0.15, 0.2)), TextShadow { offset: Vec2::new(4.0, 4.0), color: Color::srgba(0.4, 0.0, 0.4, 0.9) }));
            p.spawn(t("process 0xXZ terminated unexpectedly", 20.0, DIM));
            p.spawn(t("ENTER  reboot from last checkpoint", 26.0, Color::WHITE));
        });

    // ---------------- Ending ----------------
    commands
        .spawn((Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, row_gap: Val::Px(16.0), ..full() }, MaterialNode(tmats.add(TunnelMat { p: Vec4::new(0.0, 0.9, 0.8, 1.0) })), Group(G::Ending), Visibility::default()))
        .with_children(|p| {
            title_row(p, "THE PORT IS OPEN", 60.0, crate::fonts::shade());
            p.spawn((Node { max_width: Val::Px(820.0), ..default() }, t("", 22.0, Color::WHITE), HudText::EndText));
        });

    // ---------------- Boot intro tunnel ----------------
    commands
        .spawn((
            Node { flex_direction: FlexDirection::Column, align_items: AlignItems::FlexStart, justify_content: JustifyContent::Center, padding: UiRect::left(Val::Percent(8.0)), row_gap: Val::Px(6.0), ..full() },
            MaterialNode(tmats.add(TunnelMat { p: Vec4::new(0.0, 1.0, 2.2, 0.0) })),
            GlobalZIndex(400),
            Group(G::Intro),
            IntroRoot,
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn((t("", 30.0, Color::srgb(0.2, 1.0, 0.7)), HudText::IntroLog, TextShadow { offset: Vec2::new(2.0, 2.0), color: Color::BLACK }));
        });

    // ---------------- Loading splash ----------------
    let mat = mats.add(LoadMat { p: Vec4::new(0.0, 0.0, LOAD_PI, 0.0) });
    commands
        .spawn((
            Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, row_gap: Val::Px(12.0), ..full() },
            MaterialNode(mat),
            GlobalZIndex(500),
            Group(G::Loading),
            LoadRoot,
            Visibility::default(),
        ))
        .with_children(|p| {
            title_row(p, "NEON DUST", 96.0, crate::fonts::logo());
            p.spawn((Node { margin: UiRect::top(Val::Px(40.0)), ..default() }, t("", 26.0, Color::WHITE), HudText::LoadStatus, TextShadow { offset: Vec2::new(2.0, 2.0), color: Color::BLACK }));
            bar(p, Bar::Load, 520.0, 12.0, Color::srgb(1.0, 0.2, 0.8));
            p.spawn((t("", 16.0, CYAN), HudText::LoadPct));
            p.spawn((Node { margin: UiRect::top(Val::Px(30.0)), ..default() }, t("", 16.0, Color::srgba(1.0, 0.9, 1.0, 0.85)), HudText::LoadTip, TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::BLACK }));
        });
}

const STATUS: [&str; 10] = [
    "MOUNTING /dev/null",
    "SYNTHESIZING SOUNDTRACK",
    "GROWING FLOATING ISLANDS",
    "CALIBRATING NEON TUBES",
    "WAKING THE ORACLE",
    "DEFRAGMENTING DREAMS",
    "COMPILING SUNSET",
    "REROUTING THE VOID",
    "TUNING ENGINE HARMONICS",
    "ALLOCATING XZ.EXE",
];
const TIPS: [&str; 6] = [
    "TIP  Hold SHIFT to boost - it recharges while you coast.",
    "TIP  Tap SPACE in a corner to drift. Dust looks cool.",
    "TIP  Press F to bail out of the car at full speed. Dramatic.",
    "TIP  Rockets splash. So do you. Keep your distance.",
    "TIP  Sentries hate being circled. Drones hate being shot.",
    "TIP  Open the console with ` and type  help",
];

fn loading_update(
    time: Res<Time<Real>>,
    chunks: Res<Chunks>,
    mut loading: ResMut<Loading>,
    mut mats: ResMut<Assets<LoadMat>>,
    q: Query<&MaterialNode<LoadMat>>,
    mut toasts: ResMut<Toasts>,
) {
    if loading.done {
        return;
    }
    loading.t += time.delta_secs();
    let ap = audio::audio_progress();
    let cp = (chunks.loaded.len() as f32 / 24.0).min(1.0);
    let target = (ap * 0.6 + cp * 0.4).min(1.0);
    loading.progress += (target - loading.progress) * (4.0 * time.delta_secs()).min(1.0);
    for m in &q {
        if let Some(mm) = mats.get_mut(&m.0) {
            mm.p = Vec4::new(loading.t, loading.progress, LOAD_PI, 0.0);
        }
    }
    if loading.t > 5.0 && ap >= 1.0 && cp >= 1.0 {
        loading.done = true;
        toasts.say("SYSTEM ONLINE - press ENTER");
    }
}

fn groups(
    state: Res<State<AppState>>,
    loading: Res<Loading>,
    intro: Res<Intro>,
    pl: Res<Pl>,
    mut q: Query<(&Group, &mut Node, Option<&HudRoot>)>,
) {
    let st = *state.get();
    for (g, mut n, hr) in &mut q {
        let show = match g.0 {
            G::Intro => intro.active && st == AppState::Playing,
            G::Loading => !loading.done,
            G::Menu => st == AppState::Menu && loading.done,
            G::Hud => matches!(st, AppState::Playing | AppState::Paused | AppState::Dead) && loading.done && hr.map_or(true, |h| pl.s[h.0.min(MAXP - 1)].active),
            G::Pause => st == AppState::Paused,
            G::Dead => st == AppState::Dead,
            G::Ending => st == AppState::Ending,
        };
        n.display = if show { Display::Flex } else { Display::None };
    }
}

#[allow(clippy::too_many_arguments)]
fn hud_texts(
    time: Res<Time<Real>>,
    stats: Res<Stats>,
    pl: Res<Pl>,
    story: Res<Story>,
    cv: Res<Cvars>,
    loading: Res<Loading>,
    intro: Res<Intro>,
    mut toasts: ResMut<Toasts>,
    car: Query<(&Vehicle, &PlayerCar)>,
    boss: Query<&Enemy>,
    mut fps: Local<f32>,
    mut q: Query<(&mut Text, &mut TextColor, &HudText, Option<&HudSlot>)>,
    mut dlg: Query<&mut Node, With<DlgBox>>,
) {
    let dt = time.delta_secs();
    *fps += (1.0 / dt.max(1e-4) - *fps) * 0.05;
    for t in toasts.0.iter_mut() {
        t.1 -= dt;
    }
    toasts.0.retain(|t| t.1 > 0.0);
    if toasts.0.len() > 3 {
        let n = toasts.0.len() - 3;
        toasts.0.drain(..n);
    }
    let mut speeds = [0.0f32; MAXP];
    for (v, p) in &car {
        speeds[p.0.min(MAXP - 1)] = v.speed;
    }
    let bossv = boss.iter().find(|e| e.kind == EnemyKind::Boss);
    for mut n in &mut dlg {
        n.display = if story.cur.is_some() { Display::Flex } else { Display::None };
    }
    let blink = (time.elapsed_secs() * 3.0).sin() * 0.5 + 0.5;
    for (mut text, mut color, kind, hs) in &mut q {
        let slot = hs.map_or(0, |h| h.0.min(MAXP - 1));
        let sl = &pl.s[slot];
        let ctl_in_car = sl.in_car;
        match kind {
            HudText::Speed => text.0 = if sl.dead_t > 0.0 { "DOWN".into() } else if ctl_in_car { format!("{:>3} KM/H", (speeds[slot] * 3.6) as i32) } else { "ON FOOT".into() },
            HudText::Hp => text.0 = format!("{}INTEGRITY {:>3}", if pl.count() > 1 { format!("P{} ", slot + 1) } else { String::new() }, sl.hp.max(0.0) as i32),
            HudText::Ammo => {
                text.0 = if ctl_in_car {
                    format!("CANNONS  ROCKETS x{}", sl.ammo[3])
                } else {
                    let a = if sl.cur == 0 { "INF".to_string() } else { sl.ammo[sl.cur].to_string() };
                    format!("[{}] {}  {}", sl.cur + 1, WEAPON_NAMES[sl.cur], a)
                }
            }
            HudText::Chapter => text.0 = story.chapter.clone(),
            HudText::Objective => text.0 = story.objective.clone(),
            HudText::Compass => {
                text.0 = match story.target {
                    Some(tp) => {
                        let rel = tp - sl.pos;
                        let d = Vec2::new(rel.x, rel.z).length();
                        let fwd = Vec2::new(-sl.rig.yaw.sin(), -sl.rig.yaw.cos());
                        let right = Vec2::new(sl.rig.yaw.cos(), -sl.rig.yaw.sin());
                        let r2 = Vec2::new(rel.x, rel.z).normalize_or_zero();
                        let ang = r2.dot(right).atan2(r2.dot(fwd));
                        let dir = if ang.abs() < 0.3 { "AHEAD" } else if ang.abs() > 2.5 { "BEHIND" } else if ang > 0.0 { if ang > 1.3 { "HARD RIGHT >>" } else { "RIGHT >" } } else if ang < -1.3 { "<< HARD LEFT" } else { "< LEFT" };
                        format!("{:>5} m   {}", d as i32, dir)
                    }
                    None => String::new(),
                }
            }
            HudText::Score => text.0 = format!("SCORE {}   KILLS {}", stats.score, stats.kills),
            HudText::Fps => text.0 = if cv.show_fps && slot == 0 { format!("{:.0} fps", *fps) } else { String::new() },
            HudText::DlgWho => {
                if let Some((w, _, _, _)) = &story.cur {
                    text.0 = w.clone();
                    color.0 = match w.as_str() {
                        "ORACLE" => CYAN,
                        "WARDEN" => Color::srgb(1.0, 0.25, 0.2),
                        _ => ORANGE,
                    };
                }
            }
            HudText::DlgText => {
                if let Some((_, tx, el, _)) = &story.cur {
                    let n = ((el * 55.0) as usize).min(tx.chars().count());
                    text.0 = tx.chars().take(n).collect();
                }
            }
            HudText::Toast(i) => {
                text.0 = toasts.0.get(*i).map(|t| t.0.clone()).unwrap_or_default();
                color.0 = Color::srgba(1.0, 1.0, 1.0, toasts.0.get(*i).map(|t| t.1.min(1.0)).unwrap_or(0.0));
            }
            HudText::Boss => text.0 = if bossv.is_some() { "THE WARDEN".into() } else { String::new() },
            HudText::Hint => {
                let key = if slot == 0 { "F" } else { "Y" };
                let hackkey = if slot == 0 { "E" } else { "X" };
                let mut h = String::new();
                if sl.dead_t > 0.0 {
                    h = format!("REBOOTING IN {:.0}", sl.dead_t.ceil());
                } else {
                    if !sl.in_car && sl.car_pos != Vec3::ZERO && sl.car_pos.distance(sl.pos) < 6.0 {
                        h = format!("[{}] ENTER CAR", key);
                    }
                    if let Some(tp) = story.terminal {
                        if !sl.in_car && tp.distance(sl.pos) < 8.0 && story.step == crate::story::Step::Hack {
                            h = format!("[HOLD {}] HACK TERMINAL", hackkey);
                        }
                    }
                }
                text.0 = h;
            }
            HudText::LoadStatus => {
                let i = ((loading.t / 1.1) as usize) % STATUS.len();
                let s = STATUS[i];
                let n = (((loading.t % 1.1) * 40.0) as usize).min(s.len());
                let cur = if (loading.t * 4.0) as i32 % 2 == 0 { "_" } else { " " };
                text.0 = format!("> {}{}", &s[..n], cur);
            }
            HudText::LoadPct => text.0 = format!("[ {:>3}% ]   synth {}/7   islands {}", (loading.progress * 100.0) as i32, (audio::audio_progress() * 7.0) as i32, "streaming"),
            HudText::LoadTip => text.0 = TIPS[((loading.t / 4.0) as usize) % TIPS.len()].into(),
            HudText::Prompt => color.0 = Color::srgba(1.0, 1.0, 1.0, 0.35 + 0.65 * blink),
            HudText::IntroLog => {
                const LOG: [&str; 6] = [
                    "XZ.EXE  v0.1  -- boot sequence",
                    "[ OK ] mounting /dev/null",
                    "[ OK ] allocating neon",
                    "[ !! ] process table: NOT FOUND",
                    "[ .. ] waking XZ ...",
                    "[ OK ] you are inside the machine",
                ];
                let n = ((intro.t / 0.7) as usize + 1).min(LOG.len());
                text.0 = LOG[..n].join("\n");
            }
            HudText::EndText => {
                text.0 = if story.ending == 0 {
                    "[1]  PULL THE PLUG - escape the machine\n[2]  STAY - become the new kernel".into()
                } else {
                    format!("{}\n\nSCORE {}   KILLS {}\n\nENTER - continue in free roam", ending_text(story.ending), stats.score, stats.kills)
                }
            }
        }
    }
}

fn hud_bars(
    pl: Res<Pl>,
    story: Res<Story>,
    car: Query<(&Vehicle, &PlayerCar)>,
    boss: Query<&Enemy>,
    loading: Res<Loading>,
    mut q: Query<(&mut Node, &Bar, Option<&HudSlot>), Without<BarBox>>,
    mut boxes: Query<(&mut Node, &BarBox, Option<&HudSlot>), Without<Bar>>,
) {
    let mut boost = [0.0f32; MAXP];
    for (v, p) in &car {
        boost[p.0.min(MAXP - 1)] = v.boost_fuel;
    }
    let bossv = boss.iter().find(|e| e.kind == EnemyKind::Boss);
    let hack_on = story.step == crate::story::Step::Hack;
    let calc = |b: &Bar, slot: usize| {
        let sl = &pl.s[slot];
        match b {
            Bar::Hp => (sl.hp / sl.max_hp, true),
            Bar::Boost => (boost[slot], sl.in_car),
            Bar::Hack => (story.hack, hack_on),
            Bar::Boss => (bossv.map(|e| e.hp / e.max).unwrap_or(0.0), bossv.is_some()),
            Bar::Load => (loading.progress, true),
        }
    };
    for (mut n, b, hs) in &mut q {
        n.width = Val::Percent((calc(b, hs.map_or(0, |h| h.0.min(MAXP - 1))).0 * 100.0).clamp(0.0, 100.0));
    }
    for (mut n, b, hs) in &mut boxes {
        n.display = if calc(&b.0, hs.map_or(0, |h| h.0.min(MAXP - 1))).1 { Display::Flex } else { Display::None };
    }
}

fn radar(pl: Res<Pl>, enemies: Query<(&Transform, &Enemy)>, mut dots: Query<(&RadarDot, &mut Node, Option<&HudSlot>)>) {
    let range = 220.0;
    let mut lists: [Vec<Vec2>; MAXP] = [vec![], vec![]];
    for i in 0..MAXP {
        let sl = &pl.s[i];
        let fwd = Vec2::new(-sl.rig.yaw.sin(), -sl.rig.yaw.cos());
        let right = Vec2::new(sl.rig.yaw.cos(), -sl.rig.yaw.sin());
        for (t, _) in &enemies {
            let rel = Vec2::new(t.translation.x - sl.pos.x, t.translation.z - sl.pos.z);
            let p = Vec2::new(rel.dot(right), rel.dot(fwd)) / range;
            let l = p.length();
            lists[i].push(if l > 1.0 { p / l } else { p });
        }
    }
    for (d, mut n, hs) in &mut dots {
        let slot = hs.map_or(0, |h| h.0.min(MAXP - 1));
        if let Some(p) = lists[slot].get(d.0) {
            n.display = Display::Flex;
            n.left = Val::Px(70.0 + p.x * 66.0);
            n.top = Val::Px(70.0 - p.y * 66.0);
        } else {
            n.display = Display::None;
        }
    }
}

fn overlays(pl: Res<Pl>, mut v: Query<(&mut BackgroundColor, Option<&HudSlot>), With<Vignette>>) {
    for (mut b, hs) in &mut v {
        let sl = &pl.s[hs.map_or(0, |h| h.0.min(MAXP - 1))];
        let down = if sl.dead_t > 0.0 { 0.35 } else { 0.0 };
        b.0 = Color::srgba(1.0, 0.0, 0.1, (sl.hurt * 0.55 + down).min(0.55));
    }
}

/// Tag every descendant of a HUD root with its player slot (so one HUD layout serves both players).
fn tag_hud(mut commands: Commands, roots: Query<(Entity, &HudRoot), Without<Tagged>>, children: Query<&Children>) {
    for (e, r) in &roots {
        let mut any = false;
        for d in children.iter_descendants(e) {
            commands.entity(d).insert(HudSlot(r.0));
            any = true;
        }
        if any {
            commands.entity(e).insert(Tagged);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn fx_ui_update(
    time: Res<Time<Real>>,
    mut intro: ResMut<Intro>,
    state: Res<State<AppState>>,
    mut tm: ResMut<Assets<TunnelMat>>,
    mut cm: ResMut<Assets<CityMat>>,
    q_t: Query<(&MaterialNode<TunnelMat>, Option<&IntroRoot>)>,
    q_c: Query<&MaterialNode<CityMat>>,
) {
    let dt = time.delta_secs();
    if intro.active {
        intro.t += dt;
        if intro.t > 5.2 || *state.get() != AppState::Playing {
            intro.active = false;
        }
    }
    let t = time.elapsed_secs();
    for (m, is_intro) in &q_t {
        if let Some(mm) = tm.get_mut(&m.0) {
            if is_intro.is_some() {
                let alpha = if intro.t < 4.0 { 1.0 } else { (1.0 - (intro.t - 4.0) / 1.2).clamp(0.0, 1.0) };
                mm.p = Vec4::new(t, alpha, 2.2, 0.0);
            } else {
                mm.p = Vec4::new(t, 0.88, 0.8, 1.0);
            }
        }
    }
    for m in &q_c {
        if let Some(mm) = cm.get_mut(&m.0) {
            mm.p = Vec4::new(t, 1.0, 0.0, 0.0);
        }
    }
}

/// Animated gradient text (palette scrolls along the letters + a white shimmer sweep).
/// Bevy UI text cannot take a custom material, so the "gradient text shader" runs per glyph on the CPU.
fn grad_letters(time: Res<Time<Real>>, mut q: Query<(&GradLetter, &mut TextColor)>) {
    let t = time.elapsed_secs();
    for (g, mut c) in &mut q {
        let x = g.i as f32 / (g.n.max(2) - 1) as f32;
        let base = grad_wrap(x * 0.8 - t * 0.12);
        let sweep = (-(((t * 0.35).fract() * 1.6 - 0.3) - x).powi(2) * 40.0).exp();
        c.0 = base.mix(&Color::WHITE, sweep * 0.7);
    }
}
