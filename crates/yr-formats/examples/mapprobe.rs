// Temporary probe: parse every retail map and test coordinate conventions.
use std::collections::HashSet;
use yr_formats::{MapFile, MixArchive};
fn main() {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    let (mut ok, mut bad) = (0, 0);
    let (mut t_xy, mut t_yx, mut t_n, mut o_xy, mut o_yx, mut o_n, mut s_in, mut s_n) = (0, 0, 0, 0, 0, 0, 0, 0);
    for top in ["multimd.mix", "mapsmd03.mix", "expandmd01.mix"] {
        let data = std::fs::read(dir.join(top)).unwrap();
        let mix = MixArchive::parse(&data).unwrap();
        for e in mix.entries() {
            let d = mix.get_by_id(e.id).unwrap();
            let text = String::from_utf8_lossy(d);
            if !text.contains("[IsoMapPack5]") { continue; }
            match MapFile::parse(&text) {
                Ok(m) => {
                    ok += 1;
                    let cells: HashSet<(u16, u16)> = m.tiles.iter().map(|t| (t.x, t.y)).collect();
                    for (x, y, _) in &m.terrain { t_n += 1; if cells.contains(&(*x, *y)) { t_xy += 1 } if cells.contains(&(*y, *x)) { t_yx += 1 } }
                    for (i, &o) in m.overlay.iter().enumerate() { if o != 0xFF { o_n += 1; let (a, b) = ((i % 512) as u16, (i / 512) as u16); if cells.contains(&(a, b)) { o_xy += 1 } if cells.contains(&(b, a)) { o_yx += 1 } } }
                    for s in &m.structures { s_n += 1; if cells.contains(&(s.x, s.y)) { s_in += 1 } }
                }
                Err(err) => { bad += 1; println!("FAIL {top}/0x{:08x}: {err}", e.id); }
            }
        }
    }
    println!("maps ok {ok} bad {bad}");
    println!("terrain {t_n}: key%1000=x hits {t_xy}, swapped {t_yx}");
    println!("overlay {o_n}: i%512=x hits {o_xy}, swapped {o_yx}");
    println!("structures {s_n}: (x,y) on tile {s_in}");
}
