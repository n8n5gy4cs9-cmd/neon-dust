# Neon Dust — Technical Plan

## Stack
Rust, Bevy (see Cargo.toml), WGSL custom shaders, `cpal`-free audio via Bevy `AudioSource` fed with WAV bytes generated in code.

## Module layout (`src/`)
| module | role |
|---|---|
| main.rs | App, plugins, states (Boot/Menu/Playing/Paused/Dead) |
| noise.rs | hash, value/simplex noise, fBm, ridged |
| world/terrain.rs | chunk streaming, heightfield, island mask, mesh gen, vertex colors |
| world/props.rs | trees, crystals, clouds, mountains silhouettes, water |
| world/sky.rs | sky dome shader, sun, stars, fog setup |
| shaders/*.wgsl | neon grid terrain, water, sky |
| vehicle.rs | DustRacing2D-derived physics in 3D |
| player.rs | on-foot controller, enter/exit car |
| camera.rs | third-person/chase camera |
| weapons.rs | weapons, bullets, tracers, hit detection |
| enemies.rs | drones, sentries |
| fx.rs | particles: explosions, fire, smoke, sparks, floating motes |
| audio/synth.rs | oscillators, envelopes, drums, WAV encoding |
| audio/music.rs | synthwave sequencer → tracks |
| audio/sfx.rs | SFX bank |
| ui.rs | HUD, menu, dialogue |
| story.rs | missions + dialogue |
| console.rs | dev console + commands |

## Coordinate mapping (2D → 3D)
DustRacing2D (x,y) → Bevy (x,z); up = +y. Forces computed on plane, gravity/ground in y.

## Milestones
M0 docs · M1 scaffold+terrain · M2 vehicle · M3 player+camera · M4 weapons+fx · M5 audio · M6 enemies+story+UI · M7 console · M8 polish/page.
