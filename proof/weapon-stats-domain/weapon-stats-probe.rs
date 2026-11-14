use yr_rules::Rules;
use yr_sim::{terrain::Loco, Command, Fix, Sim, WeaponSpec};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (rof,damage,verse)=(&args[1],&args[2],&args[3]);
    let reload=if rof=="absent" {String::new()} else {format!("ROF={rof}\n")};
    let verses=[verse.as_str();11].join(",");
    let rules=Rules::load(&yr_formats::IniFile::parse(&format!("[VehicleTypes]\n0=TANK\n[TANK]\nPrimary=Gun\n[Gun]\nDamage={damage}\n{reload}Range=5.75\nWarhead=WH\n[WH]\nVerses={verses}\n")), &yr_formats::IniFile::default());
    println!("ROF={rof} Damage={damage} Verse={verse}");
    let Some(weapon)=WeaponSpec::from_rules(&rules,"Gun") else {println!("weapon=none");return;};
    println!("weapon=some reload={} damage={} verse={}",weapon.rof_ticks,weapon.damage,weapon.verses[0].units());
    let spawn=|x,weapon| Command::Spawn{kind:0,x:Fix::from_int(x),y:Fix::ZERO,loco:Loco::Track,speed:Fix::ZERO,hp:1000,armor:5,weapon};
    let mut sim=Sim::new(42);sim.step(&[spawn(0,Some(weapon)),spawn(4,None)]);sim.drain_events();
    if let Err(e)=sim.save_bytes() {println!("save_error={e}");return;}
    sim.step(&[Command::Attack{id:1,target:2}]);sim.drain_events();
    let mut restored=Sim::from_save_bytes(&sim.save_bytes().unwrap()).unwrap();
    let mut trace=Vec::new();
    for _ in 0..16 {
        sim.step(&[]);restored.step(&[]);
        assert_eq!(sim.drain_events(),restored.drain_events());assert_eq!(sim.state_hash(),restored.state_hash());
        let bytes=sim.save_bytes().unwrap();assert_eq!(bytes,restored.save_bytes().unwrap());
        trace.extend((bytes.len() as u64).to_le_bytes());trace.extend(bytes);
    }
    println!("tick={} hash={:016x} target_hp={} save_bytes={}",sim.tick(),sim.state_hash(),sim.units().iter().find(|u|u.id==2).map_or(0,|u|u.hp),trace.len());
    if let Some(path)=args.get(4) {std::fs::write(path,trace).unwrap();}
}
