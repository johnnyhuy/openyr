//! sim-bench: run a scripted deterministic sim and print the state hash.
//! Usage: `sim-bench [ticks] [seed]` — identical arguments must always
//! print identical hashes (the parity acid test in miniature).

use yr_sim::{Command, Fix, Sim};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let ticks: u64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(42);
    let mut sim = Sim::new(seed);
    let start = std::time::Instant::now();
    sim.step(&[
        Command::Spawn {
            x: Fix::ZERO,
            y: Fix::ZERO,
            loco: yr_sim::terrain::Loco::Track,
            speed: Fix::from_int(4),
            hp: 100,
            armor: 0,
            weapon: None,
        },
        Command::Spawn {
            x: Fix::from_int(2560),
            y: Fix::ZERO,
            loco: yr_sim::terrain::Loco::Track,
            speed: Fix::from_int(4),
            hp: 100,
            armor: 0,
            weapon: None,
        },
    ]);
    sim.step(&[
        Command::Move {
            id: 1,
            x: Fix::from_int(25_600),
            y: Fix::from_int(12_800),
        },
        Command::Move {
            id: 2,
            x: Fix::ZERO,
            y: Fix::from_int(12_800),
        },
    ]);
    for _ in 2..ticks {
        sim.step(&[]);
    }
    println!(
        "ticks={} seed={} units={} hash=0x{:016x} elapsed={:?}",
        sim.tick(),
        seed,
        sim.units().len(),
        sim.state_hash(),
        start.elapsed()
    );
}
