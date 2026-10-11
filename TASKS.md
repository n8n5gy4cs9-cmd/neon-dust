# Neon Dust — Task Board
_Legend: `[x]` done · `[~]` in progress · `[ ]` todo · `[!]` blocked · ★ = optional. Update this file with every commit._

**Progress: 52/78 tasks done** · version 0.1.0 · updated 2026-10-11

**WHERE TO CONTINUE:** World, vehicle, on-foot, weapons, fx, enemies, audio and story logic all compile and run (software-rendered screenshots verified). NEXT: playtest/balance pass, split-screen co-op (optional M12), tunnel/menu shader extras (M10.7-9), release QA (M13).

## M0 — Plans & docs  (5/5)
- [x] **M0.1** Write PRD.md (pitch, story, pillars, features, non-goals)
- [x] **M0.2** Write PLAN.md (stack, module layout, 2D→3D coordinate mapping, milestones)
- [x] **M0.3** Write TASKS.md (this board) in detailed milestone format
- [x] **M0.4** docs/index.html: full game page — screenshots, guide, manual, controls, console reference
- [x] **M0.5** README.md (build/run, controls, credits incl. Shadertoy inspirations)

## M1 — Project skeleton  (5/6)
- [x] **M1.1** Cargo project, Bevy 0.16 (+wav), dev/release profiles — _deps opt-level 3 in dev_
- [x] **M1.2** App states Menu/Playing/Paused/Dead/Ending + SessionPlugin (new game, respawn, pause, cursor grab)
- [x] **M1.3** Shared game.rs: Stats, Cheats, Control, FxQueue, SfxQueue, Toasts, Shake
- [x] **M1.4** Cvars (view radius, fog, bloom, fov, volumes, sensitivity, timescale)
- [x] **M1.5** Headless screenshot harness: `./run_shot.sh out.png secs [--play]` (xvfb + lavapipe) — _used for all visual checks_
- [ ] **M1.6** Window/settings persistence (settings.json) ★

## M2 — Procedural world  (11/12)
- [x] **M2.1** noise.rs: hash, value noise, fBm, ridged, RNG
- [x] **M2.2** meshgen.rs: flat-shaded mesh builder (cuboid, cone, frustum, wheel, vertex colour + glow alpha)
- [x] **M2.3** Floating islands: island mask, bridges/causeways, rim height per region, cliff edges
- [x] **M2.4** Mountains (ridged noise), plains/valleys mask, hills, detail
- [x] **M2.5** Rivers carved into terrain + reflective water mesh
- [x] **M2.6** Waterfalls spilling off island rim (animated shader strip)
- [x] **M2.7** Island undersides (inverted rock cones with glowing veins)
- [x] **M2.8** Async chunk streaming (128 m chunks, view radius cvar, task pool) + prop colliders
- [x] **M2.9** Forests (bioluminescent trees), crystal clusters, neon monoliths per chunk
- [x] **M2.10** Distant ridge silhouette layers + distant floating-island skybox props
- [x] **M2.11** Drifting clouds (above + below), floating glow motes
- [ ] **M2.12** Volumetric light shafts (Bevy VolumetricFog) ★

## M3 — Look (Neon Drift)  (8/10)
- [x] **M3.1** neon.wgsl terrain: tessellated pink/cyan neon line grid on near-black ground (Shadertoy-style) + contour lines + fresnel ridges
- [x] **M3.2** neon.wgsl modes: glowing solids, reflective water, waterfall, underside veins
- [x] **M3.3** sky.wgsl: gradient sunset, striped retro sun, stars, horizon glow, grid sea
- [x] **M3.4** Atmospheric fog (exp. with sun inscatter), bloom, TonyMcMapface tonemapping, FXAA
- [x] **M3.5** Dynamic directional light + cascaded shadows, violet ambient
- [x] **M3.6** Synthwave palette tune after first reference review (darker ground, warm haze)
- [x] **M3.7** Particles: explosions, fire, smoke, sparks, tracers, dust, boost flames, shockwave rings, flash lights
- [x] **M3.8** Screen shake + FOV speed kick
- [ ] **M3.9** Colour-grading component + post tweak (vignette/chromatic aberration) ★
- [ ] **M3.10** Palette presets via console (`palette sunset|night|ice`) ★

## M4 — Vehicle (DustRacing2D → 3D)  (7/8)
- [x] **M4.1** 2D rigid body in x/z plane: mass, inertia, yaw rate
- [x] **M4.2** Engine force from power curve, brakes, reverse, quadratic drag + rolling resistance
- [x] **M4.3** Slip-angle tyre model with sliding friction (drift) + handbrake rear-grip cut
- [x] **M4.4** Boost with fuel (regen, pickups)
- [x] **M4.5** 3D: gravity, heightfield following, slope acceleration, jumps off crests, pitch/roll from normal
- [x] **M4.6** Prop collisions, water drag + splash, void fall → respawn at last safe spot
- [x] **M4.7** Procedural car mesh (neon strips, underglow, lights) + headlight/underglow lights
- [ ] **M4.8** Physics unit tests (top speed, drift, jump arc) ★

