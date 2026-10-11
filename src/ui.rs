//! UI: loading splash (kaleidoscope shader), title menu, HUD, radar, overlays.
use crate::audio;
use crate::camera::CamRig;
use crate::enemies::{Enemy, EnemyKind, Tgt};
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum G {
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
        app.add_plugins(UiMaterialPlugin::<LoadMat>::default())
            .add_systems(Startup, setup_ui)
            .add_systems(Update, (loading_update, groups, hud_texts, hud_bars, radar, overlays));
    }
}

fn t(s: &str, size: f32, c: Color) -> impl Bundle {
    (Text::new(s), TextFont { font_size: size, ..default() }, TextColor(c))
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

fn bar(p: &mut ChildSpawnerCommands, kind: Bar, w: f32, h: f32, c: Color) {
    p.spawn((Node { width: Val::Px(w), height: Val::Px(h), border: UiRect::all(Val::Px(1.0)), ..default() }, BorderColor(Color::srgba(1.0, 1.0, 1.0, 0.35)), BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5))))
        .with_children(|b| {
            b.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(c), kind));
        });
}

fn title_row(p: &mut ChildSpawnerCommands, s: &str, size: f32) {
    p.spawn(Node { flex_direction: FlexDirection::Row, ..default() }).with_children(|r| {
        let n = s.chars().count().max(1);
        for (i, ch) in s.chars().enumerate() {
            r.spawn((
                Text::new(ch.to_string()),
                TextFont { font_size: size, ..default() },
                TextColor(grad(i as f32 / (n - 1).max(1) as f32)),
                TextShadow { offset: Vec2::new(size * 0.04, size * 0.04), color: Color::srgba(0.8, 0.0, 0.6, 0.9) },
            ));
        }
    });
}

