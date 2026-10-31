//! Known RA2/YR filenames for reverse-resolving hashed MIX entry IDs.
//!
//! MIX archives store only `mix_id(name)`, so names must be guessed and
//! confirmed by hash. Extend these lists as names are verified against retail
//! archives; see `docs/formats/mix.md`.

use crate::hash::mix_id;
use crate::ini::IniFile;
use std::collections::{HashMap, HashSet};

/// Fixed names: archives, rules, string tables, palettes.
pub const KNOWN_NAMES: &[&str] = &[
    // top-level archives and expansion packs
    "RA2.MIX",
    "RA2MD.MIX",
    "LANGUAGE.MIX",
    "LANGMD.MIX",
    "THEME.MIX",
    "THEMEMD.MIX",
    "MULTI.MIX",
    "MULTIMD.MIX",
    "MAPS01.MIX",
    "MAPS02.MIX",
    "MAPSMD03.MIX",
    "MOVIES01.MIX",
    "MOVIES02.MIX",
    "MOVMD03.MIX",
    "WDT.MIX",
    "EXPANDMD01.MIX",
    "SIDEC01.MIX",
    "SIDEC02.MIX",
    "SIDEC01MD.MIX",
    "SIDEC02MD.MIX",
    "SIDENC01.MIX",
    "SIDENC02.MIX",
    // theater archives
    "TEMPERAT.MIX",
    "SNOW.MIX",
    "URBAN.MIX",
    "URBANN.MIX",
    "DESERT.MIX",
    "LUNAR.MIX",
    "NEWURBAN.MIX",
    "TEMPERATMD.MIX",
    "SNOWMD.MIX",
    "URBANMD.MIX",
    "URBANNMD.MIX",
    "DESERTMD.MIX",
    "LUNARMD.MIX",
    "NEWURBANMD.MIX",
    "ISOTEMP.MIX",
    "ISOSNOW.MIX",
    "ISOURB.MIX",
    "ISOUBN.MIX",
    "ISODES.MIX",
    "ISOLUN.MIX",
    "ISOTEMMD.MIX",
    "ISOSNOMD.MIX",
    "ISOURBMD.MIX",
    "ISOUBNMD.MIX",
    "ISODESMD.MIX",
    "ISOLUNMD.MIX",
    "ISOGEN.MIX",
    "ISOGENMD.MIX",
    "TEM.MIX",
    "SNO.MIX",
    "URB.MIX",
    "UBN.MIX",
    "DES.MIX",
    "LUN.MIX",
    "GENERIC.MIX",
    "GENERMD.MIX",
    // engine archives
    "AUDIO.MIX",
    "AUDIOMD.MIX",
    "SOUNDS.MIX",
    "SOUND.MIX",
    "CACHE.MIX",
    "CACHEMD.MIX",
    "CONQUER.MIX",
    "CONQMD.MIX",
    "LOCAL.MIX",
    "LOCALMD.MIX",
    "CAMEO.MIX",
    "CAMEOMD.MIX",
    "NEUTRAL.MIX",
    "NEUTRALMD.MIX",
    "NTRLMD.MIX",
    "LOAD.MIX",
    "LOADMD.MIX",
    "SPEECH01.MIX",
    "SPEECH02.MIX",
    "SPEECH03.MIX",
    "ECACHE01.MIX",
    "ECACHE02.MIX",
    "ECACHE03.MIX",
    "ECACHE04.MIX",
    "ECACHE05.MIX",
    "ECACHE06.MIX",
    "ECACHE07.MIX",
    "ECACHE08.MIX",
    "ECACHE09.MIX",
    "ECACHE10.MIX",
    "ECACHEMD.MIX",
    "ELOCAL01.MIX",
    "ELOCALMD.MIX",
    "MAPSMD.MIX",
    "MIX.MIX",
    // INI / rules
    "RULESMD.INI",
    "ARTMD.INI",
    "SOUNDMD.INI",
    "RA2MD.INI",
    "RA2.INI",
    "EVAMD.INI",
    "AI.INI",
    "AIMD.INI",
    "MPMAPS.MD.INI",
    "MULTIMD.INI",
    "MAPSELMD.INI",
    "MISSIONMD.INI",
    "RULES.INI",
    "ART.INI",
    "SOUND.INI",
    "THEME.INI",
    "THEMEMD.INI",
    "EVA.INI",
    "BATTLEMD.INI",
    "BATTLE.INI",
    "KEYBOARD.INI",
    "KEYBOARDMD.INI",
    "MPMODESMD.INI",
    "TEMPERATMD.INI",
    "SNOWMD.INI",
    "URBANMD.INI",
    "DESERTMD.INI",
    "LUNARMD.INI",
    "URBANNMD.INI",
    "TEMPERAT.INI",
    "SNOW.INI",
    "URBAN.INI",
    "COOPCAMPMD.INI",
    // string tables
    "RA2.CSF",
    "RA2MD.CSF",
    "LANGUAGE.CSF",
    "LANGMD.CSF",
    // palettes
    "UNITS.PAL",
    "UNITTEM.PAL",
    "UNITSNO.PAL",
    "UNITURB.PAL",
    "UNITDES.PAL",
    "UNITLUN.PAL",
    "UNITUBN.PAL",
    "ISOTEM.PAL",
    "ISOSNO.PAL",
    "ISOURB.PAL",
    "ISODES.PAL",
    "ISOLUN.PAL",
    "ISOUBN.PAL",
    "TEMPERAT.PAL",
    "SNOW.PAL",
    "URBAN.PAL",
    "DESERT.PAL",
    "LUNAR.PAL",
    "NEWURBAN.PAL",
    "LIB.PAL",
    "ANIM.PAL",
    "ISO.PAL",
    "CAMEO.PAL",
    "GOLDNN.PAL",
    "TIBERIUM.PAL",
    "MULTI.PAL",
    "SIDE01.PAL",
    "SIDE02.PAL",
    "SIDE03.PAL",
    "MOUSEPAL.PAL",
    "PALETTE.PAL",
    "GRFXTXT.PAL",
    "UIBKGD.PAL",
    "SIDEBAR.PAL",
    "LIBTEM.PAL",
    "LIBSNO.PAL",
    "LIBURB.PAL",
    "LIBDES.PAL",
    "LIBLUN.PAL",
    "LIBUBN.PAL",
    "TEMPERAT.PAL",
    "AMMO.PAL",
    // misc
    "MOUSE.SHA",
    "MOUSE.SHP",
    "PIPS.SHP",
    "PIPS2.SHP",
    "PIPBRD.SHP",
    "SIDEBAR.SHP",
    "GRFXTXT.SHP",
    "POWERP.SHP",
    "TABS.SHP",
    "SIDE1.SHP",
    "SIDE2.SHP",
    "SIDE3.SHP",
    "VOXELS.VPL",
    "KEYS.INI",
    "CLOCK.SHP",
    "GCLOCK2.SHP",
    "CREDITS.SHP",
    "TOP.SHP",
    "RADAR.SHP",
    "SIDE2B.SHP",
    "TAB00.SHP",
    "TAB01.SHP",
    "TAB02.SHP",
    "TAB03.SHP",
    "DARKEN.SHP",
    "AUDIO.BAG",
    "AUDIO.IDX",
    "AUDIOMD.BAG",
    "AUDIOMD.IDX",
    "8POINT.FNT",
    "6POINT.FNT",
    "GRAD6FNT.FNT",
    "GAME.FNT",
    "CREDITS.TXT",
    "CREDITSMD.TXT",
    "HPMIX01.MIX",
    "SEICNC01.MIX",
    "NTPRD01.MIX",
];

