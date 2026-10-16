//! Retail install tests: resolve real files through the engine-order
//! cascade. Skips when `~/Games/RA2YR/prefix/drive_c/Westwood/RA2` is absent.

use std::path::PathBuf;
use yr_assets::{AssetFs, Theater};
use yr_formats::{IniFile, ShpFile, TmpFile};

fn install() -> Option<AssetFs> {
    let dir = PathBuf::from(std::env::var("HOME").ok()?)
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    dir.is_dir().then(|| AssetFs::open(&dir).expect("install opens"))
}

#[test]
fn patch_rules_shadow_original() {
    let Some(fs) = install() else {
        eprintln!("skip: retail install not present");
        return;
    };
    // 1.001's EXPANDMD01.MIX must win over RA2MD.MIX/LOCALMD.MIX (742,958 B).
    let loc = fs.locate("RULESMD.INI").unwrap();
    assert_eq!((loc.source.as_str(), loc.size), ("EXPANDMD01.MIX", 743_215));
    let rules = IniFile::parse(&String::from_utf8_lossy(fs.read("rulesmd.ini").unwrap()));
    assert!(rules.section("General").is_some());

    assert_eq!(fs.locate("RA2MD.CSF").unwrap().source, "LANGMD.MIX");
    assert_eq!(fs.locate("UNITTEM.PAL").unwrap().source, "RA2.MIX/CACHE.MIX");
    assert_eq!(fs.locate("CEVA060.WAV").unwrap().source, "LANGMD.MIX/AUDIOMD.MIX");
}

#[test]
fn nested_archives_mount_in_engine_order() {
    let Some(fs) = install() else {
        eprintln!("skip: retail install not present");
        return;
    };
    let mounts: Vec<&str> = fs.mounts().collect();
    let pos = |m: &str| mounts.iter().position(|x| *x == m).unwrap_or_else(|| panic!("{m} mounted"));
    assert!(pos("LANGMD.MIX") < pos("EXPANDMD01.MIX"));
    assert!(pos("EXPANDMD01.MIX") < pos("RA2MD.MIX"));
    assert!(pos("RA2MD.MIX/CACHEMD.MIX") < pos("RA2.MIX/CACHE.MIX"));
    assert!(pos("RA2MD.MIX/LOCALMD.MIX") < pos("RA2.MIX/LOCAL.MIX"));
    assert!(pos("RA2MD.MIX/CONQMD.MIX") < pos("RA2.MIX/CONQUER.MIX"));
}

#[test]
fn theater_selects_terrain() {
    let Some(mut fs) = install() else {
        eprintln!("skip: retail install not present");
        return;
    };
    assert!(fs.read("CLEAR01.TEM").is_none(), "no theater mounted yet");
    fs.set_theater(Theater::Temperate);
    let tmp = TmpFile::parse(fs.read("CLEAR01.TEM").unwrap()).unwrap();
    assert_eq!((tmp.tile_width, tmp.tile_height), (60, 30));
    assert!(fs.read("GACNST.SHP").is_none(), "snow-only art");

    fs.set_theater(Theater::Snow);
    assert!(fs.read("CLEAR01.TEM").is_none(), "temperate unmounted");
    let shp = ShpFile::parse(fs.read("GACNST.SHP").unwrap()).unwrap();
    assert_eq!((shp.width, shp.height), (284, 226));
}
