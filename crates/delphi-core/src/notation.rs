use rand::Rng;

use crate::chord::parse_quality;
use crate::event::NoteEvent;
use crate::gm::{drum_name_to_midi, gm_program_from_name};
use crate::{Chord, ChordQuality, Duration, Dynamic, Key, Note, Scale};

// ─── Notation Parser ─────────────────────────────────────────────────

/// Articulation modifiers applied to a single note/chord.
struct Articulation {
    vel_mult: f32,
    dur_mult: f32,
}

fn parse_articulation(suffix: &str) -> Option<Articulation> {
    match suffix {
        "stac" | "staccato" => Some(Articulation { vel_mult: 1.0, dur_mult: 0.5 }),
        "stacc" | "staccatissimo" => Some(Articulation { vel_mult: 1.0, dur_mult: 0.25 }),
        "ten" | "tenuto" => Some(Articulation { vel_mult: 1.0, dur_mult: 1.1 }),
        "port" | "portato" => Some(Articulation { vel_mult: 1.0, dur_mult: 0.75 }),
        "acc" | "accent" => Some(Articulation { vel_mult: 1.3, dur_mult: 1.0 }),
        "marc" | "marcato" => Some(Articulation { vel_mult: 1.4, dur_mult: 0.85 }),
        "ferm" | "fermata" => Some(Articulation { vel_mult: 1.0, dur_mult: 2.0 }),
        "ghost" => Some(Articulation { vel_mult: 0.4, dur_mult: 0.8 }),
        "leg" | "legato" => Some(Articulation { vel_mult: 1.0, dur_mult: 1.05 }),
        "pizz" | "pizzicato" => Some(Articulation { vel_mult: 1.0, dur_mult: 0.3 }),
        "mute" => Some(Articulation { vel_mult: 0.6, dur_mult: 0.5 }),
        _ => None,
    }
}

/// Result of stripping inline modifiers from a token.
pub(crate) struct TokenParts<'a> {
    core: &'a str,
    duration: Option<u32>,
    inline_velocity: Option<u8>,
    articulation: Option<Articulation>,
    repeat: u32,
    weight: f32,
    tie_next: bool,
    random_prob: Option<f32>,
}

/// Split a raw token into its core plus inline duration/dynamic/articulation/repeat/weight.
///
/// Grammar: `CORE[:DURATION][!DYNAMIC][.ARTIC][*N][@W][?P][~]`
pub(crate) fn split_token(raw: &str) -> TokenParts<'_> {
    let mut parts = TokenParts {
        core: raw,
        duration: None,
        inline_velocity: None,
        articulation: None,
        repeat: 1,
        weight: 1.0,
        tie_next: false,
        random_prob: None,
    };

    let mut s = raw;

    // Trailing tie marker
    if s.ends_with('~') {
        parts.tie_next = true;
        s = &s[..s.len() - 1];
    }

    // Random removal ?P
    if let Some(q) = s.find('?') {
        let prob_str = &s[q + 1..];
        parts.random_prob = Some(prob_str.parse::<f32>().unwrap_or(0.5));
        s = &s[..q];
    }

    // Elongation @W
    if let Some(at) = s.find('@') {
        let w_str = &s[at + 1..];
        parts.weight = w_str.parse::<f32>().unwrap_or(1.0);
        s = &s[..at];
    }

    // Repeat *N
    if let Some(star) = s.rfind('*') {
        if let Ok(n) = s[star + 1..].parse::<u32>() {
            parts.repeat = n.max(1);
            s = &s[..star];
        }
    }

    // Articulation .suffix
    if let Some(dot) = s.rfind('.') {
        let raw_suffix = &s[dot + 1..];
        let (suffix, embedded_dyn) = if let Some(bang) = raw_suffix.find('!') {
            (&raw_suffix[..bang], Some(&raw_suffix[bang + 1..]))
        } else {
            (raw_suffix, None)
        };
        if !suffix.is_empty() && !suffix.chars().next().unwrap().is_ascii_digit() {
            if let Some(a) = parse_articulation(suffix) {
                parts.articulation = Some(a);
                s = &s[..dot];
                if let Some(dyn_str) = embedded_dyn {
                    if let Some(v) = Dynamic::velocity_from_str(dyn_str) {
                        parts.inline_velocity = Some(v);
                    }
                }
            } else {
                // Ornament sentinels — use negative vel_mult as tag
                let ornament_tag = match suffix {
                    "tr" | "trill" => Some(-1.0),
                    "mord" | "mordent" => Some(-2.0),
                    "lmord" => Some(-3.0),
                    "turn" | "gruppetto" => Some(-4.0),
                    "grace" | "acciaccatura" => Some(-5.0),
                    "appoggiatura" => Some(-6.0),
                    "trem" | "tremolo" => Some(-7.0),
                    "gliss" | "glissando" => Some(-8.0),
                    "arp" | "arpeggio" => Some(-9.0),
                    "roll" => Some(-10.0),
                    _ => None,
                };
                if let Some(tag) = ornament_tag {
                    parts.articulation = Some(Articulation { vel_mult: tag, dur_mult: 1.0 });
                    s = &s[..dot];
                    if let Some(dyn_str) = embedded_dyn {
                        if let Some(v) = Dynamic::velocity_from_str(dyn_str) {
                            parts.inline_velocity = Some(v);
                        }
                    }
                }
            }
        }
    }

    // Dynamic !dyn
    if let Some(bang) = s.rfind('!') {
        let dyn_str = &s[bang + 1..];
        if let Some(v) = Dynamic::velocity_from_str(dyn_str) {
            parts.inline_velocity = Some(v);
            s = &s[..bang];
        }
    }

    // Duration :dur
    if let Some(colon) = s.rfind(':') {
        let dur_str = &s[colon + 1..];
        if let Some(d) = Duration::from_suffix(dur_str) {
            parts.duration = Some(d.ticks);
            s = &s[..colon];
        }
    }

    parts.core = s;
    parts
}

