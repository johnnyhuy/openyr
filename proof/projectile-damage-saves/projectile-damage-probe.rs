use yr_rules::Rules;
use yr_sim::{terrain::Loco, Command, Event, Fix, Sim, WeaponSpec};
fn main() {
    let args:Vec<String>=std::env::args().collect();
    if args[1]=="load" {
        let bytes=std::fs::read(&args[2]).unwrap();
        match Sim::from_save_bytes(&bytes) {Ok(sim)=>println!("load=ok tick={} hash={:016x}",sim.tick(),sim.state_hash()),Err(e)=>println!("load=error {e}")}
        return;
    }
    let (damage,verse)=(&args[1],&args[2]);let verses=[verse.as_str();11].join(",");
    let rules=Rules::load(&yr_formats::IniFile::parse(&format!("[VehicleTypes]\n0=TANK\n[TANK]\nPrimary=Gun\n[Gun]\nDamage={damage}\nROF=60\nRange=5.75\nWarhead=WH\nProjectile=Bullet\nSpeed=40\n[Bullet]\nInviso=no\n[WH]\nVerses={verses}\n")),&yr_formats::IniFile::default());
    let weapon=WeaponSpec::from_rules(&rules,"Gun").unwrap();let expected=weapon.damage_against(5);
    let spawn=|x,weapon| Command::Spawn{kind:0,x:Fix::from_int(x),y:Fix::ZERO,loco:Loco::Track,speed:Fix::ZERO,hp:1000,armor:5,weapon};
    let mut sim=Sim::new(42);sim.step(&[spawn(0,Some(weapon)),spawn(4,None),Command::Attack{id:1,target:2}]);
    println!("Damage={damage} Verse={verse} applied={expected} flight={:?}",sim.projectiles());
    let initial=match sim.save_bytes(){Ok(v)=>v,Err(e)=>{println!("save=error {e}");return;}};
    let mut restored=Sim::from_save_bytes(&initial).unwrap();assert_eq!(sim.drain_events(),restored.drain_events());
    let mut trace=Vec::new();trace.extend((initial.len() as u64).to_le_bytes());trace.extend(initial);
    let mut hits=Vec::new();
    for _ in 0..8 {
        sim.step(&[]);restored.step(&[]);let events=sim.drain_events();assert_eq!(events,restored.drain_events());
        hits.extend(events.iter().filter_map(|e|match e{Event::Hit{target:2,damage}=>Some(*damage),_=>None}));
        assert_eq!(sim.state_hash(),restored.state_hash());let bytes=sim.save_bytes().unwrap();assert_eq!(bytes,restored.save_bytes().unwrap());
        trace.extend((bytes.len() as u64).to_le_bytes());trace.extend(bytes);
    }
    assert!(sim.projectiles().is_empty());assert_eq!(hits,if expected>0{vec![expected]}else{vec![]});
    println!("save=ok tick={} hash={:016x} hits={hits:?} target_hp={} bytes={}",sim.tick(),sim.state_hash(),sim.units().iter().find(|u|u.id==2).map_or(0,|u|u.hp),trace.len());
    if let Some(path)=args.get(3){std::fs::write(path,trace).unwrap();}
}