## M5 — On-foot & camera (OpenRW-style)  (5/6)
- [x] **M5.1** Third-person orbit camera with shoulder offset + aim zoom
- [x] **M5.2** Chase camera for car (auto-recentre, speed pull-back, drift-aware yaw)
- [x] **M5.3** Character XZ (procedural mesh, walking leg animation), sprint, jump, water slow
- [x] **M5.4** Enter / exit car (F), bail-out with inherited velocity
- [x] **M5.5** Aim ray → crosshair world point; muzzle aiming from camera
- [ ] **M5.6** Climb/vault & ledge grab ★

## M6 — Weapons & combat  (6/7)
- [x] **M6.1** Projectile system with tracer bullets (5 tracer colours), segment-sphere hits, terrain/prop hits
- [x] **M6.2** Weapons: Pulse Pistol, Plasma SMG, Rail Shotgun, Rocket Launcher; ammo, cooldown, switch (1-4, wheel)
- [x] **M6.3** Car cannons (LMB) + rockets (RMB)
- [x] **M6.4** Explosions with splash damage (enemies + self)
- [x] **M6.5** Pickups (health, ammo, boost, weapon crate)
- [x] **M6.6** Damage model: hurt vignette state, god mode, death → Kernel Panic
- [ ] **M6.7** Hit markers + damage numbers ★

## M7 — Enemies  (4/5)
- [x] **M7.1** Glitcher drones (orbit, lead-aim bursts)
- [x] **M7.2** Firewall sentries (rotating turrets)
- [x] **M7.3** Pursuit cars (same vehicle physics, ramming, void avoidance)
- [x] **M7.4** The Warden boss (fan volleys + rockets, support drones)
- [ ] **M7.5** Enemy director balance pass ★

## M8 — Audio (all in code)  (6/6)
- [x] **M8.1** synth.rs: PolyBLEP saw/pulse, filters, drums, reverb, ping-pong delay, WAV encoder
- [x] **M8.2** music.rs: sequencer (pad, bass, arp, lead, kick/snare/hat/tom/fx, sidechain) → stems
- [x] **M8.3** Songs: menu (92 BPM), drive (124 BPM, base + combat stems), port finale (142 BPM), death stinger
- [x] **M8.4** SFX bank (20 effects) + seamless engine loop + tyre screech loop
- [x] **M8.5** Audio plugin: background generation thread, dynamic music mix (calm/combat/port), SFX queue with distance attenuation, engine pitch
- [x] **M8.6** `ND_DUMP_AUDIO=dir` exports the generated WAVs for listening

## M9 — Story & missions  (4/5)
- [x] **M9.1** Story state machine: Wake → Boot → Flush → Gate → Firewall → Hack → Pursuit → Port → Warden → Choice → Free
- [x] **M9.2** Reachable objective placement (BFS over passable land) + beacons
- [x] **M9.3** ORACLE / XZ / WARDEN dialogue queue, two endings, weapon unlock rewards
- [x] **M9.4** Terminal hack mechanic (hold E), sentry gate
- [ ] **M9.5** Extra side missions (time trial, wave defence) ★

## M10 — UI  (6/9)
- [x] **M10.1** Loading splash: kaleidoscope raymarch shader (Shadertoy "Active Matter" structure, PI tuned 3.6/4.1) + progress bar + cool rotating status text
- [x] **M10.2** Title menu: gradient letter title, controls, story blurb, blinking prompt
- [x] **M10.3** HUD: speed, boost bar, integrity bar, weapon/ammo, objective + bearing, score, crosshair
- [x] **M10.4** Radar, toasts, dialogue box with typing effect, boss bar, hack bar
- [x] **M10.5** Damage vignette, FPS counter
- [x] **M10.6** Pause / Kernel Panic / Ending overlays
- [ ] **M10.7** Tunnel effect (Shadertoy tXGGRD-style) for boot intro and ending ★
- [ ] **M10.8** Reflective neon-city water band behind menu (fdffz2-style) ★
- [ ] **M10.9** Gradient text shader (ttBGRD-style) for title ★

## M11 — Console & cheats (nordrep-style)  (6/6)
- [x] **M11.1** Pure command registry + parser + aliases + tab completion + history (core/commands)
- [x] **M11.2** Console UI (` key), scrollback, input caret, input capture
- [x] **M11.3** General commands: help, commands, clear, echo, history, version, quit, pos, status, seed, fps, fov, bloom, fog, viewdist, timescale, musicvol, sfxvol, sens, track, screenshot
- [x] **M11.4** Cheat commands (need `cheats on`): god, noclip, infammo, onehit, peaceful, infboost, lowgrav, speed, heal, give, tp, goto, spawn, killall, car, flip, mission, next, regen
- [x] **M11.5** `--cmd "..."` CLI to run console lines at startup
- [x] **M11.6** commands.txt generated from registry

## M12 — Split-screen co-op ★ (optional)  (0/5)
- [ ] **M12.1** Port nordrep `core/split.rs` layout rules (1–2 players, top/bottom or left/right)
- [ ] **M12.2** Second Player + car, per-player camera viewport
- [ ] **M12.3** Input slots: **P1 = keyboard + mouse only, P2 = gamepad only** (join with Start, dead zones)
- [ ] **M12.4** Per-viewport HUD; shared world tick; enemy targeting nearest player
- [ ] **M12.5** `split 1|2` console command

## M13 — Release & QA  (0/3)
- [ ] **M13.1** Release build check + perf pass (view radius, particle caps)
- [ ] **M13.2** Final screenshots for the HTML page
- [ ] **M13.3** Final docs pass (README, TASKS, PRD, commands.txt)
