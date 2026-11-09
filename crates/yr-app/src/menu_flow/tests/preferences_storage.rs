use super::*;

#[cfg(unix)]
#[test]
fn preferences_storage_preserves_a_preexisting_temporary_symlink_and_its_target() {
    use std::os::unix::fs::symlink;
    let dir = Scratch::new();
    let path = dir.0.join("interface.json");
    let old_temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let unrelated = dir.0.join("unrelated.txt");
    let original = b"unrelated user data must survive a preference write";
    std::fs::write(&unrelated, original).unwrap();
    symlink(&unrelated, &old_temporary).unwrap();
    let mut app = new_app(String::new(), String::new(), 1);
    app.preferences_path = Some(path.clone());
    app.menu.front = true;
    app.menu.set(Page::Controls);
    app.apply_menu_effect(Effect::EdgeScroll);
    assert_eq!(
        std::fs::read(&unrelated).unwrap(),
        original,
        "preexisting temporary symlink cannot redirect/truncate the write"
    );
    assert!(std::fs::symlink_metadata(&old_temporary)
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(std::fs::symlink_metadata(&path).unwrap().is_file());
    let mut reopened = new_app(String::new(), String::new(), 1);
    reopened.read_preferences_file(path);
    assert!(reopened.menu.edge_scroll);
    assert_eq!(app.menu.page, Page::Controls);
    println!("Actual EdgeScroll preference route preserves unrelated symlink-target bytes and publishes a regular interface.json.");
}

#[test]
fn preferences_storage_publication_error_returns_to_controls_and_allows_retry() {
    let dir = Scratch::new();
    let path = dir.0.join("interface.json");
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("keep"), b"protected destination").unwrap();
    let mut app = new_app(String::new(), String::new(), 1);
    app.preferences_path = Some(path.clone());
    app.menu.front = true;
    app.menu.set(Page::Controls);
    app.apply_menu_effect(Effect::EdgeScroll);
    assert!(matches!(app.menu.page, Page::Message { .. }));
    assert_eq!(
        std::fs::read(path.join("keep")).unwrap(),
        b"protected destination"
    );
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
    let effect = app.menu.escape();
    app.apply_menu_effect(effect);
    assert_eq!(app.menu.page, Page::Controls);
    // Remove only the fixture's owned obstruction, then use the same UI route.
    std::fs::remove_file(path.join("keep")).unwrap();
    std::fs::remove_dir(&path).unwrap();
    app.apply_menu_effect(Effect::EdgeScroll);
    assert_eq!(app.menu.page, Page::Controls);
    let mut reopened = new_app(String::new(), String::new(), 1);
    reopened.read_preferences_file(path);
    assert!(!reopened.menu.edge_scroll);
    println!("Actual failed EdgeScroll write preserves the destination, recovers to Controls, and retries successfully after the owned obstruction is removed.");
}
