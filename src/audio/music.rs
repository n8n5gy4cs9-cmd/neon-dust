//! Procedural synthwave songs rendered offline into stems.
use super::synth::*;
use crate::noise::Rng;

pub const PAD: u32 = 1;
pub const BASS: u32 = 2;
pub const ARP: u32 = 4;
pub const LEAD: u32 = 8;
pub const KICK: u32 = 16;
pub const SNARE: u32 = 32;
pub const HAT: u32 = 64;
pub const TOM: u32 = 128;
pub const FX: u32 = 256;

pub struct Section {
    pub bars: usize,
    pub mask: u32,
}

pub struct Song {
    pub bpm: f32,
    pub root: i32,
    pub prog: Vec<(i32, bool)>, // semitone offset from root, minor?
    pub chord_bars: usize,
    pub sections: Vec<Section>,
    pub seed: u64,
    pub lead_oct: i32,
    pub bass_style: u8,
}

const MINOR_PENT: [i32; 5] = [0, 3, 5, 7, 10];

pub fn render(song: &Song, stem: u32) -> Buf {
    let beat = 60.0 / song.bpm;
    let bar = beat * 4.0;
    let total_bars: usize = song.sections.iter().map(|s| s.bars).sum();
    let n = (total_bars as f32 * bar * SR) as usize + (2.0 * SR) as usize;
    let loop_n = (total_bars as f32 * bar * SR) as usize;
    let mut rng = Rng::new(song.seed);

    let mut music = Buf::new(n); // pad/bass/arp (sidechained)
    let mut leadb = Buf::new(n);
    let mut drums = Buf::new(n);
    let mut verb_in = vec![0.0f32; n];
    let mut kick_times: Vec<usize> = vec![];

    let kick_s = kick();
    let snare_s = snare(&mut rng);
    let clap_s = clap(&mut rng);
    let hat_c = hat(&mut rng, false);
    let hat_o = hat(&mut rng, true);
    let crash_s = crash(&mut rng);

    // lead motif (generated once per song for coherence)
    let motif: Vec<(i32, f32)> = {
        let mut m = vec![];
        let mut deg = 2i32;
        let mut t = 0.0;
        while t < 4.0 {
            let len = [0.5, 0.5, 1.0, 0.75, 0.25][(rng.next_u32() % 5) as usize];
            deg = (deg + [-2, -1, 0, 1, 2][(rng.next_u32() % 5) as usize]).clamp(0, 9);
            m.push((deg, len));
            t += len;
        }
        m
    };

    let mut bar_idx = 0usize;
    for (si, sec) in song.sections.iter().enumerate() {
        let mask = sec.mask & stem;
        for b in 0..sec.bars {
            let t0 = bar_idx as f32 * bar;
            let s0 = (t0 * SR) as usize;
            let ci = (bar_idx / song.chord_bars) % song.prog.len();
            let (off, minor) = song.prog[ci];
            let root = song.root + off;
            let third = if minor { 3 } else { 4 };
            let chord = [root, root + third, root + 7, root + 10 + if minor { 0 } else { 1 }];
            let first_of_sec = b == 0;
            let last_of_sec = b == sec.bars - 1;

            // kick pattern always computed for sidechain
            for k in 0..4 {
                let ks = s0 + (k as f32 * beat * SR) as usize;
                kick_times.push(ks);
                if mask & KICK != 0 && !(last_of_sec && k == 3 && sec.bars > 2 && mask & TOM != 0) {
                    drums.add_at(ks, &kick_s, 0.95, 0.95);
                }
            }
            // pad
            if mask & PAD != 0 && b % song.chord_bars == 0 {
                let dur = bar * song.chord_bars as f32;
                for (i, &nn) in chord.iter().enumerate() {
                    let f = midi((nn + 36 + 12) as f32);
                    let sig = supersaw(f, dur, 0.5, 0.9, 1400.0 + 800.0 * (si as f32 * 0.3), 4, 14.0, 0.0);
                    let (gl, gr) = pan_gains(if i % 2 == 0 { -0.5 } else { 0.5 });
                    music.add_at(s0, &sig, 0.09 * gl, 0.09 * gr);
                    for (k, v) in sig.iter().enumerate() {
                        if s0 + k < n {
                            verb_in[s0 + k] += v * 0.03;
                        }
                    }
                }
            }
            // bass
            if mask & BASS != 0 {
                let bn = root as f32 + 24.0;
                for e in 0..8 {
                    let semis = match (song.bass_style, e) {
                        (0, 3) | (0, 6) => 12.0,
                        (0, 7) => 7.0,
                        (1, 1) | (1, 3) | (1, 5) | (1, 7) => 12.0,
                        (_, 4) if song.bass_style == 2 => 7.0,
                        _ => 0.0,
                    };
                    let ns = s0 + (e as f32 * beat * 0.5 * SR) as usize;
                    let sig = bass_note(midi(bn + semis), beat * 0.45, 1500.0);
                    music.add_at(ns, &sig, 0.34, 0.34);
                }
            }
            // arp
            if mask & ARP != 0 {
                let pat = [0usize, 1, 2, 3, 2, 1, 2, 1];
                for sx in 0..16 {
                    let ns = s0 + (sx as f32 * beat * 0.25 * SR) as usize;
                    let idx = pat[sx % 8];
                    let nn = chord[idx % 4] + 60 + if sx % 8 >= 4 { 12 } else { 0 };
                    let sig = pluck(midi(nn as f32), beat * 0.22, 3000.0);
                    let pn = ((sx as f32 * 0.7).sin()) * 0.6;
                    let (gl, gr) = pan_gains(pn);
                    music.add_at(ns, &sig, 0.12 * gl, 0.12 * gr);
                }
            }
            // lead
            if mask & LEAD != 0 {
                let mut t = 0.0f32;
                let bar_in_phrase = bar_idx % 4;
                let mut acc = 0.0;
                for (deg, len) in motif.iter() {
                    // pick notes belonging to this bar of the 4-bar motif
                    if acc >= bar_in_phrase as f32 * 1.0 && acc < (bar_in_phrase + 1) as f32 * 1.0 {
                        let scale_note = MINOR_PENT[(*deg as usize) % 5] + 12 * (*deg / 5);
                        let nn = root + 12 * (song.lead_oct) + scale_note + 24;
                        let ns = s0 + (t * beat * SR) as usize;
                        let sig = supersaw(midi(nn as f32), len * beat * 0.95, 0.01, 0.2, 3800.0, 3, 12.0, 0.004);
                        music_lead_add(&mut leadb, ns, &sig, 0.13);
                        for (k, v) in sig.iter().enumerate() {
                            let idx = ns + k;
                            if idx < n {
                                verb_in[idx] += v * 0.08;
                            }
                        }
                        t += len;
                    }
                    acc += len / 4.0;
                    if t >= 4.0 {
                        break;
                    }
                }
            }
            // snare / clap
            if mask & SNARE != 0 {
                for k in [1usize, 3] {
                    let ss = s0 + (k as f32 * beat * SR) as usize;
                    drums.add_at(ss, &snare_s, 0.55, 0.55);
                    drums.add_at(ss, &clap_s, 0.35, 0.35);
                    for (i, v) in snare_s.iter().enumerate() {
                        if ss + i < n {
                            verb_in[ss + i] += v * 0.2;
                        }
                    }
                }
            }
            // hats
            if mask & HAT != 0 {
                for e in 0..8 {
                    let hs = s0 + (e as f32 * beat * 0.5 * SR) as usize;
                    if e % 2 == 1 {
                        drums.add_at(hs, &hat_o, 0.4, 0.3);
                    } else {
                        drums.add_at(hs, &hat_c, 0.3, 0.4);
                    }
                }
            }
            // toms fill
            if mask & TOM != 0 && last_of_sec {
                for k in 0..4 {
                    let ts = s0 + ((3.0 + k as f32 * 0.25) * beat * SR) as usize;
                    let f = [220.0, 180.0, 140.0, 100.0][k];
                    drums.add_at(ts, &tom(f), 0.6, 0.6);
                }
            }
            // fx
            if mask & FX != 0 {
                if first_of_sec && si > 0 {
                    drums.add_at(s0, &crash_s, 0.5, 0.5);
                }
                if last_of_sec && si + 1 < song.sections.len() {
                    let r = riser(bar, &mut rng);
                    drums.add_at(s0, &r, 0.4, 0.4);
                }
            }
            bar_idx += 1;
        }
    }

    // sidechain: duck music bus by kick envelope
    let mut duck = vec![1.0f32; n];
    for &ks in &kick_times {
        for i in 0..(0.22 * SR) as usize {
            if ks + i >= n {
                break;
            }
            let g = 1.0 - 0.75 * (-(i as f32 / SR) * 14.0).exp();
            if g < duck[ks + i] {
                duck[ks + i] = g;
            }
        }
    }
    for i in 0..n {
        music.l[i] *= duck[i];
        music.r[i] *= duck[i];
    }
    // lead through ping-pong delay
    pingpong(&mut leadb, beat * 0.75, 0.38, 0.55);
    pingpong(&mut music, beat * 0.75, 0.3, 0.25);

    let mut out = Buf::new(n);
    out.mix(&music, 1.0);
    out.mix(&leadb, 1.0);
    out.mix(&drums, 1.0);
    let (rl, rr) = reverb(&verb_in, 0.9);
    for i in 0..n {
        out.l[i] += rl[i];
        out.r[i] += rr[i];
    }
    // fold the tail back onto the start for a seamless loop, then cut to loop length
    for i in loop_n..n {
        let j = i - loop_n;
        out.l[j] += out.l[i];
        out.r[j] += out.r[i];
    }
    out.l.truncate(loop_n);
    out.r.truncate(loop_n);
    master(&mut out, 0.82);
    out
}