/// Expand an ornament tagged via negative vel_mult sentinel.
fn expand_ornament(tag: f32, midi_note: u8, total_dur: u32) -> Vec<(u8, u32)> {
    let tag = tag as i32;
    match tag {
        // Trill
        -1 => {
            let sub = (total_dur / 8).max(30);
            let mut out = Vec::new();
            let mut t = 0;
            let mut upper = false;
            while t + sub <= total_dur {
                let n = if upper { midi_note.saturating_add(2).min(127) } else { midi_note };
                out.push((n, sub));
                t += sub;
                upper = !upper;
            }
            if out.is_empty() { out.push((midi_note, total_dur)); }
            out
        }
        // Mordent (upper)
        -2 => {
            let third = total_dur / 3;
            vec![
                (midi_note, third),
                (midi_note.saturating_add(2).min(127), third),
                (midi_note, total_dur - 2 * third),
            ]
        }
        // Lower mordent
        -3 => {
            let third = total_dur / 3;
            vec![
                (midi_note, third),
                (midi_note.saturating_sub(1), third),
                (midi_note, total_dur - 2 * third),
            ]
        }
        // Turn
        -4 => {
            let q = total_dur / 4;
            vec![
                (midi_note.saturating_add(2).min(127), q),
                (midi_note, q),
                (midi_note.saturating_sub(1), q),
                (midi_note, total_dur - 3 * q),
            ]
        }
        // Grace / acciaccatura
        -5 => {
            let grace_dur = (total_dur / 4).max(30);
            vec![
                (midi_note.saturating_sub(1), grace_dur),
                (midi_note, total_dur - grace_dur),
            ]
        }
        // Appoggiatura
        -6 => {
            let half = total_dur / 2;
            vec![
                (midi_note.saturating_add(2).min(127), half),
                (midi_note, total_dur - half),
            ]
        }
        // Tremolo
        -7 => {
            let sub = (total_dur / 6).max(30);
            let mut out = Vec::new();
            let mut t = 0;
            while t + sub <= total_dur {
                out.push((midi_note, sub));
                t += sub;
            }
            if out.is_empty() { out.push((midi_note, total_dur)); }
            out
        }
        // Glissando
        -8 => {
            let steps = 8u32;
            let sub = total_dur / steps;
            (0..steps).map(|i| {
                let n = midi_note.saturating_add(i as u8).min(127);
                let dur = if i == steps - 1 { total_dur - sub * (steps - 1) } else { sub };
                (n, dur)
            }).collect()
        }
        // Arpeggio
        -9 => {
            let third = total_dur / 3;
            vec![
                (midi_note, third),
                (midi_note.saturating_add(4).min(127), third),
                (midi_note.saturating_add(7).min(127), total_dur - 2 * third),
            ]
        }
        // Roll
        -10 => {
            let sub = (total_dur / 6).max(30);
            let mut out = Vec::new();
            let mut t = 0;
            while t + sub <= total_dur {
                out.push((midi_note, sub));
                t += sub;
            }
            if out.is_empty() { out.push((midi_note, total_dur)); }
            out
        }
        _ => vec![(midi_note, total_dur)],
    }
}

// ─── Roman Numeral & Scale Degree Resolution ─────────────────────────

/// Parse a Roman numeral prefix from a token core.
/// Returns (degree 1-7, is_uppercase, remaining_str) or None.
fn parse_roman_prefix(s: &str) -> Option<(usize, bool, &str)> {
    static UPPER: &[(&str, usize)] = &[
        ("VII", 7), ("VI", 6), ("IV", 4), ("V", 5), ("III", 3), ("II", 2), ("I", 1),
    ];
    static LOWER: &[(&str, usize)] = &[
        ("vii", 7), ("vi", 6), ("iv", 4), ("v", 5), ("iii", 3), ("ii", 2), ("i", 1),
    ];

    for &(prefix, degree) in UPPER {
        if s.starts_with(prefix) {
            let rest = &s[prefix.len()..];
            // Reject if another Roman numeral letter follows (partial match)
            if rest.starts_with('I') || rest.starts_with('V') {
                continue;
            }
            return Some((degree, true, rest));
        }
    }
    for &(prefix, degree) in LOWER {
        if s.starts_with(prefix) {
            let rest = &s[prefix.len()..];
            if rest.starts_with('i') || rest.starts_with('v') {
                continue;
            }
            return Some((degree, false, rest));
        }
    }
    None
}

/// Resolve a Roman numeral token to MIDI notes using the current scale.
fn resolve_roman_numeral(core: &str, scale: &Scale) -> Option<Vec<u8>> {
    let (degree, is_upper, rest) = parse_roman_prefix(core)?;

    let scale_notes = scale.notes();
    if degree == 0 || degree > scale_notes.len() {
        return None;
    }
    let root_note = &scale_notes[degree - 1];
    let default_quality = if is_upper { ChordQuality::Major } else { ChordQuality::Minor };

    let (octave, quality) = if rest.is_empty() {
        (4i8, default_quality)
    } else if let Some(first) = rest.chars().next() {
        if first.is_ascii_digit() {
            // First try the whole rest as a known quality (e.g. "7", "9", "5")
            if let Ok(q) = parse_quality(rest) {
                (4, q)
            } else {
                // Treat leading digit as octave, remainder as quality
                let octave = (first as i8) - (b'0' as i8);
                let qrest = &rest[1..];
                let q = if qrest.is_empty() {
                    default_quality
                } else {
                    parse_quality(qrest).ok()?
                };
                (octave, q)
            }
        } else {
            (4, parse_quality(rest).ok()?)
        }
    } else {
        (4, default_quality)
    };

    let root = Note::new(root_note.pitch_class, root_note.accidental, octave);
    Some(Chord::new(root, quality).to_midi())
}

/// Resolve a scale-degree token (`^1` through `^7`, optional octave `^53`) to a MIDI note.
fn resolve_scale_degree(core: &str, scale: &Scale) -> Option<u8> {
    if !core.starts_with('^') || core.len() < 2 {
        return None;
    }
    let digits = &core[1..];
    let first = digits.as_bytes().first()?;
    if !first.is_ascii_digit() {
        return None;
    }
    let degree = (first - b'0') as usize;
    let scale_notes = scale.notes();
    if degree == 0 || degree > scale_notes.len() {
        return None;
    }
    let note = &scale_notes[degree - 1];

    let octave = if digits.len() >= 2 {
        let second = digits.as_bytes()[1];
        if second.is_ascii_digit() {
            (second - b'0') as i8
        } else {
            4
        }
    } else {
        4
    };

    let resolved = Note::new(note.pitch_class, note.accidental, octave);
    Some(resolved.to_midi())
}

/// Parse .delphi notation text into NoteEvents.
pub fn parse_notation_to_events(
    source: &str,
    channel: u8,
    program: u8,
    default_velocity: u8,
    key: Option<&str>,
) -> Vec<NoteEvent> {
    parse_notation_to_events_ts(source, channel, program, default_velocity, 4, 4, key)
}

