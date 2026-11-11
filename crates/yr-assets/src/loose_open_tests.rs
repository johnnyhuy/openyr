use super::AssetFs;
use std::{
    fs, io,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Install(PathBuf);
impl Install {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "openyr-loose-open-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, bytes: &[u8]) {
        fs::write(self.0.join(name), bytes).unwrap();
    }
    fn second_case(&self, name: &str, bytes: &[u8]) -> bool {
        use std::io::Write;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.0.join(name))
        {
            Ok(mut file) => {
                file.write_all(bytes).unwrap();
                true
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => false,
            Err(e) => panic!("create case variant: {e}"),
        }
    }
}
impl Drop for Install {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn loose_open_rejects_ambiguous_case_names_in_either_creation_order_without_writes() {
    for names in [
        ["RULESMD.INI", "rulesmd.ini"],
        ["rulesmd.ini", "RULESMD.INI"],
    ] {
        let dir = Install::new();
        dir.write(names[0], b"[General]\nvalue=first\n");
        if !dir.second_case(names[1], b"[General]\nvalue=second\n") {
            println!(
                "Case-insensitive filesystem cannot create both names; collision fixture skipped."
            );
            return;
        }
        for _ in 0..3 {
            let error = match AssetFs::open(&dir.0) {
                Ok(_) => panic!("ambiguous loose files must not silently select one"),
                Err(error) => error,
            };
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(error.to_string().contains("RULESMD.INI"));
            assert_eq!(
                fs::read(dir.0.join(names[0])).unwrap(),
                b"[General]\nvalue=first\n"
            );
            assert_eq!(
                fs::read(dir.0.join(names[1])).unwrap(),
                b"[General]\nvalue=second\n"
            );
        }
    }
    println!("Both physical creation orders reject ambiguous rule names on repeated opens and preserve every original file byte.");
}

#[test]
fn loose_open_retains_unique_lookup_empty_file_and_directory_policies() {
    let dir = Install::new();
    dir.write("rUlEsMd.InI", b"[General]\nvalue=unique\n");
    dir.write("ignored.txt", b"");
    dir.second_case("RULESMD.INI", b"");
    fs::create_dir(dir.0.join("not-a-file.shp")).unwrap();
    {
        let assets = AssetFs::open(&dir.0).unwrap();
        assert_eq!(
            assets.read("RULESMD.INI"),
            Some(&b"[General]\nvalue=unique\n"[..])
        );
        assert_eq!(assets.read_loose("rulesmd.ini"), assets.read("RULESMD.INI"));
        assert!(assets.read("ignored.txt").is_none());
        assert!(assets.read("not-a-file.shp").is_none());
    }
    assert_eq!(
        fs::read(dir.0.join("rUlEsMd.InI")).unwrap(),
        b"[General]\nvalue=unique\n"
    );
    let ordered = AssetFs::from_files(vec![
        ("a.txt".into(), b"first".to_vec()),
        ("A.TXT".into(), b"second".to_vec()),
    ]);
    assert_eq!(ordered.read("a.txt"), Some(&b"second"[..]));
    println!("Unique mixed-case files retain exact loose/read bytes; empty files and directories stay ignored.");
}
