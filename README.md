# NEON DUST
Fast synthwave driving + shooting + on-foot run & gun across procedural floating neon islands.
Rust + Bevy 0.16. Everything (meshes, shaders, SFX, soundtrack) is generated in code.

```
cargo run --release
```
Linux deps: `libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`.

Full guide, manual, screenshots and console reference: open `docs/index.html` (regenerate with `python3 tools/gen_page.py`).
Docs: `PRD.md`, `PLAN.md`, `TASKS.md`, `commands.txt`.

Controls: WASD, Shift boost/sprint, Space drift/jump, LMB/RMB fire, F enter/exit car, 1-4 weapons, E hack, Esc pause, ` console.

Fonts (SIL OFL, embedded in `src/fonts/`): VT323, Share Tech Mono (terminal text); Monoton, Bungee Shade (titles).
Shader inspiration: stduhpf (synthwave sunset), Gruberk (Active Matter loading splash, PI tuned to 3.6).