fn setup_ui(mut commands: Commands, mut mats: ResMut<Assets<LoadMat>>) {
    // ---------------- HUD ----------------
    commands
        .spawn((Node { flex_direction: FlexDirection::Column, ..full() }, Group(G::Hud), Visibility::default()))
        .with_children(|root| {
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

    // ---------------- Menu ----------------
    commands
        .spawn((Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, row_gap: Val::Px(10.0), ..full() }, Group(G::Menu), Visibility::default()))
        .with_children(|p| {
            p.spawn(Node { margin: UiRect::bottom(Val::Px(0.0)), ..default() }).with_children(|r| title_row(r, "NEON DUST", 128.0));
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
            title_row(p, "PAUSED", 80.0);
            p.spawn(t("ESC / ENTER  resume", 22.0, Color::WHITE));
            p.spawn(t("` open console (cheats on  ->  god, noclip, give all ...)", 16.0, DIM));
        });

    // ---------------- Dead ----------------
    commands
        .spawn((Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, row_gap: Val::Px(14.0), ..full() }, BackgroundColor(Color::srgba(0.15, 0.0, 0.02, 0.6)), Group(G::Dead), Visibility::default()))
        .with_children(|p| {
            p.spawn((t("KERNEL PANIC", 96.0, Color::srgb(1.0, 0.15, 0.2)), TextShadow { offset: Vec2::new(4.0, 4.0), color: Color::srgba(0.4, 0.0, 0.4, 0.9) }));
            p.spawn(t("process 0xXZ terminated unexpectedly", 20.0, DIM));
            p.spawn(t("ENTER  reboot from last checkpoint", 26.0, Color::WHITE));
        });

    // ---------------- Ending ----------------
    commands
        .spawn((Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, row_gap: Val::Px(16.0), ..full() }, BackgroundColor(Color::srgba(0.02, 0.0, 0.1, 0.8)), Group(G::Ending), Visibility::default()))
        .with_children(|p| {
            title_row(p, "THE PORT IS OPEN", 70.0);
            p.spawn((Node { max_width: Val::Px(820.0), ..default() }, t("", 22.0, Color::WHITE), HudText::EndText));
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
            title_row(p, "NEON DUST", 96.0);
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
    mut q: Query<(&Group, &mut Node)>,
) {
    let st = *state.get();
    for (g, mut n) in &mut q {
        let show = match g.0 {
            G::Loading => !loading.done,
            G::Menu => st == AppState::Menu && loading.done,
            G::Hud => matches!(st, AppState::Playing | AppState::Paused | AppState::Dead) && loading.done,
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
    ctl: Res<Control>,
    story: Res<Story>,
    tgt: Res<Tgt>,
    rig: Res<CamRig>,
    cv: Res<Cvars>,
    loading: Res<Loading>,
    mut toasts: ResMut<Toasts>,
    car: Query<(&Transform, &Vehicle), With<PlayerCar>>,
    boss: Query<&Enemy>,
    mut fps: Local<f32>,
    mut q: Query<(&mut Text, &mut TextColor, &HudText)>,
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
    let (speed, car_pos) = car.single().map(|(t, v)| (v.speed, t.translation)).unwrap_or((0.0, Vec3::ZERO));
    let bossv = boss.iter().find(|e| e.kind == EnemyKind::Boss);
    if let Ok(mut n) = dlg.single_mut() {
        n.display = if story.cur.is_some() { Display::Flex } else { Display::None };
    }
    let blink = (time.elapsed_secs() * 3.0).sin() * 0.5 + 0.5;
    for (mut text, mut color, kind) in &mut q {
        match kind {
            HudText::Speed => text.0 = if ctl.in_car { format!("{:>3} KM/H", (speed * 3.6) as i32) } else { "ON FOOT".into() },
            HudText::Hp => text.0 = format!("INTEGRITY {:>3}", stats.hp.max(0.0) as i32),
            HudText::Ammo => {
                text.0 = if ctl.in_car {
                    format!("CANNONS  ROCKETS x{}", stats.ammo[3])
                } else {
                    let a = if stats.cur == 0 { "INF".to_string() } else { stats.ammo[stats.cur].to_string() };
                    format!("[{}] {}  {}", stats.cur + 1, WEAPON_NAMES[stats.cur], a)
                }
            }
            HudText::Chapter => text.0 = story.chapter.clone(),
            HudText::Objective => text.0 = story.objective.clone(),
            HudText::Compass => {
                text.0 = match story.target {
                    Some(tp) => {
                        let rel = tp - tgt.pos;
                        let d = Vec2::new(rel.x, rel.z).length();
                        let fwd = Vec2::new(-rig.yaw.sin(), -rig.yaw.cos());
                        let right = Vec2::new(rig.yaw.cos(), -rig.yaw.sin());
                        let r2 = Vec2::new(rel.x, rel.z).normalize_or_zero();
                        let ang = r2.dot(right).atan2(r2.dot(fwd));
                        let dir = if ang.abs() < 0.3 { "AHEAD" } else if ang.abs() > 2.5 { "BEHIND" } else if ang > 0.0 { if ang > 1.3 { "HARD RIGHT >>" } else { "RIGHT >" } } else if ang < -1.3 { "<< HARD LEFT" } else { "< LEFT" };
                        format!("{:>5} m   {}", d as i32, dir)
                    }
                    None => String::new(),
                }
            }
            HudText::Score => text.0 = format!("SCORE {}   KILLS {}", stats.score, stats.kills),
            HudText::Fps => text.0 = if cv.show_fps { format!("{:.0} fps", *fps) } else { String::new() },
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
                let mut h = String::new();
                if !ctl.in_car && car_pos != Vec3::ZERO && car_pos.distance(tgt.pos) < 6.0 {
                    h = "[F] ENTER CAR".into();
                }
                if let Some(tp) = story.terminal {
                    if !ctl.in_car && tp.distance(tgt.pos) < 8.0 && story.step == crate::story::Step::Hack {
                        h = "[HOLD E] HACK TERMINAL".into();
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
    stats: Res<Stats>,
    story: Res<Story>,
    ctl: Res<Control>,
    car: Query<&Vehicle, With<PlayerCar>>,
    boss: Query<&Enemy>,
    loading: Res<Loading>,
    mut q: Query<(&mut Node, &Bar)>,
    mut parents: Query<&mut Node, Without<Bar>>,
    child_of: Query<&ChildOf, With<Bar>>,
) {
    let bf = car.single().map(|v| v.boost_fuel).unwrap_or(0.0);
    let bossv = boss.iter().find(|e| e.kind == EnemyKind::Boss);
    let hack_on = story.step == crate::story::Step::Hack;
    let _ = (&mut parents, &child_of);
    for (mut n, b) in &mut q {
        let (frac, visible) = match b {
            Bar::Hp => (stats.hp / stats.max_hp, true),
            Bar::Boost => (bf, ctl.in_car),
            Bar::Hack => (story.hack, hack_on),
            Bar::Boss => (bossv.map(|e| e.hp / e.max).unwrap_or(0.0), bossv.is_some()),
            Bar::Load => (loading.progress, true),
        };
        n.width = Val::Percent((frac * 100.0).clamp(0.0, 100.0));
        n.display = if visible { Display::Flex } else { Display::None };
    }
}

fn radar(
    tgt: Res<Tgt>,
    rig: Res<CamRig>,
    enemies: Query<(&Transform, &Enemy)>,
    mut dots: Query<(&RadarDot, &mut Node)>,
) {
    let fwd = Vec2::new(-rig.yaw.sin(), -rig.yaw.cos());
    let right = Vec2::new(rig.yaw.cos(), -rig.yaw.sin());
    let range = 220.0;
    let mut list: Vec<Vec2> = vec![];
    for (t, _) in &enemies {
        let rel = Vec2::new(t.translation.x - tgt.pos.x, t.translation.z - tgt.pos.z);
        let p = Vec2::new(rel.dot(right), rel.dot(fwd)) / range;
        let l = p.length();
        list.push(if l > 1.0 { p / l } else { p });
    }
    for (d, mut n) in &mut dots {
        if let Some(p) = list.get(d.0) {
            n.display = Display::Flex;
            n.left = Val::Px(70.0 + p.x * 66.0);
            n.top = Val::Px(70.0 - p.y * 66.0);
        } else {
            n.display = Display::None;
        }
    }
}

fn overlays(stats: Res<Stats>, mut v: Query<&mut BackgroundColor, With<Vignette>>) {
    for mut b in &mut v {
        b.0 = Color::srgba(1.0, 0.0, 0.1, (stats.hurt * 0.55).min(0.5));
    }
}