/// Parse notation with explicit time signature for bar-mode support.
pub fn parse_notation_to_events_ts(
    source: &str,
    channel: u8,
    program: u8,
    default_velocity: u8,
    time_sig_num: u8,
    time_sig_den: u8,
    key: Option<&str>,
) -> Vec<NoteEvent> {
    let mut events: Vec<NoteEvent> = Vec::new();
    let mut tick: u32 = 0;
    let mut current_duration: u32 = 480;
    let mut velocity = default_velocity;
    let mut tie_accum: u32 = 0;
    let mut cresc_ramp: Option<(u8, u8, usize, usize)> = None;

    // Resolve key to scale for Roman numeral / scale-degree support
    let mut current_scale: Option<Scale> = key
        .and_then(|k| k.parse::<Key>().ok())
        .map(|k| k.to_scale(4));

    // Extract inline pragmas
    let mut channel = channel;
    let mut program = program;
    for line in source.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("# @instrument ") {
            program = gm_program_from_name(rest.trim());
        } else if let Some(rest) = line.strip_prefix("# @channel ") {
            if let Ok(ch) = rest.trim().parse::<u8>() {
                channel = ch;
            }
        } else if let Some(rest) = line.strip_prefix("# @velocity ") {
            if let Ok(v) = rest.trim().parse::<u8>() {
                velocity = v;
            }
        }
    }

    // ── Bar-notation detection ──
    let notation_lines: Vec<&str> = source.lines()
        .map(|l| l.trim())
        .filter(|l| {
            !l.is_empty() && !l.starts_with("//") && !l.starts_with('#')
                && !l.starts_with('@')
                && !l.starts_with("tempo(") && !l.starts_with("key(")
                && !l.starts_with("time_sig(") && !l.starts_with("swing(")
                && !l.starts_with("humanize(")
        })
        .collect();

    let is_bar_notation = notation_lines.iter().any(|line| {
        let stripped = line.trim();
        (stripped.starts_with('|') || stripped.ends_with('|') || stripped.contains(" | "))
            && !stripped.chars().all(|c| c != ' ')
    });

    if is_bar_notation {
        // Pick up any # @key or key() directives for bar notation
        for line in source.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("# @key ") {
                if let Ok(k) = rest.trim().parse::<Key>() {
                    current_scale = Some(k.to_scale(4));
                }
            } else if let Some(rest) = line.strip_prefix("key(").and_then(|r| r.strip_suffix(')')) {
                if let Ok(k) = rest.trim().parse::<Key>() {
                    current_scale = Some(k.to_scale(4));
                }
            }
        }

        let beat_ticks = 480u32 * 4 / time_sig_den.max(1) as u32;
        let measure_ticks = beat_ticks * time_sig_num as u32;
        let bar_text: String = notation_lines.join(" ");
        let bars: Vec<&str> = bar_text.split('|')
            .map(|b| b.trim())
            .filter(|b| !b.is_empty())
            .collect();

        let mut tick: u32 = 0;
        let mut cresc_ramp: Option<(u8, u8, usize, usize)> = None;
        let mut tie_accum: u32 = 0;

        for bar_str in bars {
            let tokens: Vec<&str> = bar_str.split_whitespace().collect();
            if tokens.is_empty() {
                tick += measure_ticks;
                continue;
            }
            let ticks_per_token = measure_ticks / tokens.len() as u32;
            for token in tokens {
                let prev_len = events.len();
                emit_token(token, &mut events, &mut tick, ticks_per_token, velocity, channel, program, &mut tie_accum, current_scale.as_ref());
                apply_ramp(&mut events, prev_len, &mut cresc_ramp);
            }
        }
        return events;
    }

    // Flatten source into token stream
    let mut all_tokens: Vec<String> = Vec::new();

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        // Mid-piece key change: inject sentinel token
        if let Some(rest) = line.strip_prefix("# @key ") {
            all_tokens.push(format!("__key__{}", rest.trim()));
            continue;
        }
        if let Some(rest) = line.strip_prefix("key(").and_then(|r| r.strip_suffix(')')) {
            all_tokens.push(format!("__key__{}", rest.trim()));
            continue;
        }
        if line.starts_with('#') || line.starts_with('@') {
            continue;
        }
        if line.starts_with("tempo(")
            || line.starts_with("time_sig(") || line.starts_with("swing(")
            || line.starts_with("humanize(")
        {
            continue;
        }
        all_tokens.extend(
            line.split_whitespace()
                .filter(|t| *t != "|")
                .map(|t| t.to_string())
        );
    }

    // ── Pre-process: volta brackets [1 ... [2 ... ──
    {
        let source_flat: String = all_tokens.join(" ");
        if source_flat.contains("[1") && source_flat.contains("[2") {
            let parts_v: Vec<&str> = source_flat.splitn(2, "[1").collect();
            let main_body = parts_v[0].trim();
            let rest = if parts_v.len() > 1 { parts_v[1] } else { "" };
            let vparts: Vec<&str> = rest.splitn(2, "[2").collect();
            let ending1 = vparts[0].trim().trim_matches('|').trim();
            let ending2 = if vparts.len() > 1 { vparts[1].trim().trim_matches('|').trim() } else { "" };

            let pass1 = format!("{} {}", main_body, ending1);
            let pass2 = format!("{} {}", main_body, ending2);
            let combined = format!("{} {}", pass1, pass2);
            all_tokens = combined.split_whitespace()
                .filter(|t| *t != "|" && *t != "[1" && *t != "[2")
                .map(|t| t.to_string())
                .collect();
        }
    }

    let len = all_tokens.len();
    let mut i = 0;

    while i < len {
        let raw = &all_tokens[i];

        if raw == "|" { i += 1; continue; }

        // ── Mid-piece key change sentinel ──
        if let Some(key_str) = raw.strip_prefix("__key__") {
            if let Ok(k) = key_str.parse::<Key>() {
                current_scale = Some(k.to_scale(4));
            }
            i += 1;
            continue;
        }

        // ── Slow sequence: <C4 E4 G4> ──
        if raw.starts_with('<') {
            let mut sub_tokens: Vec<String> = Vec::new();
            let first = raw.trim_start_matches('<');
            if !first.is_empty() {
                let first = first.trim_end_matches('>');
                if !first.is_empty() {
                    sub_tokens.push(first.to_string());
                }
            }
            if !raw.contains('>') {
                i += 1;
                while i < len {
                    let t = &all_tokens[i];
                    let is_end = t.contains('>');
                    let clean = t.trim_end_matches('>');
                    if !clean.is_empty() {
                        sub_tokens.push(clean.to_string());
                    }
                    i += 1;
                    if is_end { break; }
                }
            } else {
                i += 1;
            }
            if let Some(chosen) = sub_tokens.first() {
                let prev_len = events.len();
                emit_token(chosen, &mut events, &mut tick, current_duration, velocity, channel, program, &mut tie_accum, current_scale.as_ref());
                apply_ramp(&mut events, prev_len, &mut cresc_ramp);
            }
            continue;
        }

        // ── Structural markers (skip) ──
        if matches!(raw.as_str(), "DC" | "D.C." | "DS" | "D.S." | "segno" | "fine" | "coda") {
            i += 1;
            continue;
        }

        if raw == "breath" { tick += 120; i += 1; continue; }
        if raw == "caesura" { tick += 240; i += 1; continue; }

        // ── Standalone duration change ──
        if raw.starts_with(':') && raw.len() > 1 {
            if let Some(d) = Duration::from_suffix(&raw[1..]) {
                current_duration = d.ticks;
            }
            i += 1;
            continue;
        }

        // ── Standalone dynamic ──
        if raw.starts_with('!') && raw.len() > 1 {
            if let Some(v) = Dynamic::velocity_from_str(&raw[1..]) {
                velocity = v;
            }
            i += 1;
            continue;
        }

        // ── Subdivision: [C4 E4 G4] ──
        if raw.starts_with('[') || raw == "[" {
            let mut sub_tokens: Vec<String> = Vec::new();
            let first = raw.trim_start_matches('[');
            if !first.is_empty() {
                let first = first.trim_end_matches(']');
                if !first.is_empty() {
                    sub_tokens.push(first.to_string());
                }
            }
            if !raw.contains(']') {
                i += 1;
                while i < len {
                    let t = &all_tokens[i];
                    let is_end = t.contains(']');
                    let clean = t.trim_end_matches(']');
                    if !clean.is_empty() {
                        sub_tokens.push(clean.to_string());
                    }
                    i += 1;
                    if is_end { break; }
                }
            } else {
                i += 1;
            }

            if !sub_tokens.is_empty() {
                let sub_dur = current_duration / sub_tokens.len() as u32;
                for st in &sub_tokens {
                    let prev_len = events.len();
                    emit_token(st, &mut events, &mut tick, sub_dur, velocity, channel, program, &mut tie_accum, current_scale.as_ref());
                    apply_ramp(&mut events, prev_len, &mut cresc_ramp);
                }
            }
            continue;
        }

        // ── Tuplet: (3 C4 E4 G4) ──
        if raw.starts_with('(') || raw == "(" {
            let first = raw.trim_start_matches('(');
            let mut tuplet_count: Option<u32> = None;
            let mut sub_tokens: Vec<String> = Vec::new();

            if let Ok(n) = first.parse::<u32>() {
                tuplet_count = Some(n);
            } else if !first.is_empty() {
                let first = first.trim_end_matches(')');
                if !first.is_empty() {
                    sub_tokens.push(first.to_string());
                }
            }

            if !raw.contains(')') {
                i += 1;
                while i < len {
                    let t = &all_tokens[i];
                    let is_end = t.contains(')');
                    let clean = t.trim_end_matches(')');
                    if !clean.is_empty() {
                        if tuplet_count.is_none() {
                            if let Ok(n) = clean.parse::<u32>() {
                                tuplet_count = Some(n);
                                i += 1;
                                if is_end { break; }
                                continue;
                            }
                        }
                        sub_tokens.push(clean.to_string());
                    }
                    i += 1;
                    if is_end { break; }
                }
            } else {
                i += 1;
            }

            let n = tuplet_count.unwrap_or(3);
            let total_time = current_duration * (n.saturating_sub(1)).max(1);
            let note_count = sub_tokens.len().max(1) as u32;
            let sub_dur = total_time / note_count;

            for st in &sub_tokens {
                let prev_len = events.len();
                emit_token(st, &mut events, &mut tick, sub_dur, velocity, channel, program, &mut tie_accum, current_scale.as_ref());
                apply_ramp(&mut events, prev_len, &mut cresc_ramp);
            }
            continue;
        }

        // ── Layer group: {bd(3,8) sd(2,8) hh(5,8)} ──
        if raw.starts_with('{') || raw == "{" {
            let mut sub_tokens: Vec<String> = Vec::new();
            let first = raw.trim_start_matches('{');
            if !first.is_empty() {
                let first = first.trim_end_matches('}');
                if !first.is_empty() {
                    sub_tokens.push(first.to_string());
                }
            }
            if !raw.contains('}') {
                i += 1;
                while i < len {
                    let t = &all_tokens[i];
                    let is_end = t.contains('}');
                    let clean = t.trim_end_matches('}');
                    if !clean.is_empty() {
                        sub_tokens.push(clean.to_string());
                    }
                    i += 1;
                    if is_end { break; }
                }
            } else {
                i += 1;
            }

            let base_tick = tick;
            let mut max_advance: u32 = 0;
            for st in &sub_tokens {
                let save_tick = tick;
                tick = base_tick;
                let prev_len = events.len();
                emit_token(st, &mut events, &mut tick, current_duration, velocity, channel, program, &mut tie_accum, current_scale.as_ref());
                apply_ramp(&mut events, prev_len, &mut cresc_ramp);
                let advance = tick - base_tick;
                if advance > max_advance {
                    max_advance = advance;
                }
                tick = save_tick;
            }
            tick = base_tick + max_advance;
            continue;
        }

        // ── Crescendo / Decrescendo ──
        if (raw.starts_with("cresc(") || raw.starts_with("dim(")) && raw.ends_with(')') {
            if let Some(paren) = raw.find('(') {
                let args_str = &raw[paren + 1..raw.len() - 1];
                let args: Vec<&str> = args_str.split(',').collect();
                if args.len() >= 3 {
                    let start_vel = Dynamic::velocity_from_str(args[0].trim());
                    let end_vel = Dynamic::velocity_from_str(args[1].trim());
                    if let (Some(sv), Some(ev)) = (start_vel, end_vel) {
                        if let Ok(n) = args[2].trim().parse::<usize>() {
                            if n > 0 {
                                cresc_ramp = Some((sv, ev, n, 0));
                            }
                        }
                    }
                }
            }
            i += 1;
            continue;
        }

        // ── Regular token ──
        let prev_len = events.len();
        emit_token(raw, &mut events, &mut tick, current_duration, velocity, channel, program, &mut tie_accum, current_scale.as_ref());
        apply_ramp(&mut events, prev_len, &mut cresc_ramp);
        i += 1;
    }

    events
}

