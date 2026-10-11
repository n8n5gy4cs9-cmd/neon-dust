# Neon Dust — Product Requirements

## Pitch
Fast-paced synthwave driving + shooting + on-foot run-and-gun in an endless procedural sky of floating neon islands.
You are **XZ**, a rogue process that woke up inside a computer. Drive, shoot, bail out, run, and escape through the Port.

## Story
- **Boot**: XZ wakes in `/dev/null`, a dead sector. A voice — **ORACLE**, the OS kernel — says: "You are not in the process table."
- **Act 1 – Sectors**: Cross the Floating Sectors (islands of cached memory) by car. Corrupted daemons (**Glitchers**) hunt you.
- **Act 2 – The Firewall**: Gates only open on foot: leave the car, run through terminals, shoot sentries, hack nodes.
- **Act 3 – The Port**: Reach the glowing USB Port on the horizon. ORACLE reveals it is the one that wiped you. Choose: kill it or escape.
- 5 missions (see `docs/story.md` in-game text in `src/story.rs`).

## Pillars
1. **Speed** – everything fast; boost, drift, rapid fire.
2. **Neon Drift look** – wireframe ridges, glow, bloom, fog, synthwave palette (cyan/magenta/violet/midnight/pink-orange).
3. **Everything generated in code** – meshes, textures, WGSL shaders, SFX, music. No external assets.
4. **Car ⇄ foot** – seamless enter/exit.
5. **Dev console** – full cheats/dev commands (nordrep-style).

## Features
### Driving (DustRacing2D → 3D)
- DustRacing2D car model: 2D force-based physics (engine force, drag, rolling resistance, lateral grip with sliding friction, handbrake, steering by speed). Ported with x,y → x,z plane and y as up; vertical added: gravity, ground following via heightfield, pitch/roll from terrain normal.
- Boost, drift, collision with props/enemies, surface handling (grass slower).
### Third person / shooting (openrw → Bevy)
- Third-person orbit camera, character controller (walk/run/jump), enter/exit vehicle (F), weapon switch, aim, hitscan + tracer.
- Car: forward guns + camera chase.
### Weapons
Pulse pistol, Plasma SMG, Rail shotgun, Rocket launcher. Tracer bullets, muzzle flash, explosions, fire, smoke, sparks.
### Enemies
Glitcher drones (fly), Sentry turrets, Pursuit cars. Health, drops.
### World
- Infinite chunked procedural terrain (fBm/ridged noise), floating islands with undersides, rivers, waterfalls, forests (bioluminescent), clouds, distant mountain silhouettes, glowing sun, grid terrain shader, wireframe ridge contours, reflective water, fog, bloom, color grading, dynamic shadows, floating particles.
### Audio (all synthesized)
- SFX: engine (speed-pitched loop), shots, explosions, pickups, UI.
- Procedural full synthwave soundtrack: bass arp, pads, lead, drums, multiple tracks/moods (menu, drive, combat, boss), rendered at startup.
### UI
Title, HUD (speed, health, ammo, mission, minimap-lite), story dialogue, pause, game over, settings.
### Dev console
Backtick/`~` toggles. Commands: help, god, noclip, tp, speed, boost, spawn, give, heal, kill, time, fog, bloom, music, sfx, seed, fov, timescale, wireframe, fps, clear, mission, bind, echo, quit. History, autocomplete, cvars.

## Non-goals
Multiplayer, external assets, save game cloud.

## Platforms
Linux/Windows/macOS native (Bevy). Optional WASM later.

## Quality bar
60 fps target on mid GPU; `cargo run --release`.
