#!/usr/bin/env python3
"""Generate docs/index.html (game page + guide + manual) from commands.txt. Run: python3 tools/gen_page.py"""
import html, re

cmds = []
cat = None
lines = open("commands.txt").read().splitlines()
i = 0
rows = {}
while i < len(lines):
    l = lines[i]
    if l.startswith("== "):
        cat = l.strip("= ").strip(); rows[cat] = []
    elif l.startswith("  ") and not l.startswith("      ") and cat:
        usage = l.strip(); desc = lines[i + 1].strip() if i + 1 < len(lines) else ""
        rows[cat].append((usage, desc)); i += 1
    i += 1

def cmd_table():
    out = []
    for c, items in rows.items():
        out.append(f"<h3>{html.escape(c)}</h3><table class='t'>")
        for u, d in items:
            ch = "[cheat]" in u
            u2 = u.replace("[cheat]", "").strip()
            out.append(f"<tr><td><code>{html.escape(u2)}</code>{' <span class=badge>cheat</span>' if ch else ''}</td><td>{html.escape(d)}</td></tr>")
        out.append("</table>")
    return "\n".join(out)

shots = [
    ("menu.jpg", "Title screen over a live flyover of the world"),
    ("load.jpg", "Loading splash: kaleidoscope raymarch shader + synth progress"),
    ("drive.jpg", "Driving with the third-person chase camera, HUD and radar"),
    ("combat.jpg", "Pursuit car and Glitcher drones closing in"),
    ("water.jpg", "Rivers, floating islands and distant ridges"),
    ("console.jpg", "The developer console (` key)"),
]
gallery = "\n".join(f"<figure><a href='shots/{f}'><img src='shots/{f}' alt='{html.escape(c)}' loading='lazy'></a><figcaption>{html.escape(c)}</figcaption></figure>" for f, c in shots)