/// Emit events for a single token.
fn emit_token(
    raw: &str,
    events: &mut Vec<NoteEvent>,
    tick: &mut u32,
    default_dur: u32,
    base_velocity: u8,
    channel: u8,
    program: u8,
    tie_accum: &mut u32,
    scale: Option<&Scale>,
) {
    if matches!(raw, "." | "~" | "r" | "rest" | "_") {
        *tick += default_dur;
        return;
    }

    let parts = split_token(raw);

    if matches!(parts.core, "." | "~" | "r" | "rest" | "_") {
        let dur = parts.duration.unwrap_or(default_dur);
        let effective_dur = (dur as f64 * parts.weight as f64) as u32;
        for _ in 0..parts.repeat {
            *tick += effective_dur;
        }
        return;
    }

    // Random choice: C4|E4|G4
    if raw.contains('|') && !raw.starts_with('|') && !raw.ends_with('|') {
        let choices: Vec<&str> = raw.split('|').filter(|s| !s.is_empty()).collect();
        if choices.len() > 1 {
            let chosen = choices[rand::thread_rng().gen_range(0..choices.len())];
            emit_token(chosen, events, tick, default_dur, base_velocity, channel, program, tie_accum, scale);
            return;
        }
    }

    // Random removal
    if let Some(prob) = parts.random_prob {
        if rand::thread_rng().gen::<f32>() < prob {
            *tick += parts.duration.unwrap_or(default_dur);
            return;
        }
    }

    let core = parts.core;
    if core.is_empty() { return; }

    let dur = parts.duration.unwrap_or(default_dur);
    let vel_raw = parts.inline_velocity.unwrap_or(base_velocity);
    let weight = parts.weight;
    let repeat = parts.repeat;

    let effective_dur = (dur as f64 * weight as f64) as u32;

    let (art_dur, art_vel) = if let Some(ref art) = parts.articulation {
        if art.vel_mult >= 0.0 {
            let d = (effective_dur as f32 * art.dur_mult) as u32;
            let v = ((vel_raw as f32 * art.vel_mult).round() as u8).min(127);
            (d, v)
        } else {
            (effective_dur, vel_raw)
        }
    } else {
        (effective_dur, vel_raw)
    };

    let is_ornament = parts.articulation.as_ref().map_or(false, |a| a.vel_mult < 0.0);
    let ornament_tag = parts.articulation.as_ref().map(|a| a.vel_mult).unwrap_or(0.0);

    let tie_next = parts.tie_next;

    // Euclidean pattern: name(hits,steps[,offset])
    if core.contains('(') && core.ends_with(')') {
        if let Some(paren) = core.find('(') {
            let name = &core[..paren];
            let args = &core[paren + 1..core.len() - 1];
            let arg_parts: Vec<&str> = args.split(',').collect();
            if arg_parts.len() >= 2 {
                if let (Ok(hits), Ok(steps)) = (
                    arg_parts[0].trim().parse::<u32>(),
                    arg_parts[1].trim().parse::<u32>(),
                ) {
                    let offset = if arg_parts.len() >= 3 {
                        arg_parts[2].trim().parse::<usize>().unwrap_or(0)
                    } else {
                        0
                    };
                    let midi = drum_name_to_midi(name);
                    if midi > 0 {
                        let mut pattern = euclidean(hits as usize, steps as usize);
                        if offset > 0 && !pattern.is_empty() {
                            let off = offset % pattern.len();
                            pattern.rotate_left(off);
                        }
                        let step_dur = (dur * 4 / steps).max(1);
                        for rep in 0..repeat {
                            let rep_offset = rep * steps * step_dur;
                            for (j, &hit) in pattern.iter().enumerate() {
                                if hit {
                                    events.push(NoteEvent {
                                        tick: *tick + rep_offset + j as u32 * step_dur,
                                        midi_note: midi,
                                        velocity: art_vel,
                                        duration_ticks: step_dur / 2,
                                        channel: 9,
                                        program: 0,
                                    });
                                }
                            }
                        }
                        *tick += repeat * steps * step_dur;
                        return;
                    }
                }
            }
        }
    }

    // Drum name
    let drum_midi = drum_name_to_midi(core);
    if drum_midi > 0 {
        for _ in 0..repeat {
            if is_ornament {
                let expanded = expand_ornament(ornament_tag, drum_midi, art_dur / 2);
                let mut t = *tick;
                for (n, d) in expanded {
                    events.push(NoteEvent { tick: t, midi_note: n, velocity: art_vel, duration_ticks: d, channel: 9, program: 0 });
                    t += d;
                }
            } else {
                events.push(NoteEvent {
                    tick: *tick,
                    midi_note: drum_midi,
                    velocity: art_vel,
                    duration_ticks: art_dur / 2,
                    channel: 9,
                    program: 0,
                });
            }
            *tick += effective_dur;
        }
        return;
    }

    // Polyphony: C4,E4,G4 (also supports Roman numerals and scale degrees)
    if core.contains(',') {
        let parts_list: Vec<&str> = core.split(',').collect();
        for _ in 0..repeat {
            for part in &parts_list {
                let sub = split_token(part);
                let sub_core = sub.core;
                let sub_dur = sub.duration.unwrap_or(art_dur);
                let sub_vel = sub.inline_velocity.unwrap_or(art_vel);
                // Try Roman numeral
                let mut matched = false;
                if let Some(sc) = scale {
                    if let Some(midis) = resolve_roman_numeral(sub_core, sc) {
                        for midi in midis {
                            events.push(NoteEvent {
                                tick: *tick, midi_note: midi, velocity: sub_vel,
                                duration_ticks: sub_dur, channel, program,
                            });
                        }
                        matched = true;
                    } else if let Some(midi) = resolve_scale_degree(sub_core, sc) {
                        events.push(NoteEvent {
                            tick: *tick, midi_note: midi, velocity: sub_vel,
                            duration_ticks: sub_dur, channel, program,
                        });
                        matched = true;
                    }
                }
                if !matched {
                    if let Ok(note) = sub_core.parse::<Note>() {
                        events.push(NoteEvent {
                            tick: *tick,
                            midi_note: note.to_midi(),
                            velocity: sub_vel,
                            duration_ticks: sub_dur,
                            channel,
                            program,
                        });
                    } else if let Ok(chord) = sub_core.parse::<Chord>() {
                        for midi in chord.to_midi() {
                            events.push(NoteEvent {
                                tick: *tick,
                                midi_note: midi,
                                velocity: sub_vel,
                                duration_ticks: sub_dur,
                                channel,
                                program,
                            });
                        }
                    }
                }
            }
            *tick += effective_dur;
        }
        return;
    }

    // Slash chord: Am/E, C/G
    if core.contains('/') {
        if let Some(slash) = core.find('/') {
            let chord_part = &core[..slash];
            let bass_part = &core[slash + 1..];
            if let Ok(chord) = chord_part.parse::<Chord>() {
                let chord_midis = chord.to_midi();
                let bass_midi = if let Ok(note) = bass_part.parse::<Note>() {
                    Some(note.to_midi())
                } else {
                    let with_octave = format!("{}3", bass_part);
                    with_octave.parse::<Note>().ok().map(|n| n.to_midi())
                };
                for _ in 0..repeat {
                    if let Some(bm) = bass_midi {
                        events.push(NoteEvent {
                            tick: *tick, midi_note: bm, velocity: art_vel,
                            duration_ticks: art_dur, channel, program,
                        });
                    }
                    for midi in &chord_midis {
                        events.push(NoteEvent {
                            tick: *tick, midi_note: *midi, velocity: art_vel,
                            duration_ticks: art_dur, channel, program,
                        });
                    }
                    *tick += effective_dur;
                }
                return;
            }
        }
    }

    // Roman numeral chord (I, IV, V7, IVsus4, iv3dim7, etc.)
    if let Some(sc) = scale {
        if let Some(midi_notes) = resolve_roman_numeral(core, sc) {
            for _ in 0..repeat {
                if is_ornament {
                    if let Some(&root) = midi_notes.first() {
                        let expanded = expand_ornament(ornament_tag, root, art_dur);
                        let mut t = *tick;
                        for (n, d) in expanded {
                            events.push(NoteEvent { tick: t, midi_note: n, velocity: art_vel, duration_ticks: d, channel, program });
                            t += d;
                        }
                        for &midi in midi_notes.iter().skip(1) {
                            events.push(NoteEvent {
                                tick: *tick, midi_note: midi, velocity: art_vel,
                                duration_ticks: art_dur, channel, program,
                            });
                        }
                    }
                } else {
                    for &midi in &midi_notes {
                        events.push(NoteEvent {
                            tick: *tick, midi_note: midi, velocity: art_vel,
                            duration_ticks: art_dur, channel, program,
                        });
                    }
                }
                *tick += effective_dur;
            }
            return;
        }
    }

    // Chord symbol
    if let Ok(chord) = core.parse::<Chord>() {
        for _ in 0..repeat {
            if is_ornament {
                let midis = chord.to_midi();
                if let Some(&root) = midis.first() {
                    let expanded = expand_ornament(ornament_tag, root, art_dur);
                    let mut t = *tick;
                    for (n, d) in expanded {
                        events.push(NoteEvent { tick: t, midi_note: n, velocity: art_vel, duration_ticks: d, channel, program });
                        t += d;
                    }
                    for &midi in midis.iter().skip(1) {
                        events.push(NoteEvent {
                            tick: *tick, midi_note: midi, velocity: art_vel,
                            duration_ticks: art_dur, channel, program,
                        });
                    }
                }
            } else {
                for midi in chord.to_midi() {
                    events.push(NoteEvent {
                        tick: *tick, midi_note: midi, velocity: art_vel,
                        duration_ticks: art_dur, channel, program,
                    });
                }
            }
            *tick += effective_dur;
        }
        return;
    }

    // Scale degree note (^1, ^5, ^53 = degree 5 octave 3)
    if let Some(sc) = scale {
        if let Some(midi) = resolve_scale_degree(core, sc) {
            if *tie_accum > 0 {
                if let Some(last) = events.last_mut() {
                    if last.midi_note == midi {
                        last.duration_ticks += art_dur;
                        *tie_accum = 0;
                        if tie_next {
                            *tie_accum = art_dur;
                        }
                        *tick += effective_dur;
                        return;
                    }
                }
                *tie_accum = 0;
            }

            for rep in 0..repeat {
                if is_ornament {
                    let expanded = expand_ornament(ornament_tag, midi, art_dur);
                    let mut t = *tick;
                    for (n, d) in expanded {
                        events.push(NoteEvent { tick: t, midi_note: n, velocity: art_vel, duration_ticks: d, channel, program });
                        t += d;
                    }
                } else {
                    events.push(NoteEvent {
                        tick: *tick, midi_note: midi, velocity: art_vel,
                        duration_ticks: art_dur, channel, program,
                    });
                }
                if rep < repeat - 1 || !tie_next {
                    *tick += effective_dur;
                } else {
                    *tick += effective_dur;
                    *tie_accum = art_dur;
                }
            }
            return;
        }
    }

    // Note (C4, D#5, Bb3)
    if let Ok(note) = core.parse::<Note>() {
        let midi = note.to_midi();

        if *tie_accum > 0 {
            if let Some(last) = events.last_mut() {
                if last.midi_note == midi {
                    last.duration_ticks += art_dur;
                    *tie_accum = 0;
                    if tie_next {
                        *tie_accum = art_dur;
                    }
                    *tick += effective_dur;
                    return;
                }
            }
            *tie_accum = 0;
        }

        for rep in 0..repeat {
            if is_ornament {
                let expanded = expand_ornament(ornament_tag, midi, art_dur);
                let mut t = *tick;
                for (n, d) in expanded {
                    events.push(NoteEvent { tick: t, midi_note: n, velocity: art_vel, duration_ticks: d, channel, program });
                    t += d;
                }
            } else {
                events.push(NoteEvent {
                    tick: *tick, midi_note: midi, velocity: art_vel,
                    duration_ticks: art_dur, channel, program,
                });
            }
            if rep < repeat - 1 || !tie_next {
                *tick += effective_dur;
            } else {
                *tick += effective_dur;
                *tie_accum = art_dur;
            }
        }
        return;
    }

    // Unknown token — skip
}

