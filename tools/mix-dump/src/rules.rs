//! `mix-dump rules`: load typed game data from an install through the VFS.
//!
//!   mix-dump rules <gamedir>            summary + validation issues
//!   mix-dump rules <gamedir> <ID>...    resolved techno, weapon chain, art

use std::path::Path;
use yr_assets::AssetFs;
use yr_formats::{CsfFile, IniFile};
use yr_rules::{Armor, Rules, Severity, TechnoType};

pub fn cmd_rules(args: &[&str]) -> Result<(), String> {
    let (dir, ids) = args.split_first().ok_or("usage: mix-dump rules <gamedir> [ID...]")?;
    let fs = AssetFs::open(Path::new(dir)).map_err(|e| format!("{dir}: {e}"))?;
    let ini = |name: &str| -> Result<IniFile, String> {
        let data = fs.read(name).ok_or_else(|| format!("{name} not found"))?;
        Ok(IniFile::parse(&String::from_utf8_lossy(data)))
    };
    let (rules_ini, art_ini) = (ini("RULESMD.INI")?, ini("ARTMD.INI")?);
    let rules = Rules::load(&rules_ini, &art_ini);
    let csf = fs.read("RA2MD.CSF").and_then(|d| CsfFile::parse(d).ok());

    if ids.is_empty() {
        summary(&rules);
        return Ok(());
    }
    for id in ids {
        let t = rules.techno(id).ok_or_else(|| format!("{id}: not a registered techno type"))?;
        techno(&fs, &rules, &rules_ini, &art_ini, csf.as_ref(), t);
    }
    Ok(())
}

fn summary(rules: &Rules) {
    println!(
        "infantry {}  vehicles {}  aircraft {}  buildings {}  weapons {}  warheads {}  projectiles {}",
        rules.infantry.len(),
        rules.vehicles.len(),
        rules.aircraft.len(),
        rules.buildings.len(),
        rules.weapons.len(),
        rules.warheads.len(),
        rules.projectiles.len()
    );
    println!(
        "globals   build_speed {}  refund {}  houses {}  skirmish money {} (units {})",
        opt(&rules.globals.build_speed),
        opt(&rules.globals.refund_percent),
        rules.houses.len(),
        opt(&rules.skirmish.money),
        opt(&rules.skirmish.unit_count)
    );
    let mut issues: Vec<_> = rules.issues.iter().collect();
    issues.sort_by_key(|i| std::cmp::Reverse(i.severity));
    println!("\nissues ({}):", issues.len());
    for i in issues {
        let sev = match i.severity {
            Severity::Warning => "warn",
            Severity::Note => "note",
        };
        println!("  {sev}  [{}] {}", i.section, i.message);
    }
}

fn opt<T: std::fmt::Display>(v: &Option<T>) -> String {
    v.as_ref().map_or_else(|| "(default)".into(), T::to_string)
}

fn techno(fs: &AssetFs, rules: &Rules, rules_ini: &IniFile, art_ini: &IniFile, csf: Option<&CsfFile>, t: &TechnoType) {
    let display = t
        .ui_name
        .as_deref()
        .and_then(|label| csf.and_then(|c| c.get(label)))
        .or(t.name.as_deref())
        .unwrap_or("?");
    println!("{} \u{2014} {display}", t.id);
    println!("  kind      {:?} #{}{}", t.kind, t.index, if t.defined { "" } else { " (no rules section)" });
    if let Some(label) = &t.ui_name {
        println!("  UIName    {label}");
    }
    println!(
        "  strength  {}   armor {}   cost {}   tech {}   sight {}   speed {}",
        opt(&t.strength),
        t.armor.map_or("(default)", Armor::name),
        opt(&t.cost),
        opt(&t.tech_level),
        opt(&t.sight),
        opt(&t.speed)
    );
    if !t.owner.is_empty() {
        println!("  owner     {}", t.owner.join(", "));
    }
    if !t.prerequisite.is_empty() {
        println!("  prereq    {}", t.prerequisite.join(", "));
    }

    println!("  art       [{}]", t.art.as_ref().map_or("(none)", |a| a.section.as_str()));
    let mut files = Vec::new();
    if let Some(art) = &t.art {
        if art.voxel {
            for suffix in ["", "TUR", "BARL"] {
                files.push(format!("{}{suffix}.VXL", t.image));
                files.push(format!("{}{suffix}.HVA", t.image));
            }
        } else {
            files.push(format!("{}.SHP", t.image));
        }
        files.extend(art.cameo.iter().chain(art.alt_cameo.iter()).map(|c| format!("{c}.SHP")));
    }
    for f in files {
        if let Some(loc) = fs.locate(&f) {
            println!("            {f:<18} {:>8} B  {}", loc.size, loc.source);
        }
    }

    let slots = [
        ("Primary", &t.primary),
        ("Secondary", &t.secondary),
        ("ElitePrimary", &t.elite_primary),
        ("EliteSecondary", &t.elite_secondary),
    ];
    let numbered: Vec<(String, Option<String>)> =
        t.weapons.iter().enumerate().map(|(i, w)| (format!("Weapon{}", i + 1), Some(w.clone()))).collect();
    let all = slots
        .iter()
        .map(|(k, v)| (k.to_string(), (*v).clone()))
        .chain(numbered);
    for (slot, w) in all {
        let Some(w) = w else { continue };
        let Some(weapon) = rules.weapons.get(&w) else { continue };
        println!(
            "  {slot:<14} {w}: damage {} ROF {} range {} burst {} projectile {} warhead {}",
            opt(&weapon.damage),
            opt(&weapon.rof),
            weapon.range.map_or_else(|| "(default)".into(), |r| format!("{r} ({} lp)", r.to_leptons())),
            opt(&weapon.burst),
            opt(&weapon.projectile),
            opt(&weapon.warhead)
        );
        if let (Some(wh), Some(dmg)) = (weapon.warhead.as_deref().and_then(|h| rules.warheads.get(h)), weapon.damage) {
            let row: Vec<String> = Armor::ALL
                .iter()
                .map(|&a| format!("{} {}", a.name(), wh.verse(a).mul_int(dmg)))
                .collect();
            println!("                 damage/shot: {}", row.join(", "));
        }
    }

    for (file, ini, section) in [("rulesmd.ini", rules_ini, t.id.as_str()), ("artmd.ini", art_ini, t.image.as_str())] {
        if let Some(sec) = ini.section(section) {
            println!("  {file} [{}]:", sec.name);
            for (i, (k, v)) in sec.entries.iter().enumerate() {
                let shadowed = sec.entries[i + 1..].iter().any(|(k2, _)| k2.eq_ignore_ascii_case(k));
                println!("    {k}={v}{}", if shadowed { "   (overridden below)" } else { "" });
            }
        }
    }
    println!();
}
