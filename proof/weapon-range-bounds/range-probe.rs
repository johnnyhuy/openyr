use yr_rules::{Decimal, Rules};
use yr_sim::{combat::cells_to_leptons, terrain::Loco, Command, Fix, Sim, WeaponSpec};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let text = &args[1];
    let rules = Rules::load(&yr_formats::IniFile::parse(&format!("[VehicleTypes]\n0=TANK\n[TANK]\nPrimary=Gun\n[Gun]\nDamage=65\nROF=2\nRange={text}\nWarhead=WH\nProjectile=Bullet\nSpeed=40\n[Bullet]\nInviso=no\n[WH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n")), &yr_formats::IniFile::default());
    println!("range={text} converted={}", cells_to_leptons(Decimal::parse(text).unwrap()).raw());
    let Some(weapon) = WeaponSpec::from_rules(&rules, "Gun") else { println!("weapon=none"); return; };
    println!("weapon={}", weapon.range.raw());
    let spawn = |x, weapon| Command::Spawn {kind:0, x:Fix::from_int(x),y:Fix::ZERO,loco:Loco::Track,speed:Fix::ZERO,hp:1000,armor:5,weapon};
    let mut sim=Sim::new(42);
    sim.step(&[spawn(0,Some(weapon)),spawn(4,None),Command::Attack{id:1,target:2}]);
    sim.drain_events();
    let bytes=match sim.save_bytes() {Ok(v)=>v,Err(e)=>{println!("save_error={e}");return;}};
    let mut restored=Sim::from_save_bytes(&bytes).unwrap();
    let mut trace=Vec::new();
    for _ in 0..16 {
        sim.step(&[]);restored.step(&[]);
        assert_eq!(sim.drain_events(),restored.drain_events());
        assert_eq!(sim.state_hash(),restored.state_hash());
        let bytes=sim.save_bytes().unwrap();assert_eq!(bytes,restored.save_bytes().unwrap());
        trace.extend((bytes.len() as u64).to_le_bytes());trace.extend(bytes);
    }
    println!("tick={} hash={:016x} target_hp={} save_bytes={}",sim.tick(),sim.state_hash(),sim.units().iter().find(|u|u.id==2).unwrap().hp,trace.len());
    if let Some(path)=args.get(2) { std::fs::write(path,trace).unwrap(); }
}