/// Euclidean rhythm generator (Bjorklund algorithm).
pub fn euclidean(hits: usize, steps: usize) -> Vec<bool> {
    if steps == 0 {
        return vec![];
    }
    if hits == 0 {
        return vec![false; steps];
    }
    let hits = hits.min(steps);
    if hits == steps {
        return vec![true; steps];
    }
    let mut front: Vec<Vec<bool>> = vec![vec![true]; hits];
    let mut back: Vec<Vec<bool>> = vec![vec![false]; steps - hits];
    while back.len() > 1 {
        let take = front.len().min(back.len());
        let tails: Vec<Vec<bool>> = back.drain(..take).collect();
        for (i, tail) in tails.into_iter().enumerate() {
            front[i].extend(tail);
        }
        if back.is_empty() {
            let first_len = front[0].len();
            let split = front.iter().position(|g| g.len() != first_len).unwrap_or(front.len());
            if split < front.len() {
                back = front.split_off(split);
            } else {
                break;
            }
        }
    }
    front.into_iter().chain(back).flatten().collect()
}

/// Apply crescendo/decrescendo velocity ramp to newly emitted events.
fn apply_ramp(
    events: &mut [NoteEvent],
    prev_len: usize,
    ramp: &mut Option<(u8, u8, usize, usize)>,
) {
    if events.len() <= prev_len {
        return;
    }
    let done = match ramp {
        Some((sv, ev, total, applied)) if *applied < *total => {
            let t = if *total <= 1 {
                1.0
            } else {
                *applied as f32 / (*total - 1) as f32
            };
            let vel = ((*sv as f32 + t * (*ev as f32 - *sv as f32)).round() as u8).min(127);
            for event in events[prev_len..].iter_mut() {
                event.velocity = vel;
            }
            *applied += 1;
            *applied >= *total
        }
        _ => false,
    };
    if done {
        *ramp = None;
    }
}

