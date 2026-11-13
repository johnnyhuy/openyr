//! Presentation names from the install's CSF table. Never used by the sim.

use yr_assets::AssetFs;
use yr_formats::CsfFile;
use yr_rules::TechnoType;

pub(crate) struct SidebarNames(Option<CsfFile>);

impl SidebarNames {
    pub(crate) fn load(fs: &AssetFs) -> Self {
        let table = match fs.read("RA2MD.CSF") {
            Some(bytes) => match CsfFile::parse(bytes) {
                Ok(table) => Some(table),
                Err(e) => {
                    eprintln!("OpenYR: cannot read sidebar names from RA2MD.CSF ({e})");
                    None
                }
            },
            None => {
                eprintln!("OpenYR: RA2MD.CSF missing; using rules names in the sidebar");
                None
            }
        };
        Self(table)
    }

    pub(crate) fn name(&self, techno: &TechnoType) -> String {
        if let Some(name) = techno
            .ui_name
            .as_deref()
            .and_then(|label| self.0.as_ref()?.get(label))
            .filter(|name| !name.trim().is_empty())
        {
            return name.to_string();
        }
        let fallback = techno
            .name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(&techno.id);
        eprintln!(
            "OpenYR: sidebar {} has no usable CSF value for {}; using {fallback}",
            techno.id,
            techno.ui_name.as_deref().unwrap_or("<no UIName>")
        );
        fallback.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yr_formats::{CsfEntry, IniFile};
    use yr_rules::Rules;

    #[test]
    fn csf_loaded_multi_values_keep_the_first_localized_name_and_reject_bad_counts() {
        let rules = Rules::load(
            &IniFile::parse(
                "[InfantryTypes]\n0=E1\n[E1]\nUIName=Name:GI\nName=Internal infantry\n",
            ),
            &IniFile::default(),
        );
        let techno = rules.techno("E1").unwrap();
        let mut table = b" FSC".to_vec();
        for n in [3u32, 1, 2, 0, 0] {
            table.extend(n.to_le_bytes());
        }
        table.extend(b" LBL");
        table.extend(2u32.to_le_bytes());
        table.extend(7u32.to_le_bytes());
        table.extend(b"Name:GI");
        for value in ["First name", "Second name"] {
            table.extend(b" RTS");
            table.extend((value.len() as u32).to_le_bytes());
            for unit in value.encode_utf16() {
                table.extend((!unit).to_le_bytes());
            }
        }
        let load =
            |bytes| SidebarNames::load(&AssetFs::from_files(vec![("RA2MD.CSF".into(), bytes)]));
        let names = load(table.clone());
        assert_eq!(names.name(techno), "First name");
        let entries = &names.0.as_ref().unwrap().entries;
        assert!(std::sync::Arc::ptr_eq(&entries[0].label, &entries[1].label));
        table[28..32].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(load(table).name(techno), "Internal infantry");
        println!("Native CSF localization preserves first-value lookup, shares names, and falls back after hostile counts");
    }

    #[test]
    fn localized_names_and_missing_null_empty_or_unreadable_strings() {
        let rules = Rules::load(
            &IniFile::parse(
                "[InfantryTypes]\n0=E1\n[E1]\nUIName=Name:GI\nName=Internal infantry\n",
            ),
            &IniFile::parse(""),
        );
        let mut techno = rules.techno("E1").unwrap().clone();
        let table = |value: Option<&str>| {
            SidebarNames(Some(CsfFile {
                version: 3,
                entries: vec![CsfEntry {
                    label: "NAME:gi".into(),
                    value: value.map(str::to_string),
                    sound: None,
                }],
            }))
        };
        assert_eq!(
            table(Some("Infanterie élite")).name(&techno),
            "Infanterie élite"
        );
        for strings in [table(None), table(Some("  ")), table(Some(""))] {
            assert_eq!(strings.name(&techno), "Internal infantry");
        }
        techno.ui_name = Some("Name:Missing".into());
        assert_eq!(table(Some("GI")).name(&techno), "Internal infantry");
        for files in [vec![], vec![("RA2MD.CSF".into(), b"broken".to_vec())]] {
            let strings = SidebarNames::load(&AssetFs::from_files(files));
            assert_eq!(strings.name(&techno), "Internal infantry");
            techno.name = Some(" ".into());
            assert_eq!(strings.name(&techno), "E1");
            techno.name = Some("Internal infantry".into());
        }
    }
}
