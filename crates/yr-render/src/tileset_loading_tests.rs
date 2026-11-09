use super::*;

const MAP: &str = "[Map]\nTheater=TEMPERATE\nSize=0,0,2,2\nLocalSize=0,0,2,2\n[IsoMapPack5]\n1=CAAEABUAAAAAEQAA\n";
const HOSTILE: &str = "[TileSet0000]\nFileName=Clear\nTilesInSet=4294967295\n";

#[test]
fn tileset_budget_error_reaches_terrain_loading_without_template_access() {
    let mut fs = AssetFs::from_files(vec![("TEMPERATMD.INI".into(), HOSTILE.as_bytes().to_vec())]);
    let map = MapFile::parse(MAP).unwrap();
    let error = build_terrain(&mut fs, &map, &[], Theater::Temperate)
        .err()
        .unwrap();
    assert_eq!(
        error,
        "TEMPERATMD.INI: theater tile index exceeds 65536 filenames"
    );
    println!("Terrain loading returned: {error}");
}

#[test]
fn tileset_budget_error_reaches_actual_map_render_entrypoint() {
    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let dir = Scratch(std::env::temp_dir().join(
        format!("openyr-tileset-budget-{}-{}", std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
    ));
    fs::create_dir(&dir.0).unwrap();
    fs::write(dir.0.join("TEMPERATMD.INI"), HOSTILE).unwrap();
    fs::write(dir.0.join("SYNTHETIC.MAP"), MAP).unwrap();
    let error = render_map(dir.0.to_str().unwrap(), "SYNTHETIC.MAP", 1)
        .err()
        .unwrap();
    assert_eq!(
        error,
        "TEMPERATMD.INI: theater tile index exceeds 65536 filenames"
    );
    println!("Map render entrypoint returned: {error}");
}
