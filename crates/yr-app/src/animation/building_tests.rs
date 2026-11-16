use super::*;
use yr_sim::{BuildingSpec, Command, Factory, Product, Role};

fn fields(a: yr_render::sprites::BuildingAnimation) -> (u64, Option<u64>, Option<u64>, bool, bool) {
    (a.elapsed, a.production, a.unloading, a.damaged, a.powered)
}

fn place(x: u16, owner: u8, role: Role, power: i64) -> Command {
    Command::PlaceBuilding {
        kind: 0,
        x,
        y: 0,
        w: 1,
        h: 1,
        hp: 100,
        armor: 0,
        power,
        owner,
        role,
    }
}

fn compare(sim: &Sim) {
    let before = sim.save_bytes().unwrap();
    let sample = BuildingAnimations::new(sim);
    for b in sim.buildings() {
        assert_eq!(
            fields(sample.get(b)),
            fields(building_animation(sim, b)),
            "building {}",
            b.id
        );
    }
    assert_eq!(sim.save_bytes().unwrap(), before);
}

#[test]
fn shared_building_state_matches_independent_queries_through_host_churn() {
    let mut sim = Sim::new(9);
    let roles = [
        Role::Combat,
        Role::Harvester,
        Role::Refinery,
        Role::WarFactory,
        Role::Barracks,
        Role::ConstructionYard,
    ];
    let commands: Vec<_> = [0, 1, 2, 255]
        .into_iter()
        .flat_map(|owner| {
            roles.into_iter().enumerate().flat_map(move |(i, role)| {
                [
                    place(i as u16 * 2, owner, role, 100),
                    place(i as u16 * 2 + 1, owner, role, -125),
                ]
            })
        })
        .collect();
    sim.step(&commands);
    assert_eq!(sim.buildings().len(), 48);
    compare(&sim);
    let product = Product::Building(BuildingSpec {
        w: 1,
        h: 1,
        hp: 100,
        armor: 0,
        power: 10,
        role: Role::Combat,
        adjacent: 1,
        base_normal: true,
        free_unit: None,
    });
    sim.set_credits(50);
    sim.step(&[Command::Produce {
        factory: Factory::Building,
        kind: 1,
        product,
        cost: 100,
        ticks: 4,
    }]);
    for _ in 0..8 {
        compare(&sim);
        sim.step(&[]);
    }
    sim.set_credits(100);
    for _ in 0..8 {
        compare(&sim);
        sim.step(&[]);
    }
    let snapshot = BuildingAnimations::new(&sim);
    let mut damaged = sim.buildings()[0].clone();
    damaged.hp = 50;
    assert!(snapshot.get(&damaged).damaged);
    damaged.hp = 51;
    assert!(!snapshot.get(&damaged).damaged);
    let removed: Vec<_> = sim
        .buildings()
        .iter()
        .step_by(2)
        .map(|b| Command::Destroy { id: b.id })
        .collect();
    sim.step(&removed);
    assert_eq!(sim.buildings().len(), 24);
    compare(&sim);
    compare(&Sim::from_save_bytes(&sim.save_bytes().unwrap()).unwrap());
    let removed: Vec<_> = sim
        .buildings()
        .iter()
        .map(|b| Command::Destroy { id: b.id })
        .collect();
    sim.step(&removed);
    assert!(sim.buildings().is_empty());
    compare(&sim);
}

#[test]
#[ignore = "manual presentation sampling probe; no CI timing threshold"]
fn building_animation_scaling_probe() {
    use std::{hint::black_box, time::Instant};
    for count in [512, 2048, 8192] {
        let mut sim = Sim::new(7);
        sim.step(
            &(0..count)
                .map(|x| place(x, 0, Role::Combat, 1))
                .collect::<Vec<_>>(),
        );
        let start = Instant::now();
        for _ in 0..8 {
            for b in sim.buildings() {
                black_box(building_animation(black_box(&sim), b));
            }
        }
        let old = start.elapsed();
        let start = Instant::now();
        for _ in 0..8 {
            let state = BuildingAnimations::new(black_box(&sim));
            for b in sim.buildings() {
                black_box(state.get(b));
            }
        }
        println!(
            "{count} buildings: independent {:.1} us/frame, shared {:.1} us/frame (8 redraws)",
            old.as_secs_f64() * 1e6 / 8.0,
            start.elapsed().as_secs_f64() * 1e6 / 8.0
        );
    }
}