/// Apply swing to events by delaying offbeat eighth notes.
pub fn apply_swing(events: &mut [NoteEvent], amount: f32) {
    if amount.abs() < 0.001 {
        return;
    }
    let tpq: u32 = 480;
    let eighth: u32 = tpq / 2;
    let max_shift = tpq / 3;
    let shift = (max_shift as f32 * amount) as u32;
    for ev in events.iter_mut() {
        if ev.tick % tpq == eighth {
            ev.tick += shift;
            ev.duration_ticks = ev.duration_ticks.saturating_sub(shift);
        }
    }
}

/// Apply humanize by adding random jitter to timing and velocity.
pub fn apply_humanize(events: &mut [NoteEvent], amount: f32) {
    if amount.abs() < 0.001 {
        return;
    }
    let mut rng = rand::thread_rng();
    let tpq: f32 = 480.0;
    let tick_range = (tpq * 0.04 * amount) as i32;
    let vel_range = (12.0 * amount) as i32;
    for ev in events.iter_mut() {
        if tick_range > 0 {
            let jitter = rng.gen_range(-tick_range..=tick_range);
            ev.tick = (ev.tick as i32 + jitter).max(0) as u32;
        }
        if vel_range > 0 {
            let jitter = rng.gen_range(-vel_range..=vel_range);
            ev.velocity = (ev.velocity as i32 + jitter).clamp(1, 127) as u8;
        }
    }
}

/// Parse notation and return events PLUS warning strings for unknown tokens.
pub fn parse_notation_with_diagnostics(
    source: &str,
    channel: u8,
    program: u8,
    default_velocity: u8,
    key: Option<&str>,
) -> (Vec<NoteEvent>, Vec<String>) {
    let events = parse_notation_to_events(source, channel, program, default_velocity, key);
    let mut warnings = Vec::new();

    // Resolve scale for Roman numeral / scale degree validation
    let scale: Option<Scale> = key
        .and_then(|k| k.parse::<Key>().ok())
        .map(|k| k.to_scale(4));

    for (line_no, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') || line.starts_with('@') {
            continue;
        }
        if line.starts_with("tempo(") || line.starts_with("key(")
            || line.starts_with("time_sig(") || line.starts_with("swing(")
            || line.starts_with("humanize(")
        {
            continue;
        }
        for raw in line.split_whitespace() {
            let token = raw.trim_matches(|c: char| matches!(c, '[' | ']' | '(' | ')' | '{' | '}' | '<' | '>'));
            if token.is_empty() { continue; }
            if token == "|" { continue; }
            if matches!(token, "[1" | "[2" | "[3") { continue; }
            if matches!(token, "DC" | "D.C." | "DS" | "D.S." | "segno" | "fine" | "coda" | "breath" | "caesura") {
                continue;
            }
            if token.starts_with(':') || token.starts_with('!') { continue; }
            if matches!(token, "." | "~" | "r" | "rest" | "_") { continue; }
            if token.contains('|') && !token.starts_with('|') && !token.ends_with('|') {
                let all_valid = token.split('|').filter(|s| !s.is_empty()).all(|choice| {
                    let p = split_token(choice);
                    let c = p.core;
                    c.parse::<Note>().is_ok()
                        || c.parse::<Chord>().is_ok()
                        || drum_name_to_midi(c) > 0
                        || matches!(c, "." | "~" | "r" | "rest" | "_")
                        || scale.as_ref().and_then(|sc| resolve_roman_numeral(c, sc)).is_some()
                        || scale.as_ref().and_then(|sc| resolve_scale_degree(c, sc)).is_some()
                });
                if all_valid { continue; }
            }
            let parts = split_token(token);
            let core = parts.core;
            if core.is_empty() { continue; }
            if matches!(core, "." | "~" | "r" | "rest" | "_") { continue; }
            if core.parse::<u32>().is_ok() { continue; }
            if drum_name_to_midi(core) > 0 { continue; }
            if core.contains('(') && core.ends_with(')') { continue; }
            if core.contains(',') { continue; }
            if core.contains('/') {
                if let Some(slash) = core.find('/') {
                    if core[..slash].parse::<Chord>().is_ok() { continue; }
                }
            }
            // Roman numeral or scale degree
            if let Some(sc) = &scale {
                if resolve_roman_numeral(core, sc).is_some() { continue; }
                if resolve_scale_degree(core, sc).is_some() { continue; }
            }
            if core.parse::<Chord>().is_ok() { continue; }
            if core.parse::<Note>().is_ok() { continue; }

            warnings.push(format!("line {}: unknown token '{}'", line_no + 1, core));
        }
    }

    (events, warnings)
}

