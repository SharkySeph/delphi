use std::path::PathBuf;
use std::process;

use delphi_core::duration::TimeSignature;
use delphi_core::{Project, NoteEvent};
use delphi_engine::soundfont::render_to_wav_full;
use delphi_midi::export::{MidiExporter, MidiTrack};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "export" => cmd_export(&args[2..]),
        "play" => cmd_play(&args[2..]),
        "info" => cmd_info(&args[2..]),
        "new" => cmd_new(&args[2..]),
        "help" | "--help" | "-h" => print_usage(),
        "version" | "--version" | "-V" => println!("delphi {}", env!("CARGO_PKG_VERSION")),
        other => {
            // Maybe it's a file path — try to export it
            let path = PathBuf::from(other);
            if path.exists() {
                cmd_export(&args[1..]);
            } else {
                eprintln!("Unknown command: {}", other);
                print_usage();
                process::exit(1);
            }
        }
    }
}

fn print_usage() {
    eprintln!(
        "Delphi — music composition toolkit

USAGE:
    delphi <command> [options]

COMMANDS:
    export <file> [--format midi|wav|both] [--output <dir>] [--sf <soundfont>]
        Export a .dstudio or .delphi file to MIDI and/or WAV.

    play <file> [--sf <soundfont>]
        Play a .dstudio or .delphi file through the audio output.

    info <file>
        Show project information (title, tempo, tracks, cells).

    new <name>
        Create a new .dstudio project file.

    help
        Show this help message.

    version
        Show version information."
    );
}

fn load_project(path: &str) -> Project {
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        eprintln!("File not found: {}", path);
        process::exit(1);
    }
    let mut project = Project::new();
    if let Err(e) = project.load(&path_buf) {
        eprintln!("Failed to load {}: {}", path, e);
        process::exit(1);
    }
    project
}

fn find_soundfont(args: &[String]) -> Option<PathBuf> {
    // Check --sf flag
    for i in 0..args.len() {
        if args[i] == "--sf" || args[i] == "--soundfont" {
            if i + 1 < args.len() {
                return Some(PathBuf::from(&args[i + 1]));
            }
        }
    }
    // Check project settings
    None
}

