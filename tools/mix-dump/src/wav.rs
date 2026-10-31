//! `mix-dump wav`: decode retail audio to a 16-bit PCM WAV.
//!
//! Accepts an AUD file, an IMA ADPCM WAV, or a sound-bank entry written as
//! `<archive.mix>:<path>/AUDIO.BAG#<name>` (the index is read from the
//! sibling `AUDIO.IDX`).

use crate::render::load_spec;
use std::fs;
use std::io::Write;
use yr_formats::audio::{decode_wav, encode_wav, parse_bag_index};
use yr_formats::{AudFile, Pcm};

pub fn cmd_wav(asset: &str, out: &str, stdout: &mut dyn Write) -> crate::CommandResult {
    let pcm = decode(asset)?;
    fs::write(out, encode_wav(&pcm)).map_err(|e| e.to_string())?;
    writeln!(
        stdout,
        "decoded {asset} -> {out} ({} Hz, {} ch, {} samples, {:.2} s)",
        pcm.sample_rate,
        pcm.channels,
        pcm.samples.len(),
        pcm.samples.len() as f64 / f64::from(pcm.channels) / f64::from(pcm.sample_rate)
    )?;
    Ok(())
}

fn decode(asset: &str) -> Result<Pcm, String> {
    if let Some((bag_spec, name)) = asset.split_once('#') {
        let idx_spec = bag_spec
            .strip_suffix(".BAG")
            .or_else(|| bag_spec.strip_suffix(".bag"))
            .map(|stem| format!("{stem}.IDX"))
            .ok_or("bank entries are written <...>/AUDIO.BAG#<name>")?;
        let index = parse_bag_index(&load_spec(&idx_spec)?).map_err(|e| e.to_string())?;
        let entry = index
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("{name} not in {idx_spec}"))?;
        return entry
            .decode(&load_spec(bag_spec)?)
            .map_err(|e| e.to_string());
    }
    let data = load_spec(asset)?;
    if data.starts_with(b"RIFF") {
        decode_wav(&data).map_err(|e| e.to_string())
    } else {
        AudFile::decode(&data).map_err(|e| e.to_string())
    }
}
