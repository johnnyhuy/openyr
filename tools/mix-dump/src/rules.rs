//! `mix-dump rules`: load typed game data from an install through the VFS.
//!
//!   mix-dump rules <gamedir>            summary + validation issues
//!   mix-dump rules <gamedir> <ID>...    resolved techno, weapon chain, art

use std::io::Write;
use std::path::Path;
use yr_assets::AssetFs;
use yr_formats::{CsfFile, IniFile};
use yr_rules::{Armor, Rules, Severity, SoundData, SuperWeaponType, TechnoType};

pub fn cmd_rules(args: &[&str], stdout: &mut dyn Write) -> crate::CommandResult {
    let (dir, ids) = args
        .split_first()
        .ok_or("usage: mix-dump rules <gamedir> [ID...]")?;
    let fs = AssetFs::open(Path::new(dir)).map_err(|e| format!("{dir}: {e}"))?;
    let ini = |name: &str| -> Result<IniFile, String> {
        let data = fs.read(name).ok_or_else(|| format!("{name} not found"))?;
        Ok(IniFile::parse(&String::from_utf8_lossy(data)))
    };
    let (rules_ini, art_ini) = (ini("RULESMD.INI")?, ini("ARTMD.INI")?);
    let rules = Rules::load(&rules_ini, &art_ini);
    // Sound data is optional so partial installs still dump their rules.
    let sound = fs
        .read("SOUNDMD.INI")
        .map(|d| SoundData::load(&IniFile::parse(&String::from_utf8_lossy(d))));
    let csf = fs.read("RA2MD.CSF").and_then(|d| CsfFile::parse(d).ok());

    if ids.is_empty() {
        summary(&rules, sound.as_ref(), stdout)?;
        return Ok(());
    }
    for id in ids {
        if let Some(t) = rules.techno(id) {
            techno(&fs, &rules, &rules_ini, &art_ini, csf.as_ref(), t, stdout)?;
        } else if let Some(sw) = rules.superweapons.get(id) {
            superweapon(sw, &rules_ini, stdout)?;
        } else if let Some(ev) = sound.as_ref().and_then(|s| s.events.get(id)) {
            writeln!(stdout, "{id}: sound event #{}", ev.index)?;
            writeln!(stdout, "  sounds    {}", ev.sounds.join(" "))?;
            writeln!(stdout, "  control   {}", ev.control.join(" "))?;
            writeln!(
                stdout,
                "  volume {}  priority {}  type {}",
                opt(&ev.volume),
                opt(&ev.priority),
                ev.event_type.join(" ")
            )?;
            writeln!(stdout)?;
        } else {
            return Err(
                format!("{id}: not a registered techno, superweapon or sound event").into(),
            );
        }
    }
    Ok(())
}

fn superweapon(
    sw: &SuperWeaponType,
    rules_ini: &IniFile,
    stdout: &mut dyn Write,
) -> crate::CommandResult {
    writeln!(stdout, "{}: {}", sw.id, sw.name.as_deref().unwrap_or("?"))?;
    writeln!(
        stdout,
        "  type {}  action {}  powered {}  recharge {} min  timer {}  shell-off {}",
        opt(&sw.kind),
        opt(&sw.action),
        opt(&sw.is_powered),
        opt(&sw.recharge_time),
        opt(&sw.show_timer),
        opt(&sw.disableable_from_shell)
    )?;
    writeln!(
        stdout,
        "  image {}  weapon {}  range {}  line {}  pre-dep {}",
        opt(&sw.sidebar_image),
        opt(&sw.weapon_type),
        opt(&sw.range),
        opt(&sw.line_multiplier),
        opt(&sw.pre_dependent)
    )?;
    if let Some(sec) = rules_ini.section(&sw.id) {
        writeln!(stdout, "  rulesmd.ini [{}]:", sec.name)?;
        for (k, v) in &sec.entries {
            writeln!(stdout, "    {k}={v}")?;
        }
    }
    writeln!(stdout)?;
    Ok(())
}

fn summary(
    rules: &Rules,
    sound: Option<&SoundData>,
    stdout: &mut dyn Write,
) -> crate::CommandResult {
    writeln!(stdout,
        "infantry {}  vehicles {}  aircraft {}  buildings {}  weapons {}  warheads {}  projectiles {}  supers {}",
        rules.infantry.len(),
        rules.vehicles.len(),
        rules.aircraft.len(),
        rules.buildings.len(),
        rules.weapons.len(),
        rules.warheads.len(),
        rules.projectiles.len(),
        rules.superweapons.len()
    )?;
    match sound {
        Some(s) => writeln!(
            stdout,
            "sounds    {} events ({} with sections)  default volume {}  limit {}",
            s.events.len(),
            s.events.iter().filter(|e| e.defined).count(),
            opt(&s.defaults.volume),
            opt(&s.defaults.limit)
        )?,
        None => writeln!(stdout, "sounds    (SOUNDMD.INI not found)")?,
    }
    writeln!(
        stdout,
        "globals   build_speed {}  refund {}  houses {}  skirmish money {} (units {})",
        opt(&rules.globals.build_speed),
        opt(&rules.globals.refund_percent),
        rules.houses.len(),
        opt(&rules.skirmish.money),
        opt(&rules.skirmish.unit_count)
    )?;
    let mut issues: Vec<_> = rules.issues.iter().collect();
    issues.sort_by_key(|i| std::cmp::Reverse(i.severity));
    writeln!(stdout, "\nissues ({}):", issues.len())?;
    for i in issues {
        let sev = match i.severity {
            Severity::Warning => "warn",
            Severity::Note => "note",
        };
        writeln!(stdout, "  {sev}  [{}] {}", i.section, i.message)?;
    }
    if let Some(s) = sound {
        writeln!(stdout, "\nsound issues ({}):", s.issues.len())?;
        for i in &s.issues {
            let sev = match i.severity {
                Severity::Warning => "warn",
                Severity::Note => "note",
            };
            writeln!(stdout, "  {sev}  [{}] {}", i.section, i.message)?;
        }
    }
    Ok(())
}