fn music_lead_add(b: &mut Buf, at: usize, sig: &[f32], g: f32) {
    let (gl, gr) = pan_gains(0.1);
    b.add_at(at, sig, g * gl, g * gr);
}

pub fn song_menu() -> Song {
    Song {
        bpm: 92.0,
        root: 9,
        prog: vec![(0, true), (-4, false), (3, false), (-2, false)],
        chord_bars: 2,
        sections: vec![
            Section { bars: 4, mask: PAD | ARP },
            Section { bars: 4, mask: PAD | ARP | BASS | KICK },
            Section { bars: 8, mask: PAD | ARP | BASS | KICK | HAT | LEAD | FX },
        ],
        seed: 11,
        lead_oct: 0,
        bass_style: 1,
    }
}

pub fn song_drive() -> Song {
    Song {
        bpm: 124.0,
        root: 6,
        prog: vec![(0, true), (-3, false), (3, false), (-2, false)],
        chord_bars: 2,
        sections: vec![
            Section { bars: 4, mask: PAD | ARP | KICK },
            Section { bars: 4, mask: PAD | ARP | BASS | KICK | HAT | FX },
            Section { bars: 8, mask: PAD | ARP | BASS | KICK | HAT | SNARE | LEAD | FX | TOM },
            Section { bars: 4, mask: PAD | ARP | KICK },
            Section { bars: 8, mask: PAD | ARP | BASS | KICK | HAT | SNARE | LEAD | FX | TOM },
            Section { bars: 4, mask: PAD | BASS | KICK | HAT | FX },
        ],
        seed: 42,
        lead_oct: 1,
        bass_style: 0,
    }
}