/// Resolve a MIX entry ID against [`KNOWN_NAMES`].
pub fn known_name(id: u32) -> Option<&'static str> {
    KNOWN_NAMES.iter().copied().find(|n| mix_id(n) == id)
}

/// Extensions tried for bare INI tokens.
const TOKEN_EXTS: &[&str] = &[
    "SHP", "VXL", "HVA", "PAL", "PCX", "WAV", "AUD", "INI", "MIX", "CSF", "MAP", "BIK", "VPL",
    "FNT", "SHA",
];
/// Per-theater extensions used by terrain templates and theater art.
const THEATER_EXTS: &[&str] = &["TEM", "SNO", "URB", "DES", "LUN", "UBN"];
/// Theater letters, substituted into the 2nd character of building art
/// (generic, temperate, arctic/snow, urban, desert, lunar, new urban).
const THEATER_LETTERS: &[char] = &['G', 'T', 'A', 'U', 'D', 'L', 'N'];
/// Art suffixes with the extensions they appear on: voxel turret/barrel,
/// cameo icons, building buildup.
const ART_SUFFIXES: &[(&str, &[&str])] = &[
    ("TUR", &["VXL", "HVA"]),
    ("BARL", &["VXL", "HVA"]),
    ("ICON", &["SHP", "PCX"]),
    ("UICO", &["SHP", "PCX"]),
    ("MK", &["SHP"]),
];