fn opt<T: std::fmt::Display>(v: &Option<T>) -> String {
    v.as_ref().map_or_else(|| "(default)".into(), T::to_string)
}

fn techno(
    fs: &AssetFs,
    rules: &Rules,
    rules_ini: &IniFile,
    art_ini: &IniFile,
    csf: Option<&CsfFile>,
    t: &TechnoType,
    stdout: &mut dyn Write,
) -> crate::CommandResult {
    let display = t
        .ui_name
        .as_deref()
        .and_then(|label| csf.and_then(|c| c.get(label)))
        .or(t.name.as_deref())
        .unwrap_or("?");
    writeln!(stdout, "{}: {display}", t.id)?;
    writeln!(
        stdout,
        "  kind      {:?} #{}{}",
        t.kind,
        t.index,
        if t.defined { "" } else { " (no rules section)" }
    )?;
    if let Some(label) = &t.ui_name {
        writeln!(stdout, "  UIName    {label}")?;
    }
    writeln!(
        stdout,
        "  strength  {}   armor {}   cost {}   tech {}   sight {}   speed {}",
        opt(&t.strength),
        t.armor.map_or("(default)", Armor::name),
        opt(&t.cost),
        opt(&t.tech_level),
        opt(&t.sight),
        opt(&t.speed)
    )?;
    if !t.owner.is_empty() {
        writeln!(stdout, "  owner     {}", t.owner.join(", "))?;
    }
    if !t.prerequisite.is_empty() {
        writeln!(stdout, "  prereq    {}", t.prerequisite.join(", "))?;
    }

    writeln!(
        stdout,
        "  art       [{}]",
        t.art.as_ref().map_or("(none)", |a| a.section.as_str())
    )?;
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
        files.extend(
            art.cameo
                .iter()
                .chain(art.alt_cameo.iter())
                .map(|c| format!("{c}.SHP")),
        );
    }
    for f in files {
        if let Some(loc) = fs.locate(&f) {
            writeln!(
                stdout,
                "            {f:<18} {:>8} B  {}",
                loc.size, loc.source
            )?;
        }
    }

    let slots = [
        ("Primary", &t.primary),
        ("Secondary", &t.secondary),
        ("ElitePrimary", &t.elite_primary),
        ("EliteSecondary", &t.elite_secondary),
    ];
    let numbered: Vec<(String, Option<String>)> = t
        .weapons
        .iter()
        .enumerate()
        .map(|(i, w)| (format!("Weapon{}", i + 1), Some(w.clone())))
        .collect();
    let all = slots
        .iter()
        .map(|(k, v)| (k.to_string(), (*v).clone()))
        .chain(numbered);
    for (slot, w) in all {
        let Some(w) = w else { continue };
        let Some(weapon) = rules.weapons.get(&w) else {
            continue;
        };
        writeln!(
            stdout,
            "  {slot:<14} {w}: damage {} ROF {} range {} burst {} projectile {} warhead {}",
            opt(&weapon.damage),
            opt(&weapon.rof),
            weapon.range.map_or_else(
                || "(default)".into(),
                |r| format!("{r} ({} lp)", r.to_leptons())
            ),
            opt(&weapon.burst),
            opt(&weapon.projectile),
            opt(&weapon.warhead)
        )?;
        if let (Some(wh), Some(dmg)) = (
            weapon
                .warhead
                .as_deref()
                .and_then(|h| rules.warheads.get(h)),
            weapon.damage,
        ) {
            let row: Vec<String> = Armor::ALL
                .iter()
                .map(|&a| format!("{} {}", a.name(), wh.verse(a).mul_int(dmg)))
                .collect();
            writeln!(stdout, "                 damage/shot: {}", row.join(", "))?;
        }
    }

    for (file, ini, section) in [
        ("rulesmd.ini", rules_ini, t.id.as_str()),
        ("artmd.ini", art_ini, t.image.as_str()),
    ] {
        if let Some(sec) = ini.section(section) {
            writeln!(stdout, "  {file} [{}]:", sec.name)?;
            for (i, (k, v)) in sec.entries.iter().enumerate() {
                let shadowed = sec.entries[i + 1..]
                    .iter()
                    .any(|(k2, _)| k2.eq_ignore_ascii_case(k));
                writeln!(
                    stdout,
                    "    {k}={v}{}",
                    if shadowed {
                        "   (overridden below)"
                    } else {
                        ""
                    }
                )?;
            }
        }
    }
    writeln!(stdout)?;
    Ok(())
}
