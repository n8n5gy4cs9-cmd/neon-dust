//! Pure console command registry + parser (nordrep-style): `execute(line, &Snapshot) -> Output`.
//! No Bevy types so the rules stay testable. Actions are applied in console.rs.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineKind {
    Info,
    Ok,
    Err,
    Cmd,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Flag {
    God,
    Noclip,
    InfAmmo,
    OneHit,
    Peaceful,
    InfBoost,
    LowGrav,
}
impl Flag {
    pub fn by_name(n: &str) -> Option<Flag> {
        Some(match n {
            "god" | "godmode" => Flag::God,
            "noclip" | "fly" | "ghost" => Flag::Noclip,
            "infammo" | "ammo" => Flag::InfAmmo,
            "onehit" | "ohk" => Flag::OneHit,
            "peaceful" | "calm" => Flag::Peaceful,
            "infboost" | "nitro" => Flag::InfBoost,
            "lowgrav" | "moon" => Flag::LowGrav,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    CheatsOn(bool),
    Toggle(Flag, Option<bool>),
    Speed(f32),
    Heal,
    Kill,
    KillAll,
    GiveAll,
    GiveAmmo,
    GiveBoost,
    GiveWeapon(usize),
    Tp(f32, f32),
    GotoStart,
    GotoTarget,
    GotoCar,
    CarHere,
    Drive,
    Flip,
    Spawn(String, u32),
    Explode,
    Score(u32),
    Mission(String),
    NextMission,
    Regen(Option<u32>),
    Cvar(String, f32),
    ShowFps(Option<bool>),
    Timescale(f32),
    Track(Option<usize>),
    Screenshot,
    Quit,
    Clear,
    Wave,
}

#[derive(Default)]
pub struct Output {
    pub lines: Vec<(LineKind, String)>,
    pub actions: Vec<Action>,
}
impl Output {
    fn info(&mut self, s: impl Into<String>) {
        self.lines.push((LineKind::Info, s.into()));
    }
    fn ok(&mut self, s: impl Into<String>) {
        self.lines.push((LineKind::Ok, s.into()));
    }
    fn err(&mut self, s: impl Into<String>) {
        self.lines.push((LineKind::Err, s.into()));
    }
}

#[derive(Clone, Default)]
pub struct Snapshot {
    pub cheats_on: bool,
    pub pos: [f32; 3],
    pub hp: f32,
    pub kills: u32,
    pub score: u32,
    pub seed: u32,
    pub step: String,
    pub in_car: bool,
    pub enemies: usize,
    pub speed_kmh: f32,
    pub history: Vec<String>,
    pub cvars: String,
    pub version: &'static str,
}

pub struct CmdDef {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub usage: &'static str,
    pub help: &'static str,
    pub cheat: bool,
    pub cat: &'static str,
}

macro_rules! c {
    ($n:expr, [$($a:expr),*], $u:expr, $h:expr, $ch:expr, $cat:expr) => {
        CmdDef { name: $n, aliases: &[$($a),*], usage: $u, help: $h, cheat: $ch, cat: $cat }
    };
}

pub const CMDS: &[CmdDef] = &[
    c!("help", ["?", "h"], "help [command]", "List commands or explain one.", false, "General"),
    c!("commands", ["cmds"], "commands", "Print every command name.", false, "General"),
    c!("clear", ["cls"], "clear", "Clear the console log.", false, "General"),
    c!("echo", ["say"], "echo <text>", "Print text.", false, "General"),
    c!("history", [], "history", "Show previous commands (Up/Down also cycle).", false, "General"),
    c!("version", ["ver"], "version", "Show game version.", false, "General"),
    c!("quit", ["exit"], "quit", "Quit the game.", false, "General"),
    c!("cheats", [], "cheats [on|off]", "Enable cheat commands (marks the run as cheated).", false, "General"),
    c!("pos", ["where"], "pos", "Print your position.", false, "General"),
    c!("status", ["stat"], "status", "HP, kills, score, mission, enemies.", false, "General"),
    c!("missions", [], "missions", "List mission step names.", false, "General"),
    c!("seed", [], "seed [n]", "Show world seed; with n (cheat) regenerate the world.", false, "General"),
    c!("fps", [], "fps [on|off]", "Show the FPS counter.", false, "Video"),
    c!("fov", [], "fov <40-120>", "Field of view.", false, "Video"),
    c!("bloom", [], "bloom <0-1.5>", "Bloom intensity.", false, "Video"),
    c!("fog", [], "fog <200-4000>", "Fog visibility distance (m).", false, "Video"),
    c!("viewdist", ["vd"], "viewdist <2-10>", "Terrain view radius in chunks (128 m each).", false, "Video"),
    c!("screenshot", ["shot"], "screenshot", "Save screenshot_N.png.", false, "Video"),
    c!("timescale", ["ts"], "timescale <0.1-4>", "Game speed.", false, "Game"),
    c!("musicvol", [], "musicvol <0-1>", "Music volume.", false, "Audio"),
    c!("sfxvol", [], "sfxvol <0-1>", "SFX volume.", false, "Audio"),
    c!("track", [], "track <0-3|auto>", "Force music: 0 menu 1 drive 2 combat 3 port.", false, "Audio"),
    c!("sens", [], "sens <0.0005-0.01>", "Mouse sensitivity.", false, "Game"),
    c!("god", ["godmode"], "god [on|off]", "Cannot be hurt.", true, "Cheats"),
    c!("noclip", ["fly", "ghost"], "noclip [on|off]", "Fly through everything (on foot): SPACE up, CTRL down.", true, "Cheats"),
    c!("infammo", ["ammo"], "infammo [on|off]", "Infinite ammo.", true, "Cheats"),
    c!("onehit", ["ohk"], "onehit [on|off]", "Every hit kills.", true, "Cheats"),
    c!("peaceful", ["calm"], "peaceful [on|off]", "Enemies stop shooting & spawning.", true, "Cheats"),
    c!("infboost", ["nitro"], "infboost [on|off]", "Unlimited boost.", true, "Cheats"),
    c!("lowgrav", ["moon"], "lowgrav [on|off]", "Low gravity.", true, "Cheats"),
    c!("speed", [], "speed <0.2-5>", "Movement/engine power multiplier.", true, "Cheats"),
    c!("heal", [], "heal", "Full integrity + boost.", true, "Cheats"),
    c!("kill", ["suicide"], "kill", "Kernel panic yourself.", true, "Cheats"),
    c!("killall", [], "killall", "Destroy every enemy.", true, "Cheats"),
    c!("give", [], "give <all|ammo|boost|1-4>", "Give weapons / ammo / boost.", true, "Cheats"),
    c!("tp", ["teleport"], "tp <x> <z>", "Teleport to world coordinates.", true, "Cheats"),
    c!("goto", [], "goto <start|target|car>", "Teleport to start / mission beacon / your car.", true, "Cheats"),
    c!("car", [], "car", "Bring the car to you.", true, "Cheats"),
    c!("drive", [], "drive", "Jump into the car.", true, "Cheats"),
    c!("flip", [], "flip", "Put the car back on its wheels.", true, "Cheats"),
    c!("spawn", [], "spawn <drone|sentry|car|boss> [n]", "Spawn enemies in front of you.", true, "Cheats"),
    c!("wave", [], "wave", "Spawn a mixed enemy wave.", true, "Cheats"),
    c!("explode", ["boom"], "explode", "Explosion at the crosshair.", true, "Cheats"),
    c!("score", [], "score <n>", "Add score.", true, "Cheats"),
    c!("mission", [], "mission <name>", "Jump to a mission step (see `missions`).", true, "Cheats"),
    c!("next", ["skip"], "next", "Skip to the next mission step.", true, "Cheats"),
    c!("regen", [], "regen [seed]", "Rebuild the world (optional new seed).", true, "Cheats"),
];

pub fn find(name: &str) -> Option<&'static CmdDef> {
    let n = name.to_lowercase();
    CMDS.iter().find(|c| c.name == n || c.aliases.contains(&n.as_str()))
}

const ENEMIES: [&str; 4] = ["drone", "sentry", "car", "boss"];
const STEPS: [&str; 12] = ["wake", "entercar", "boot", "flush", "gate", "firewall", "hack", "pursuit", "port", "warden", "choice", "free"];

pub fn mission_names() -> &'static [&'static str] {
    &STEPS
}

pub fn complete(line: &str) -> Vec<String> {
    let parts: Vec<&str> = line.split(' ').collect();
    let last = parts.last().copied().unwrap_or("").to_lowercase();
    if parts.len() <= 1 {
        let mut v: Vec<String> = CMDS.iter().filter(|c| c.name.starts_with(&last)).map(|c| c.name.to_string()).collect();
        v.sort();
        return v;
    }
    let cmd = find(parts[0]).map(|c| c.name).unwrap_or("");
    let pool: Vec<&str> = match cmd {
        "spawn" => ENEMIES.to_vec(),
        "mission" => STEPS.to_vec(),
        "goto" => vec!["start", "target", "car"],
        "give" => vec!["all", "ammo", "boost", "1", "2", "3", "4"],
        "track" => vec!["auto", "0", "1", "2", "3"],
        "cheats" | "fps" | "god" | "noclip" | "infammo" | "onehit" | "peaceful" | "infboost" | "lowgrav" => vec!["on", "off"],
        "help" => CMDS.iter().map(|c| c.name).collect(),
        _ => vec![],
    };
    pool.into_iter().filter(|p| p.starts_with(&last)).map(|p| p.to_string()).collect()
}

fn num(a: &[&str], i: usize) -> Option<f32> {
    a.get(i).and_then(|s| s.parse::<f32>().ok())
}
fn onoff(a: &[&str]) -> Option<Option<bool>> {
    match a.first().map(|s| s.to_lowercase()).as_deref() {
        None => Some(None),
        Some("on") | Some("1") | Some("true") => Some(Some(true)),
        Some("off") | Some("0") | Some("false") => Some(Some(false)),
        _ => None,
    }
}

pub fn execute(line: &str, snap: &Snapshot) -> Output {
    let mut o = Output::default();
    let line = line.trim();
    if line.is_empty() {
        return o;
    }
    let toks: Vec<&str> = line.split_whitespace().collect();
    let Some(def) = find(toks[0]) else {
        o.err(format!("unknown command '{}'. Try `help`.", toks[0]));
        return o;
    };
    let a = &toks[1..];
    if def.cheat && !snap.cheats_on {
        o.err(format!("'{}' is a cheat. Type `cheats on` first.", def.name));
        return o;
    }
    match def.name {
        "help" => {
            if let Some(n) = a.first() {
                match find(n) {
                    Some(c) => {
                        o.info(format!("{}   {}", c.usage, if c.cheat { "[cheat]" } else { "" }));
                        o.info(format!("  {}", c.help));
                        if !c.aliases.is_empty() {
                            o.info(format!("  aliases: {}", c.aliases.join(", ")));
                        }
                    }
                    None => o.err(format!("no such command '{n}'")),
                }
            } else {
                o.info("NEON DUST CONSOLE   (Tab completes, Up/Down history, ` closes)");
                o.info("Cheat commands need `cheats on`. Try: cheats on / god / give all / spawn drone 5");
                let mut cat = "";
                for c in CMDS {
                    if c.cat != cat {
                        cat = c.cat;
                        o.info(format!("== {} ==", cat));
                    }
                    o.info(format!("  {:<34} {}{}", c.usage, c.help, if c.cheat { " [cheat]" } else { "" }));
                }
            }
        }
        "commands" => {
            let names: Vec<&str> = CMDS.iter().map(|c| c.name).collect();
            o.info(names.join("  "));
        }
        "clear" => o.actions.push(Action::Clear),
        "echo" => o.info(a.join(" ")),
        "history" => {
            for (i, h) in snap.history.iter().enumerate().rev().take(15).rev() {
                o.info(format!("{:>3}  {}", i + 1, h));
            }
        }
        "version" => o.info(format!("Neon Dust {}", snap.version)),
        "quit" => o.actions.push(Action::Quit),
        "cheats" => match onoff(a) {
            Some(Some(b)) => {
                o.actions.push(Action::CheatsOn(b));
                o.ok(format!("cheats {}", if b { "ON (run marked as cheated)" } else { "OFF" }));
            }
            Some(None) => o.info(format!("cheats are {}", if snap.cheats_on { "ON" } else { "OFF" })),
            None => o.err("usage: cheats [on|off]"),
        },
        "pos" => o.info(format!("x {:.1}  y {:.1}  z {:.1}", snap.pos[0], snap.pos[1], snap.pos[2])),
        "status" => {
            o.info(format!(
                "hp {:.0}  kills {}  score {}  mission {}  enemies {}  {} {:.0} km/h",
                snap.hp, snap.kills, snap.score, snap.step, snap.enemies, if snap.in_car { "car" } else { "foot" }, snap.speed_kmh
            ));
            o.info(snap.cvars.clone());
        }
        "missions" => o.info(STEPS.join(" -> ")),
        "seed" => match a.first().and_then(|s| s.parse::<u32>().ok()) {
            Some(n) => {
                if snap.cheats_on {
                    o.actions.push(Action::Regen(Some(n)));
                    o.ok(format!("regenerating world with seed {n}"));
                } else {
                    o.err("changing the seed is a cheat. `cheats on` first.");
                }
            }
            None => o.info(format!("seed {}", snap.seed)),
        },
        "fps" => match onoff(a) {
            Some(v) => {
                o.actions.push(Action::ShowFps(v));
                o.ok("ok");
            }
            None => o.err("usage: fps [on|off]"),
        },
        "fov" | "bloom" | "fog" | "viewdist" | "musicvol" | "sfxvol" | "sens" => {
            let (lo, hi) = match def.name {
                "fov" => (40.0, 120.0),
                "bloom" => (0.0, 1.5),
                "fog" => (200.0, 4000.0),
                "viewdist" => (2.0, 10.0),
                "sens" => (0.0005, 0.01),
                _ => (0.0, 1.0),
            };
            match num(a, 0) {
                Some(v) => {
                    let v = v.clamp(lo, hi);
                    o.actions.push(Action::Cvar(def.name.to_string(), v));
                    o.ok(format!("{} = {}", def.name, v));
                }
                None => o.err(format!("usage: {}  (range {lo}-{hi})", def.usage)),
            }
        }
        "screenshot" => {
            o.actions.push(Action::Screenshot);
            o.ok("screenshot saved");
        }
        "timescale" => match num(a, 0) {
            Some(v) => {
                o.actions.push(Action::Timescale(v.clamp(0.1, 4.0)));
                o.ok(format!("timescale {}", v.clamp(0.1, 4.0)));
            }
            None => o.err("usage: timescale <0.1-4>"),
        },
        "track" => match a.first() {
            Some(&"auto") | None => {
                o.actions.push(Action::Track(None));
                o.ok("music: auto");
            }
            Some(s) => match s.parse::<usize>() {
                Ok(n) if n < 4 => {
                    o.actions.push(Action::Track(Some(n)));
                    o.ok(format!("music track {n}"));
                }
                _ => o.err("usage: track <0-3|auto>"),
            },
        },
        "god" | "noclip" | "infammo" | "onehit" | "peaceful" | "infboost" | "lowgrav" => {
            let f = Flag::by_name(def.name).unwrap();
            match onoff(a) {
                Some(v) => {
                    o.actions.push(Action::Toggle(f, v));
                    o.ok(format!("{} {}", def.name, match v { Some(true) => "on", Some(false) => "off", None => "toggled" }));
                }
                None => o.err(format!("usage: {}", def.usage)),
            }
        }
        "speed" => match num(a, 0) {
            Some(v) => {
                o.actions.push(Action::Speed(v.clamp(0.2, 5.0)));
                o.ok(format!("speed x{}", v.clamp(0.2, 5.0)));
            }
            None => o.err("usage: speed <0.2-5>"),
        },
        "heal" => {
            o.actions.push(Action::Heal);
            o.ok("healed");
        }
        "kill" => o.actions.push(Action::Kill),
        "killall" => {
            o.actions.push(Action::KillAll);
            o.ok("all enemies destroyed");
        }
        "give" => match a.first().map(|s| s.to_lowercase()).as_deref() {
            Some("all") => {
                o.actions.push(Action::GiveAll);
                o.ok("all weapons + ammo");
            }
            Some("ammo") => {
                o.actions.push(Action::GiveAmmo);
                o.ok("ammo refilled");
            }
            Some("boost") => {
                o.actions.push(Action::GiveBoost);
                o.ok("boost refilled");
            }
            Some(s) => match s.parse::<usize>() {
                Ok(n) if (1..=4).contains(&n) => {
                    o.actions.push(Action::GiveWeapon(n - 1));
                    o.ok(format!("weapon {n} given"));
                }
                _ => o.err("usage: give <all|ammo|boost|1-4>"),
            },
            None => o.err("usage: give <all|ammo|boost|1-4>"),
        },
        "tp" => match (num(a, 0), num(a, 1)) {
            (Some(x), Some(z)) => {
                o.actions.push(Action::Tp(x, z));
                o.ok(format!("teleport to {x:.0}, {z:.0}"));
            }
            _ => o.err("usage: tp <x> <z>"),
        },
        "goto" => match a.first().copied() {
            Some("start") => o.actions.push(Action::GotoStart),
            Some("target") => o.actions.push(Action::GotoTarget),
            Some("car") => o.actions.push(Action::GotoCar),
            _ => o.err("usage: goto <start|target|car>"),
        },
        "car" => {
            o.actions.push(Action::CarHere);
            o.ok("car delivered");
        }
        "drive" => {
            o.actions.push(Action::Drive);
            o.ok("driving");
        }
        "flip" => {
            o.actions.push(Action::Flip);
            o.ok("flipped");
        }
        "spawn" => match a.first().map(|s| s.to_lowercase()) {
            Some(k) if ENEMIES.contains(&k.as_str()) => {
                let n = num(a, 1).map(|v| v as u32).unwrap_or(1).clamp(1, 30);
                o.actions.push(Action::Spawn(k.clone(), n));
                o.ok(format!("spawned {n} {k}"));
            }
            _ => o.err("usage: spawn <drone|sentry|car|boss> [n]"),
        },
        "wave" => {
            o.actions.push(Action::Wave);
            o.ok("incoming!");
        }
        "explode" => o.actions.push(Action::Explode),
        "score" => match num(a, 0) {
            Some(v) => o.actions.push(Action::Score(v.max(0.0) as u32)),
            None => o.err("usage: score <n>"),
        },
        "mission" => match a.first().map(|s| s.to_lowercase()) {
            Some(m) if STEPS.contains(&m.as_str()) => {
                o.actions.push(Action::Mission(m.clone()));
                o.ok(format!("mission -> {m}"));
            }
            _ => o.err(format!("usage: mission <{}>", STEPS.join("|"))),
        },
        "next" => {
            o.actions.push(Action::NextMission);
            o.ok("skipping to next mission step");
        }
        "regen" => {
            o.actions.push(Action::Regen(a.first().and_then(|s| s.parse().ok())));
            o.ok("regenerating world");
        }
        _ => o.err("not implemented"),
    }
    o
}

pub fn render_commands_txt() -> String {
    let mut s = String::from("NEON DUST CONSOLE COMMANDS  (generated from src/commands.rs; run `neon-dust --docs`)\nOpen with ` (backtick). Tab completes, Up/Down = history.\nCommands marked [cheat] need `cheats on` first.\n\n");
    let mut cat = "";
    for c in CMDS {
        if c.cat != cat {
            cat = c.cat;
            s.push_str(&format!("== {} ==\n", cat));
        }
        s.push_str(&format!("  {}{}\n      {}", c.usage, if c.cheat { "   [cheat]" } else { "" }, c.help));
        if !c.aliases.is_empty() {
            s.push_str(&format!("   (aliases: {})", c.aliases.join(", ")));
        }
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cheat_gate() {
        let mut s = Snapshot::default();
        assert!(execute("god", &s).lines[0].0 == LineKind::Err);
        s.cheats_on = true;
        assert_eq!(execute("god on", &s).actions, vec![Action::Toggle(Flag::God, Some(true))]);
    }
    #[test]
    fn dump_commands_txt() {
        std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/commands.txt"), render_commands_txt()).unwrap();
    }
    #[test]
    fn complete_works() {
        assert!(complete("sp").contains(&"spawn".to_string()));
        assert!(complete("spawn d").contains(&"drone".to_string()));
    }
}