/// Collect distinct uppercase name tokens from an INI: section names and
/// comma-separated values that look like filenames (numbers and booleans are
/// dropped).
pub fn tokens_from_ini(ini: &IniFile, out: &mut HashSet<String>) {
    let mut add = |t: &str| {
        let t = t.trim();
        let plausible = !t.is_empty()
            && t.len() <= 24
            && t.bytes().any(|b| b.is_ascii_alphabetic())
            // 8-hex-digit values are digests/IDs and only produce collisions.
            && !(t.len() == 8 && t.bytes().all(|b| b.is_ascii_hexdigit()))
            && t.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
            && !matches!(
                t.to_ascii_lowercase().as_str(),
                "yes" | "no" | "true" | "false" | "none"
            );
        if plausible {
            out.insert(t.to_ascii_uppercase());
        }
    };
    for sec in &ini.sections {
        add(&sec.name);
        for (_, v) in &sec.entries {
            v.split(',').for_each(&mut add);
        }
    }
}

/// Expand one token into candidate filenames: the token itself if it already
/// has an extension, else the stem with every known extension, art suffixes
/// on their matching extensions, and theater-letter variants for theater art.
pub fn expand_token(token: &str, mut emit: impl FnMut(&str)) {
    if token.contains('.') {
        emit(token);
        return;
    }
    let mut buf = String::with_capacity(40);
    let mut out = |stem: &str, suffix: &str, exts: &[&str], emit: &mut dyn FnMut(&str)| {
        for ext in exts {
            buf.clear();
            buf.push_str(stem);
            buf.push_str(suffix);
            buf.push('.');
            buf.push_str(ext);
            emit(&buf);
        }
    };
    for ext_set in [TOKEN_EXTS, THEATER_EXTS] {
        out(token, "", ext_set, &mut emit);
    }
    for (suffix, exts) in ART_SUFFIXES {
        out(token, suffix, exts, &mut emit);
    }
    // Building art is `<side><theater><name>`: GACNST = Allied (G) snow (A)
    // Construction Yard, GTCNSTMK = its temperate build-up. Side letters: G
    // Allied, N Soviet, Y Yuri, C civilian. Retail evidence: docs/formats/mix.md.
    if token.len() >= 3 && token.is_ascii() {
        let mut variant = String::with_capacity(token.len());
        for &l in THEATER_LETTERS {
            variant.clear();
            variant.push_str(&token[..1]);
            variant.push(l);
            variant.push_str(&token[2..]);
            for suffix in ["", "MK"] {
                out(&variant, suffix, &["SHP"], &mut emit);
                out(&variant, suffix, THEATER_EXTS, &mut emit);
            }
        }
    }
}

