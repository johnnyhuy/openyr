use yr_assets::AssetFs;
use yr_formats::{CsfFile, IniFile};

pub const VISIBLE: usize = 18;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapChoice {
    pub asset: String,
    pub title: String,
}

/// The packet's active order is the displayed order, not a title sort.
pub fn catalog(fs: &AssetFs) -> Vec<MapChoice> {
    let Some(bytes) = fs.read("MISSIONSMD.PKT") else {
        return Vec::new();
    };
    let packet = IniFile::parse(&String::from_utf8_lossy(bytes));
    let strings = fs.read("RA2MD.CSF").and_then(|b| CsfFile::parse(b).ok());
    from_packet(&packet, strings.as_ref())
        .into_iter()
        .filter(|entry| fs.read(&entry.asset).is_some())
        .collect()
}

fn from_packet(packet: &IniFile, strings: Option<&CsfFile>) -> Vec<MapChoice> {
    let Some(active) = packet.section("MultiMaps") else {
        return Vec::new();
    };
    active
        .entries
        .iter()
        .take(4096)
        .filter_map(|(_, name)| {
            let section = packet.section(name)?;
            if !section
                .get("GameMode")?
                .split(',')
                .any(|mode| mode.trim().eq_ignore_ascii_case("standard"))
            {
                return None;
            }
            let asset = format!("{name}.MAP");
            if !crate::snapshot::valid_map_reference(&asset) {
                return None;
            }
            let key = section.get("Description").unwrap_or(name);
            let title = strings.and_then(|s| s.get(key)).unwrap_or(key);
            Some(MapChoice {
                asset,
                title: title.chars().take(128).collect(),
            })
        })
        .collect()
}

#[derive(Debug, Clone, Default)]
pub struct MapList {
    pub entries: Vec<MapChoice>,
    pub selected: usize,
    pub top: usize,
    pub ready: Option<usize>,
    pub error: Option<String>,
    pub list_focus: bool,
    pub drag: Option<(i32, usize)>,
}

impl MapList {
    pub fn open(&mut self, asset: &str) {
        self.selected = self
            .entries
            .iter()
            .position(|m| m.asset.eq_ignore_ascii_case(asset))
            .unwrap_or(0);
        self.top = self
            .selected
            .min(self.entries.len().saturating_sub(VISIBLE));
        self.ready = None;
        self.error = None;
        self.list_focus = true;
        self.drag = None;
    }
    pub fn select(&mut self, index: usize) {
        if index >= self.entries.len() {
            return;
        }
        if index != self.selected || self.error.is_some() {
            self.ready = None;
            self.error = None;
        }
        self.selected = index;
        if index < self.top {
            self.top = index;
        } else if index >= self.top + VISIBLE {
            self.top = index + 1 - VISIBLE;
        }
    }
    pub fn move_selection(&mut self, delta: i32) {
        let index = (self.selected as i64 + i64::from(delta))
            .clamp(0, self.entries.len().saturating_sub(1) as i64);
        self.select(index as usize);
    }
    pub fn scroll(&mut self, delta: i32) {
        self.top = (self.top as i64 + i64::from(delta))
            .clamp(0, self.entries.len().saturating_sub(VISIBLE) as i64)
            as usize;
    }
    pub fn row_at(&self, (x, y): (i32, i32)) -> Option<usize> {
        if !(338..515).contains(&x) || !(128..470).contains(&y) {
            return None;
        }
        let index = self.top + ((y - 128) / 19) as usize;
        (index < self.entries.len()).then_some(index)
    }
    pub fn thumb(&self) -> (i32, i32) {
        // The measured 800x600 retail scrollbar has a 299px track and a
        // fixed 41px thumb. Its ends are separate 2px PCX caps.
        let range = self.entries.len().saturating_sub(VISIBLE);
        let offset = (self.top * 258).checked_div(range).unwrap_or(0);
        (150 + offset as i32, 41)
    }

    pub fn drag_to(&mut self, y: i32) {
        if let Some((anchor, top)) = self.drag {
            let (_, height) = self.thumb();
            let range = self.entries.len().saturating_sub(VISIBLE) as i64;
            let delta = (i64::from(y) - i64::from(anchor)) * range / i64::from(299 - height).max(1);
            self.top = (top as i64 + delta).clamp(0, range) as usize;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packet_uses_active_order_and_exact_mode_tokens() {
        let ini = IniFile::parse("[MultiMaps]\n8=B\n2=A\n7=../bad\n3=C\n[B]\nDescription=Second\nGameMode=meatgrind, STANDARD\n[A]\nDescription=First\nGameMode=standard\n[../bad]\nGameMode=standard\n[C]\nGameMode=nonstandard\n[Inactive]\nGameMode=standard\n");
        let rows = from_packet(&ini, None);
        assert_eq!(
            rows.iter().map(|m| m.asset.as_str()).collect::<Vec<_>>(),
            ["B.MAP", "A.MAP"]
        );
        assert!(from_packet(&IniFile::default(), None).is_empty());
    }

    #[test]
    fn list_selection_scrolling_and_dragging_stay_within_the_catalog() {
        let mut list = MapList {
            entries: (0..100)
                .map(|i| MapChoice {
                    asset: format!("M{i}.MAP"),
                    title: i.to_string(),
                })
                .collect(),
            ..Default::default()
        };
        list.open("M50.MAP");
        assert_eq!((list.selected, list.top), (50, 50));
        assert_eq!(list.row_at((340, 128)), Some(50));
        for point in [(337, 128), (515, 128), (340, 470)] {
            assert_eq!(list.row_at(point), None);
        }
        list.scroll(i32::MAX);
        assert_eq!(list.top, 82);
        list.move_selection(i32::MIN);
        assert_eq!((list.selected, list.top), (0, 0));
        list.ready = Some(0);
        list.scroll(40);
        list.select(0);
        assert_eq!(list.top, 0);
        assert_eq!(list.ready, Some(0));
        list.drag = Some((150, 0));
        list.drag_to(i32::MAX);
        assert_eq!(list.top, 82);
        list.drag_to(i32::MIN);
        assert_eq!(list.top, 0);
        list.error = Some("failed".into());
        list.select(0);
        assert!(list.error.is_none() && list.ready.is_none());
    }
}