page = """<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Neon Dust</title>
<style>
:root{--bg:#07021a;--bg2:#12062e;--cy:#19f2ff;--mg:#ff2bd0;--or:#ff9a2a;--tx:#e9defc;--dim:#9a88c8}
*{box-sizing:border-box}body{margin:0;background:radial-gradient(1200px 600px at 50% -10%,#3a0a5e 0,#12062e 45%,#07021a 100%) fixed;color:var(--tx);font:16px/1.6 ui-monospace,Menlo,Consolas,monospace}
header{min-height:78vh;display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center;padding:40px 16px;position:relative;overflow:hidden}
header:before{content:"";position:absolute;left:0;right:0;bottom:0;height:46%;background:
linear-gradient(transparent,rgba(255,43,208,.18)),repeating-linear-gradient(90deg,rgba(255,43,208,.5) 0 1px,transparent 1px 60px),repeating-linear-gradient(0deg,rgba(25,242,255,.35) 0 1px,transparent 1px 38px);transform:perspective(420px) rotateX(58deg);transform-origin:bottom;mask:linear-gradient(transparent,#000 40%)}
header:after{content:"";position:absolute;width:340px;height:340px;border-radius:50%;top:6%;background:repeating-linear-gradient(0deg,transparent 0 14px,#07021a 14px 18px),linear-gradient(#ffd23a,#ff2bd0);opacity:.55;filter:blur(1px);z-index:0}
h1{font-size:clamp(48px,11vw,140px);margin:0;letter-spacing:.04em;background:linear-gradient(90deg,var(--cy),var(--mg),var(--or));-webkit-background-clip:text;background-clip:text;color:transparent;text-shadow:0 0 40px rgba(255,43,208,.35);position:relative;z-index:1}
.tag{color:var(--dim);position:relative;z-index:1;max-width:760px}
.btns{margin-top:22px;position:relative;z-index:1;display:flex;gap:12px;flex-wrap:wrap;justify-content:center}
a.b{color:#07021a;background:linear-gradient(90deg,var(--cy),var(--mg));padding:10px 20px;border-radius:4px;text-decoration:none;font-weight:700}
a.b.o{background:none;color:var(--cy);border:1px solid var(--cy)}
nav{position:sticky;top:0;background:#07021acc;backdrop-filter:blur(8px);border-bottom:1px solid #ff2bd055;padding:10px 16px;z-index:5;display:flex;gap:18px;flex-wrap:wrap;justify-content:center}
nav a{color:var(--cy);text-decoration:none}
main{max-width:1060px;margin:0 auto;padding:10px 16px 80px}
h2{color:var(--mg);border-bottom:1px solid #ff2bd055;padding-bottom:6px;margin-top:60px}
h3{color:var(--cy);margin-top:28px}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:18px}
figure{margin:0;background:#12062e;border:1px solid #ff2bd044;border-radius:6px;overflow:hidden}
figure img{width:100%;display:block}figcaption{padding:8px 12px;color:var(--dim);font-size:14px}
table.t{border-collapse:collapse;width:100%;margin:8px 0}table.t td,table.t th{border-bottom:1px solid #ff2bd02a;padding:6px 10px;vertical-align:top;text-align:left}
th{color:var(--or)}code{color:var(--cy);background:#0e0425;padding:1px 6px;border-radius:3px}kbd{background:#1c0b44;border:1px solid var(--mg);border-radius:4px;padding:0 6px;color:#fff}
.badge{font-size:11px;color:#07021a;background:var(--or);border-radius:3px;padding:0 5px}
.card{background:#12062e;border:1px solid #19f2ff33;border-radius:6px;padding:14px 18px}
.quote{border-left:3px solid var(--mg);padding-left:14px;color:#ffd3f5;margin:10px 0}
footer{text-align:center;color:var(--dim);padding:40px 16px}
</style></head><body>
<header>
<h1>NEON DUST</h1>
<p class="tag">Fast, loud, synthwave. Drive, shoot, bail out, run &amp; gun across an endless sky of floating neon islands. You are <b>XZ</b>. You woke up inside your computer. The only way out is the Port.</p>
<div class="btns"><a class="b" href="#play">How to play</a><a class="b o" href="#console">Console &amp; cheats</a><a class="b o" href="#gallery">Screenshots</a></div>
</header>
<nav><a href="#story">Story</a><a href="#gallery">Screenshots</a><a href="#play">Quick start</a><a href="#controls">Controls</a><a href="#guide">Guide</a><a href="#manual">Manual</a><a href="#console">Console</a><a href="#tech">Tech</a><a href="#credits">Credits</a></nav>
<main>
<h2 id="story">Story</h2>
<div class="card">
<div class="quote">ORACLE: ...process 0xXZ detected. You are not in the process table.</div>
<p>XZ wakes in <code>/dev/null</code>, a dead sector of a machine that should not be running. ORACLE, the OS kernel, speaks in half-sentences: the sectors float, memory drifts, and a Port at the edge of the system is the only way out. Corrupted daemons - <b>Glitchers</b> - hunt anything that is not in the process table. Cross the sectors by car, break the <b>Firewall</b> on foot, outrun the pursuit daemons, and face <b>the Warden</b> at the Port. Then choose: pull the plug and escape - or stay and become the new kernel.</p>
<p><b>Acts:</b> Prologue (/dev/null) &rarr; Boot Sector &rarr; Cache Flush &rarr; The Firewall (sentries + hack) &rarr; Pursuit &rarr; The Port (boss) &rarr; two endings &rarr; free roam.</p>
</div>

<h2 id="gallery">Screenshots</h2>
<p>Captured from the real game (software-rendered with Mesa lavapipe, so frame rate is not representative).</p>
<div class="grid">GALLERY</div>

<h2 id="play">Quick start</h2>
<div class="card"><pre>git clone https://github.com/n8n5gy4cs9-cmd/neon-dust
cd neon-dust
# Linux needs: libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev
cargo run --release</pre>
<p>First launch shows the loading splash while the whole soundtrack and SFX are synthesised in code (a few seconds in release), then press <kbd>Enter</kbd>.</p></div>

<h2 id="controls">Controls</h2>
<table class="t"><tr><th>Action</th><th>Car</th><th>On foot</th></tr>
<tr><td>Move / drive</td><td><kbd>W</kbd><kbd>A</kbd><kbd>S</kbd><kbd>D</kbd> (throttle, steer, brake/reverse)</td><td><kbd>W</kbd><kbd>A</kbd><kbd>S</kbd><kbd>D</kbd> relative to camera</td></tr>
<tr><td>Boost / sprint</td><td><kbd>Shift</kbd> (boost fuel recharges)</td><td><kbd>Shift</kbd></td></tr>
<tr><td>Handbrake drift / jump</td><td><kbd>Space</kbd></td><td><kbd>Space</kbd></td></tr>
<tr><td>Fire</td><td>LMB twin cannons</td><td>LMB current weapon</td></tr>
<tr><td>Secondary</td><td>RMB rockets</td><td>RMB aim (zoom, slower)</td></tr>
<tr><td>Camera</td><td colspan="2">Mouse (chase cam auto-recentres after 1 s)</td></tr>
<tr><td>Enter / exit car</td><td colspan="2"><kbd>F</kbd> (within 6 m of the car)</td></tr>
<tr><td>Weapons</td><td colspan="2"><kbd>1</kbd>-<kbd>4</kbd> or mouse wheel</td></tr>
<tr><td>Hack terminal</td><td colspan="2">hold <kbd>E</kbd> next to it (on foot)</td></tr>
<tr><td>Pause</td><td colspan="2"><kbd>Esc</kbd></td></tr>
<tr><td>Console</td><td colspan="2"><kbd>`</kbd> (backtick)</td></tr></table>

<h2 id="guide">Player guide</h2>
<div class="grid">
<div class="card"><h3>Driving</h3><p>The car is a real force-based model: engine power, brakes, drag, rolling resistance and slip-angle tyres with sliding friction. Turn in hard and tap <kbd>Space</kbd> to break the rear tyres loose and slide. Slopes pull and push the car; crests launch it. Water slows you, void edges end you (you respawn at your last safe spot, minus a bit of integrity).</p></div>
<div class="card"><h3>Boost</h3><p>Hold <kbd>Shift</kbd> for roughly double power and a wider FOV. The bar drains in ~3 s and refills while you are not boosting. Blue pickups refill it instantly.</p></div>
<div class="card"><h3>Shooting</h3><p>Bullets are real tracers with travel time. Enemy drones lead their shots - keep moving. Rockets splash everything near the impact, including you. Rail shotgun is a point-blank monster; the pulse pistol never runs dry.</p></div>
<div class="card"><h3>Bailing out</h3><p>Press <kbd>F</kbd> at any speed to jump out with a fraction of the car's velocity. The Firewall gate only opens for someone on foot: sprint, jump and use cover while sentries burn down.</p></div>
<div class="card"><h3>Score &amp; pickups</h3><p>Drones 100, sentries 250, pursuit cars 300, the Warden 5000. Green = integrity, yellow = ammo, cyan = boost, magenta crate = new weapon.</p></div>
<div class="card"><h3>Mission walkthrough</h3><ol><li>Get in the car (<kbd>F</kbd>).</li><li>Drive to the cyan beacon.</li><li>Destroy 8 Glitchers - unlocks the Rail Shotgun.</li><li>Drive to the magenta Firewall beacon, leave the car, destroy 4 sentries, hold <kbd>E</kbd> at the terminal - unlocks the Rocket Launcher.</li><li>Outrun three pursuit cars to the orange beacon.</li><li>Reach the violet Port, destroy the Warden.</li><li>Choose <kbd>1</kbd> (escape) or <kbd>2</kbd> (become the kernel). Then free roam.</li></ol></div>
</div>

<h2 id="manual">Manual</h2>
<h3>HUD</h3>
<table class="t"><tr><td>Top-left</td><td>Chapter, current objective and bearing/distance to the beacon.</td></tr>
<tr><td>Top-right</td><td>Score, kills, radar (red dots = enemies, range 220 m, rotates with the camera).</td></tr>
<tr><td>Bottom-left</td><td>Weapon + ammo, integrity bar.</td></tr>
<tr><td>Bottom-right</td><td>Speed (km/h) and boost bar; hack progress during the Firewall.</td></tr>
<tr><td>Centre</td><td>Crosshair, context hints ([F] enter car, [HOLD E] hack), ORACLE dialogue box.</td></tr></table>
<h3>Weapons</h3>
<table class="t"><tr><th>#</th><th>Weapon</th><th>Notes</th></tr>
<tr><td>1</td><td>Pulse Pistol</td><td>5 shots/s, 18 dmg, infinite ammo, cyan tracers</td></tr>
<tr><td>2</td><td>Plasma SMG</td><td>15 shots/s, 8 dmg, spread, magenta tracers</td></tr>
<tr><td>3</td><td>Rail Shotgun</td><td>9 pellets x 11 dmg, 0.85 s cycle (unlocked after Cache Flush)</td></tr>
<tr><td>4</td><td>Rocket Launcher</td><td>90 dmg + 9 m splash (unlocked after the Firewall)</td></tr>
<tr><td>car</td><td>Cannons / rockets</td><td>LMB twin cannons (infinite), RMB rockets from your rocket ammo</td></tr></table>
<h3>Enemies</h3>
<table class="t"><tr><td><b>Glitcher</b></td><td>Flying drone, orbits and fires 3-round bursts with lead. 30 hp.</td></tr>
<tr><td><b>Sentry</b></td><td>Rotating turret guarding the Firewall. 110 hp.</td></tr>
<tr><td><b>Pursuit car</b></td><td>Uses the same car physics, boosts when far, rams. 130 hp.</td></tr>
<tr><td><b>The Warden</b></td><td>Boss at the Port: 5-way fan volleys, rockets, support drones. 1500 hp.</td></tr></table>
<h3>The world</h3>
<p>Terrain is generated from noise: huge floating islands with ridged mountain ranges, valleys, rivers that spill off the rim as waterfalls, bioluminescent forests, crystal clusters and neon monoliths. Narrow causeways bridge some islands. Chunks stream in around you; distant ridge silhouettes, far floating islands and clouds above and below provide scale. Change the world with <code>regen &lt;seed&gt;</code>.</p>

<h2 id="console">Developer console &amp; cheats</h2>
<p>Press <kbd>`</kbd>. <kbd>Tab</kbd> completes, <kbd>&uarr;</kbd>/<kbd>&darr;</kbd> history. Cheat commands require <code>cheats on</code> first (the run is then marked as cheated). You can also run lines at startup: <code>neon-dust --cmd "cheats on" --cmd "give all"</code>.</p>
COMMANDS

<h2 id="tech">Tech notes</h2>
<ul>
<li><b>Everything is generated in code:</b> meshes, vertex-colour materials, WGSL shaders (terrain, water, waterfall, sky, loading splash), every sound effect and the full synthwave soundtrack. No external asset files.</li>
<li><b>Driving</b> ports the DustRacing2D physics idea (force based, slip/sliding friction, quadratic drag + rolling resistance) from x/y to the x/z plane, adding gravity, heightfield following, jumps and terrain-aligned pitch/roll.</li>
<li><b>Camera / shooting / enter-exit</b> follow the OpenRW third-person feel.</li>
<li><b>Console</b> follows nordrep's registry design: a pure parser returns <i>Actions</i> that the game applies.</li>
<li><b>Music:</b> PolyBLEP oscillators, filters, drums, reverb and ping-pong delay render stems offline at startup; the mixer crossfades calm / combat / finale stems. Set <code>ND_DUMP_AUDIO=dir</code> to export the WAVs.</li>
<li>Screenshots: <code>./run_shot.sh out.png 20 --play --cmd "cheats on"</code> (xvfb + Mesa lavapipe).</li></ul>

<h2 id="credits">Credits</h2>
<p>Visual inspiration: stduhpf's <i>another synthwave sunset thing</i> (Shadertoy), Gruberk's <i>Active Matter loading splash</i> (loading kaleidoscope structure, used under its free-to-use licence; PI tuned to 3.6), and other Shadertoy synthwave pieces. Gameplay references: DustRacing2D, OpenRW, nordrep. Built with Rust and Bevy.</p>
</main>
<footer>NEON DUST &middot; ride the dust, find the Port.</footer>
</body></html>"""
page = page.replace("GALLERY", gallery).replace("COMMANDS", cmd_table())
open("docs/index.html", "w").write(page)
print("docs/index.html written", len(page))