fn find_soundfont_auto(project: &Project) -> Option<PathBuf> {
    // From project settings
    if let Some(ref sf) = project.settings.soundfont_path {
        let p = PathBuf::from(sf);
        if p.is_file() {
            return Some(p);
        }
    }
    // Common locations
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{}/.delphi/soundfonts/default.sf2", home),
        format!("{}/.delphi/soundfonts/GeneralUser_GS.sf2", home),
        "/usr/share/sounds/sf2/default.sf2".to_string(),
        "/usr/share/soundfonts/default.sf2".to_string(),
    ];
    for c in &candidates {
        let p = PathBuf::from(c);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn collect_and_build_tracks(events: &[NoteEvent]) -> Vec<MidiTrack> {
    let mut channel_events: std::collections::HashMap<u8, Vec<&NoteEvent>> =
        std::collections::HashMap::new();
    for ev in events {
        channel_events.entry(ev.channel).or_default().push(ev);
    }

    let mut tracks = Vec::new();
    for (ch, evs) in &channel_events {
        let program = evs.first().map(|e| e.program).unwrap_or(0);
        let name = format!("Channel {}", ch);
        let mut track = MidiTrack::new(&name, *ch, program);
        for ev in evs {
            track.add_note(ev.tick, ev.midi_note, ev.velocity, ev.duration_ticks);
        }
        tracks.push(track);
    }
    tracks
}

fn cmd_export(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: delphi export <file> [--format midi|wav|both] [--output <dir>] [--sf <soundfont>]");
        process::exit(1);
    }

    let file = &args[0];
    let project = load_project(file);

    // Parse options
    let mut format = "both".to_string();
    let mut output_dir = PathBuf::from(".");
    let mut sf_override = find_soundfont(args);

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--format" | "-f" => {
                if i + 1 < args.len() {
                    format = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--output" | "-o" => {
                if i + 1 < args.len() {
                    output_dir = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--sf" | "--soundfont" => {
                if i + 1 < args.len() {
                    sf_override = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }

    let events = project.collect_events_mixed(None, 1.0);
    if events.is_empty() {
        eprintln!("No events to export (cells are empty).");
        process::exit(1);
    }

    let stem = PathBuf::from(file)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let _ = std::fs::create_dir_all(&output_dir);

    let do_midi = format == "midi" || format == "both";
    let do_wav = format == "wav" || format == "both";

    if do_midi {
        let midi_path = output_dir.join(format!("{}.mid", stem));
        let mut exporter = MidiExporter::new();
        exporter.set_tempo(project.tempo());
        exporter.set_time_signature(TimeSignature {
            numerator: project.settings.time_sig_num,
            denominator: project.settings.time_sig_den,
        });
        for track in collect_and_build_tracks(&events) {
            exporter.add_track(track);
        }
        match exporter.write_file(midi_path.to_str().unwrap_or("")) {
            Ok(()) => println!("Exported MIDI: {}", midi_path.display()),
            Err(e) => eprintln!("MIDI export failed: {}", e),
        }
    }

    if do_wav {
        let sf_path = sf_override
            .or_else(|| find_soundfont_auto(&project));
        let sf_path = match sf_path {
            Some(p) => p,
            None => {
                eprintln!("No SoundFont found. Use --sf <path> or place a .sf2 in ~/.delphi/soundfonts/");
                process::exit(1);
            }
        };
        let wav_path = output_dir.join(format!("{}.wav", stem));
        let pan = project.channel_pan_map();
        let reverb = project.channel_reverb_map();
        let delay = project.channel_delay_map();
        let volume = project.channel_volume_map();
        match render_to_wav_full(&sf_path, &events, &project.tempo(), &wav_path, &pan, &reverb, &delay, &volume) {
            Ok(()) => println!("Exported WAV: {}", wav_path.display()),
            Err(e) => eprintln!("WAV export failed: {}", e),
        }
    }
}

fn cmd_play(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: delphi play <file> [--sf <soundfont>]");
        process::exit(1);
    }

    let file = &args[0];
    let project = load_project(file);

    let sf_path = find_soundfont(args)
        .or_else(|| find_soundfont_auto(&project));
    let sf_path = match sf_path {
        Some(p) => p,
        None => {
            eprintln!("No SoundFont found. Use --sf <path> or place a .sf2 in ~/.delphi/soundfonts/");
            process::exit(1);
        }
    };

    let events = project.collect_events_mixed(None, 1.0);
    if events.is_empty() {
        eprintln!("No events to play.");
        process::exit(1);
    }

    let pan = project.channel_pan_map();
    let reverb = project.channel_reverb_map();
    let delay = project.channel_delay_map();
    let volume = project.channel_volume_map();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Handle Ctrl-C
    let stop_clone = stop.clone();
    let _ = ctrlc::set_handler(move || {
        stop_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    println!("Playing {} ({}bpm)... Press Ctrl-C to stop.", file, project.settings.bpm);

    // Convert NoteEvent to SfEvent (they're the same type via alias)
    match delphi_engine::play_with_soundfont_full(&sf_path, &events, &project.tempo(), &stop, &pan, &reverb, &delay, &volume) {
        Ok(()) => {}
        Err(e) => eprintln!("Playback error: {}", e),
    }
}

fn cmd_info(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: delphi info <file>");
        process::exit(1);
    }

    let project = load_project(&args[0]);

    println!("Title:    {}", project.settings.title);
    println!("Tempo:    {} BPM", project.settings.bpm);
    println!("Key:      {}", project.settings.key_name);
    println!("Time Sig: {}/{}", project.settings.time_sig_num, project.settings.time_sig_den);
    println!("Swing:    {:.0}%", project.settings.swing * 100.0);
    println!("Humanize: {:.0}%", project.settings.humanize * 100.0);
    println!();
    println!("Cells: {}", project.cells.len());
    for (i, cell) in project.cells.iter().enumerate() {
        let preview = cell.source.lines().next().unwrap_or("(empty)");
        let preview = if preview.len() > 60 { &preview[..60] } else { preview };
        println!("  [{}] {} — {} ch:{} «{}»",
            i, cell.cell_type, cell.instrument, cell.channel, preview);
    }
    println!();
    println!("Tracks: {}", project.tracks.len());
    for (i, track) in project.tracks.iter().enumerate() {
        println!("  [{}] {} — {} (prog:{}, ch:{})",
            i, track.name, track.instrument, track.program, track.channel);
    }
}

fn cmd_new(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: delphi new <name>");
        process::exit(1);
    }

    let name = &args[0];
    let filename = if name.ends_with(".dstudio") {
        name.clone()
    } else {
        format!("{}.dstudio", name)
    };

    let path = PathBuf::from(&filename);
    if path.exists() {
        eprintln!("File already exists: {}", filename);
        process::exit(1);
    }

    let mut project = Project::new();
    project.settings.title = name.replace(".dstudio", "").to_string();
    project.save(&path);
    println!("Created {}", filename);
}
