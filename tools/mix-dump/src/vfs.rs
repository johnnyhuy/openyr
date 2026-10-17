//! `mix-dump vfs`: show the engine-order mount cascade for a game directory
//! and where filenames resolve from.
//!
//!   mix-dump vfs <gamedir> [--theater <name>] [file...]

use std::path::Path;
use yr_assets::{AssetFs, Theater};

pub fn cmd_vfs(args: &[&str]) -> Result<(), String> {
    let (dir, rest) = args
        .split_first()
        .ok_or("usage: mix-dump vfs <gamedir> [--theater <name>] [file...]")?;
    let mut fs = AssetFs::open(Path::new(dir)).map_err(|e| format!("{dir}: {e}"))?;
    let mut files = Vec::new();
    let mut it = rest.iter();
    while let Some(&arg) = it.next() {
        if arg == "--theater" {
            let name = it.next().ok_or("--theater needs a name")?;
            let theater =
                Theater::from_map_name(name).ok_or_else(|| format!("unknown theater {name}"))?;
            fs.set_theater(theater);
        } else {
            files.push(arg);
        }
    }
    if files.is_empty() {
        println!("lookup order (loose files in {dir} first):");
        for (i, label) in fs.mounts().enumerate() {
            println!("  {:>2}. {label}", i + 1);
        }
        return Ok(());
    }
    for name in files {
        match fs.locate(name) {
            Some(loc) => println!("{name:<20} {:>10} B  {}", loc.size, loc.source),
            None => println!("{name:<20} {:>10}    not found", "-"),
        }
    }
    Ok(())
}