pub fn song_port() -> Song {
    Song {
        bpm: 142.0,
        root: 2,
        prog: vec![(0, true), (0, true), (-4, false), (-2, false)],
        chord_bars: 1,
        sections: vec![
            Section { bars: 4, mask: PAD | ARP | BASS | KICK | HAT },
            Section { bars: 8, mask: PAD | ARP | BASS | KICK | HAT | SNARE | LEAD | FX | TOM },
            Section { bars: 8, mask: PAD | ARP | BASS | KICK | HAT | SNARE | LEAD | FX | TOM },
            Section { bars: 4, mask: PAD | ARP | BASS | KICK | SNARE | TOM | FX },
        ],
        seed: 7,
        lead_oct: 1,
        bass_style: 2,
    }
}

/// short stinger: (minor chord decay) for death / win
pub fn stinger(win: bool) -> Buf {
    let n = (5.0 * SR) as usize;
    let mut b = Buf::new(n);
    let notes: Vec<i32> = if win { vec![57, 61, 64, 69, 73, 76] } else { vec![45, 48, 51, 57] };
    for (i, nn) in notes.iter().enumerate() {
        let sig = supersaw(midi(*nn as f32), 2.5, 0.02, 1.8, 2500.0, 3, 15.0, 0.0);
        let (gl, gr) = pan_gains((i as f32 / notes.len() as f32) * 1.4 - 0.7);
        b.add_at((i as f32 * if win { 0.12 } else { 0.0 } * SR) as usize, &sig, 0.2 * gl, 0.2 * gr);
    }
    let (rl, rr) = reverb(&b.l.iter().zip(b.r.iter()).map(|(a, c)| (a + c) * 0.5).collect::<Vec<_>>(), 0.8);
    for i in 0..n {
        b.l[i] += rl[i];
        b.r[i] += rr[i];
    }
    master(&mut b, 0.8);
    b
}