// ─── Token Span Computation (for real-time highlighting) ──────────────

/// A span in a cell's source text mapped to a playback tick range.
#[derive(Debug, Clone)]
pub struct TokenSpan {
    pub byte_start: usize,
    pub byte_end: usize,
    pub tick_start: u32,
    pub tick_end: u32,
}

/// Compute token-to-tick spans for a cell's notation source text.
pub fn compute_token_spans(
    source: &str,
    time_sig_num: u8,
    time_sig_den: u8,
) -> Vec<TokenSpan> {
    let mut spans = Vec::new();

    let mut notation_tokens: Vec<(&str, usize, usize)> = Vec::new();
    let mut is_bar = false;

    let mut line_byte_start: usize = 0;
    for line in source.split('\n') {
        let trimmed = line.trim();
        let skip = trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with('@')
            || trimmed.starts_with("tempo(")
            || trimmed.starts_with("key(")
            || trimmed.starts_with("time_sig(")
            || trimmed.starts_with("swing(")
            || trimmed.starts_with("humanize(");

        if !skip {
            if (trimmed.starts_with('|') || trimmed.ends_with('|') || trimmed.contains(" | "))
                && !trimmed.chars().all(|c| c != ' ')
            {
                is_bar = true;
            }

            let mut i = 0;
            let bytes = line.as_bytes();
            while i < bytes.len() {
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i >= bytes.len() {
                    break;
                }
                let word_start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                let word = &line[word_start..i];
                notation_tokens.push((word, line_byte_start + word_start, line_byte_start + i));
            }
        }

        line_byte_start += line.len() + 1;
    }

    if notation_tokens.is_empty() {
        return spans;
    }

    let beat_ticks = 480u32 * 4 / time_sig_den.max(1) as u32;
    let measure_ticks = beat_ticks * time_sig_num as u32;

    if is_bar {
        let mut tick: u32 = 0;
        let mut bar_tokens: Vec<(&str, usize, usize)> = Vec::new();

        let flush_bar = |bar: &mut Vec<(&str, usize, usize)>,
                         tick: &mut u32,
                         spans: &mut Vec<TokenSpan>,
                         measure: u32| {
            if bar.is_empty() {
                return;
            }
            let per_token = measure / bar.len() as u32;
            for &(_, bs, be) in bar.iter() {
                spans.push(TokenSpan {
                    byte_start: bs,
                    byte_end: be,
                    tick_start: *tick,
                    tick_end: *tick + per_token,
                });
                *tick += per_token;
            }
            bar.clear();
        };

        for &(text, bs, be) in &notation_tokens {
            if text == "|" {
                flush_bar(&mut bar_tokens, &mut tick, &mut spans, measure_ticks);
            } else {
                bar_tokens.push((text, bs, be));
            }
        }
        flush_bar(&mut bar_tokens, &mut tick, &mut spans, measure_ticks);
    } else {
        let mut tick: u32 = 0;
        let mut current_dur: u32 = 480;

        let mut i = 0;
        let len = notation_tokens.len();
        while i < len {
            let (text, bs, be) = notation_tokens[i];

            if text == "|" { i += 1; continue; }

            if text.starts_with(':') && text.len() > 1 {
                if let Some(d) = Duration::from_suffix(&text[1..]) {
                    current_dur = d.ticks;
                }
                i += 1;
                continue;
            }

            if text.starts_with('!') && text.len() > 1 { i += 1; continue; }

            if matches!(text, "DC" | "D.C." | "DS" | "D.S." | "segno" | "fine" | "coda") {
                i += 1;
                continue;
            }

            if (text.starts_with("cresc(") || text.starts_with("dim(")) && text.ends_with(')') {
                i += 1;
                continue;
            }

            if text == "breath" { tick += 120; i += 1; continue; }
            if text == "caesura" { tick += 240; i += 1; continue; }

            // Bracket group [...]
            if text.starts_with('[') || text == "[" {
                let group_start = bs;
                let mut group_end = be;
                if !text.contains(']') {
                    i += 1;
                    while i < len {
                        let (t, _, e) = notation_tokens[i];
                        group_end = e;
                        let is_end = t.contains(']');
                        i += 1;
                        if is_end { break; }
                    }
                } else {
                    i += 1;
                }
                spans.push(TokenSpan {
                    byte_start: group_start,
                    byte_end: group_end,
                    tick_start: tick,
                    tick_end: tick + current_dur,
                });
                tick += current_dur;
                continue;
            }

            // Slow sequence <...>
            if text.starts_with('<') {
                let group_start = bs;
                let mut group_end = be;
                if !text.contains('>') {
                    i += 1;
                    while i < len {
                        let (t, _, e) = notation_tokens[i];
                        group_end = e;
                        let is_end = t.contains('>');
                        i += 1;
                        if is_end { break; }
                    }
                } else {
                    i += 1;
                }
                spans.push(TokenSpan {
                    byte_start: group_start,
                    byte_end: group_end,
                    tick_start: tick,
                    tick_end: tick + current_dur,
                });
                tick += current_dur;
                continue;
            }

            // Tuplet (...)
            if text.starts_with('(') || text == "(" {
                let group_start = bs;
                let mut group_end = be;
                if !text.contains(')') {
                    i += 1;
                    while i < len {
                        let (t, _, e) = notation_tokens[i];
                        group_end = e;
                        let is_end = t.contains(')');
                        i += 1;
                        if is_end { break; }
                    }
                } else {
                    i += 1;
                }
                let tuplet_dur = current_dur * 2;
                spans.push(TokenSpan {
                    byte_start: group_start,
                    byte_end: group_end,
                    tick_start: tick,
                    tick_end: tick + tuplet_dur,
                });
                tick += tuplet_dur;
                continue;
            }

            // Layer group {...}
            if text.starts_with('{') || text == "{" {
                let group_start = bs;
                let mut group_end = be;
                if !text.contains('}') {
                    i += 1;
                    while i < len {
                        let (t, _, e) = notation_tokens[i];
                        group_end = e;
                        let is_end = t.contains('}');
                        i += 1;
                        if is_end { break; }
                    }
                } else {
                    i += 1;
                }
                spans.push(TokenSpan {
                    byte_start: group_start,
                    byte_end: group_end,
                    tick_start: tick,
                    tick_end: tick + measure_ticks,
                });
                tick += measure_ticks;
                continue;
            }

            // Regular token
            let dur = span_token_duration(text, current_dur);
            spans.push(TokenSpan {
                byte_start: bs,
                byte_end: be,
                tick_start: tick,
                tick_end: tick + dur,
            });
            tick += dur;
            i += 1;
        }
    }

    spans
}