/// Terrain tile names from tile-set sections: `FileName=clear` with
/// `TilesInSet=N` gives `CLEAR01.TEM`, alternates `CLEAR01A.TEM`, ... for
/// every theater extension.
pub fn tile_names_from_ini(ini: &IniFile, mut emit: impl FnMut(&str)) {
    for sec in &ini.sections {
        let (Some(stem), Some(n)) = (sec.get("FileName"), sec.get_i64("TilesInSet")) else {
            continue;
        };
        let stem = stem.trim().to_ascii_uppercase();
        for i in 1..=n.clamp(0, 99) {
            for variant in ["", "A", "B", "C", "D", "E", "F", "G"] {
                for ext in THEATER_EXTS {
                    emit(&format!("{stem}{i:02}{variant}.{ext}"));
                }
            }
        }
    }
}

/// ID -> name map, seeded from [`KNOWN_NAMES`] and grown from INI candidates.
#[derive(Debug, Default, Clone)]
pub struct NameDb {
    names: HashMap<u32, String>,
}

impl NameDb {
    pub fn builtin() -> Self {
        let mut db = NameDb::default();
        for n in KNOWN_NAMES {
            db.names.insert(mix_id(n), (*n).to_string());
        }
        db
    }

    pub fn get(&self, id: u32) -> Option<&str> {
        self.names.get(&id).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Learn names derived from `inis` whose hash matches one of `wanted`.
    /// Returns the number of new IDs resolved.
    pub fn learn_from_inis(&mut self, inis: &[IniFile], wanted: &HashSet<u32>) -> usize {
        let before = self.names.len();
        let mut tokens = HashSet::new();
        for ini in inis {
            tokens_from_ini(ini, &mut tokens);
        }
        let mut learn = |name: &str| {
            let id = mix_id(name);
            if wanted.contains(&id) {
                self.names
                    .entry(id)
                    .or_insert_with(|| name.to_ascii_uppercase());
            }
        };
        for t in &tokens {
            expand_token(t, &mut learn);
        }
        for ini in inis {
            tile_names_from_ini(ini, &mut learn);
        }
        self.names.len() - before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_retail_id() {
        // Confirmed in langmd.mix: docs/formats/mix.md.
        assert_eq!(known_name(0xbd835079), Some("RA2MD.CSF"));
        assert_eq!(known_name(0xdead_beef), None);
    }

    #[test]
    fn learns_art_and_tile_names_from_ini() {
        let ini = IniFile::parse(
            "[HTNK]\nImage=HTNK\nCameo=HTNKICON\n[TileSet0001]\nFileName=clear\nTilesInSet=2\n",
        );
        let wanted: HashSet<u32> = [
            "HTNK.VXL",
            "HTNKTUR.VXL",
            "HTNKICON.SHP",
            "CLEAR02.TEM",
            "NAHAND.SHP",
        ]
        .iter()
        .map(|n| mix_id(n))
        .collect();
        let mut db = NameDb::default();
        assert_eq!(db.learn_from_inis(&[ini], &wanted), 4);
        assert_eq!(db.get(mix_id("HTNKTUR.VXL")), Some("HTNKTUR.VXL"));
        assert_eq!(db.get(mix_id("CLEAR02.TEM")), Some("CLEAR02.TEM"));
        assert_eq!(db.get(mix_id("NAHAND.SHP")), None);

        // Theater letter is the 2nd character; build-ups get MK.
        let ini = IniFile::parse("[GAPOWR]\nImage=GAPOWR\nDigest=0CBDACBC\n");
        let wanted: HashSet<u32> = [
            mix_id("GTPOWRMK.SHP"),
            mix_id("GGPOWR.SHP"),
            mix_id("NAPOWR.SHP"),
        ]
        .into();
        assert_eq!(db.learn_from_inis(&[ini], &wanted), 2);
        assert_eq!(db.get(mix_id("NAPOWR.SHP")), None);
    }
}