/// Compute the tick duration a token would advance.
fn span_token_duration(raw: &str, default_dur: u32) -> u32 {
    if matches!(raw, "." | "~" | "r" | "rest" | "_") {
        return default_dur;
    }

    let parts = split_token(raw);

    if matches!(parts.core, "." | "~" | "r" | "rest" | "_") {
        let dur = parts.duration.unwrap_or(default_dur);
        return (dur as f64 * parts.weight as f64) as u32 * parts.repeat;
    }

    let dur = parts.duration.unwrap_or(default_dur);
    let effective = (dur as f64 * parts.weight as f64) as u32;

    let core = parts.core;
    if core.contains('(') && core.ends_with(')') {
        if let Some(paren) = core.find('(') {
            let args = &core[paren + 1..core.len() - 1];
            let arg_parts: Vec<&str> = args.split(',').collect();
            if arg_parts.len() >= 2 {
                if let (Ok(_hits), Ok(steps)) = (
                    arg_parts[0].trim().parse::<u32>(),
                    arg_parts[1].trim().parse::<u32>(),
                ) {
                    let step_dur = (dur * 4 / steps).max(1);
                    return parts.repeat * steps * step_dur;
                }
            }
        }
    }

    effective * parts.repeat
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roman_numeral_c_major_i_iv_v() {
        // C major: I = C major, IV = F major, V = G major
        let events = parse_notation_to_events(
            "| I | IV | V | I |", 0, 0, 80, Some("C major"),
        );
        // Each bar has 1 chord = 3 notes (triad)
        assert_eq!(events.len(), 12); // 4 bars × 3 notes
        // I = C E G (60, 64, 67)
        assert_eq!(events[0].midi_note, 60);
        assert_eq!(events[1].midi_note, 64);
        assert_eq!(events[2].midi_note, 67);
        // IV = F A C (65, 69, 72)
        assert_eq!(events[3].midi_note, 65);
        assert_eq!(events[4].midi_note, 69);
        assert_eq!(events[5].midi_note, 72);
        // V = G B D (67, 71, 74)
        assert_eq!(events[6].midi_note, 67);
        assert_eq!(events[7].midi_note, 71);
        assert_eq!(events[8].midi_note, 74);
    }

    #[test]
    fn test_roman_lowercase_minor() {
        // C major: vi = Am (A C E) = 69, 72, 76
        let events = parse_notation_to_events("vi", 0, 0, 80, Some("C major"));
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].midi_note, 69); // A4
        assert_eq!(events[1].midi_note, 72); // C5
        assert_eq!(events[2].midi_note, 76); // E5
    }

    #[test]
    fn test_roman_with_quality_suffix() {
        // C major: V7 = G dominant 7 = G B D F = 67, 71, 74, 77
        let events = parse_notation_to_events("V7", 0, 0, 80, Some("C major"));
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].midi_note, 67); // G4
        assert_eq!(events[1].midi_note, 71); // B4
        assert_eq!(events[2].midi_note, 74); // D5
        assert_eq!(events[3].midi_note, 77); // F5
    }

    #[test]
    fn test_roman_with_octave_and_quality() {
        // C major: IV3sus4 = F3 sus4 = F3 Bb3 C4 = 53, 58, 60
        let events = parse_notation_to_events("IV3sus4", 0, 0, 80, Some("C major"));
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].midi_note, 53); // F3
        assert_eq!(events[1].midi_note, 58); // Bb3
        assert_eq!(events[2].midi_note, 60); // C4
    }

    #[test]
    fn test_roman_with_modifiers() {
        // IV:q!ff should work — quarter note, fortissimo
        let events = parse_notation_to_events("IV:q!ff", 0, 0, 80, Some("C major"));
        assert_eq!(events.len(), 3); // F A C
        assert_eq!(events[0].velocity, 112); // ff
        assert_eq!(events[0].duration_ticks, 480); // quarter note
    }

    #[test]
    fn test_scale_degree_notes() {
        // C major: ^1 = C4, ^3 = E4, ^5 = G4
        let events = parse_notation_to_events("^1 ^3 ^5", 0, 0, 80, Some("C major"));
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].midi_note, 60); // C4
        assert_eq!(events[1].midi_note, 64); // E4
        assert_eq!(events[2].midi_note, 67); // G4
    }

    #[test]
    fn test_scale_degree_with_octave() {
        // C major: ^53 = G3 = 55
        let events = parse_notation_to_events("^53", 0, 0, 80, Some("C major"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].midi_note, 55); // G3
    }

    #[test]
    fn test_key_change_midpiece() {
        // Start in C major (I = C), then change to G major (I = G)
        let source = "I\n# @key G major\nI";
        let events = parse_notation_to_events(source, 0, 0, 80, Some("C major"));
        // First I: C major chord (C E G)
        assert_eq!(events[0].midi_note, 60); // C4
        // After key change, second I: G major chord (G B D)
        assert_eq!(events[3].midi_note, 67); // G4
    }

    #[test]
    fn test_no_key_roman_ignored() {
        // Without a key, Roman numerals should be treated as unknown (skipped)
        let events = parse_notation_to_events("I IV V", 0, 0, 80, None);
        assert!(events.is_empty());
    }

    #[test]
    fn test_roman_in_different_key() {
        // G major: I = G, IV = C, V = D
        let events = parse_notation_to_events("| I | IV | V |", 0, 0, 80, Some("G major"));
        assert_eq!(events.len(), 9); // 3 bars × 3 notes
        assert_eq!(events[0].midi_note, 67); // G4
        assert_eq!(events[3].midi_note, 60); // C4
        assert_eq!(events[6].midi_note, 62); // D4
    }

    #[test]
    fn test_parse_roman_prefix() {
        assert_eq!(parse_roman_prefix("I"), Some((1, true, "")));
        assert_eq!(parse_roman_prefix("IV"), Some((4, true, "")));
        assert_eq!(parse_roman_prefix("IVsus4"), Some((4, true, "sus4")));
        assert_eq!(parse_roman_prefix("viidim7"), Some((7, false, "dim7")));
        assert_eq!(parse_roman_prefix("V7"), Some((5, true, "7")));
        assert_eq!(parse_roman_prefix("C4"), None); // Not a Roman numeral
        assert_eq!(parse_roman_prefix("Am"), None);
    }
}
