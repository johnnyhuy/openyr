//! App-side retail audio: INI event routing, PCM mixing and native output.
//! Audio never consumes the simulation RNG or changes a command/state hash.

mod bank;
pub mod mixer;
mod output;

pub use bank::SoundBank;
pub use output::LiveOutput;

use crate::session::{Outcome, Session};
use bank::Cue;
use mixer::{Bus, Mixer};
use std::collections::BTreeMap;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use yr_assets::AssetFs;
use yr_sim::{Command, Event, Factory, OWNER_PLAYER};

const CAPTURE_RATE: u32 = 44100;

struct Entity {
    kind: String,
    owner: u8,
}

enum Output {
    Live(LiveOutput),
    Capture(Recorder),
}

/// Presentation state only. Retain last tick's types so a unit that fires
/// and dies in the same tick still resolves its weapon report and death sound.
pub struct AudioScene {
    bank: SoundBank,
    output: Output,
    entities: BTreeMap<u32, Entity>,
    cooldowns: BTreeMap<String, u64>,
    low_power: bool,
    no_money: bool,
    outcome: Outcome,
    error: Option<String>,
}

impl AudioScene {
    pub fn live(dir: &str, map: &str, session: &Session) -> Result<Self, String> {
        Self::new(dir, map, session, Output::Live(LiveOutput::open()?))
    }

    pub fn capture(dir: &str, map: &str, session: &Session, out: &str) -> Result<Self, String> {
        Self::new(
            dir,
            map,
            session,
            Output::Capture(Recorder::new(Path::new(out)).map_err(|e| e.to_string())?),
        )
    }

    fn new(dir: &str, map: &str, session: &Session, output: Output) -> Result<Self, String> {
        let mut scene = Self::empty(dir, map, output)?;
        scene.remember(session);
        if let Some(theme) = scene.bank.theme() {
            scene.emit(session.sim.tick(), theme);
        }
        Ok(scene)
    }

    /// One accepted UI activation, using the supplied MenuClick event definition.
    pub fn menu_click(&mut self, tick: u64) {
        if let Some(cue) = self.bank.cue("MenuClick", Bus::Effect) {
            self.emit(tick, cue);
        }
    }

    pub fn front_live(dir: &str) -> Result<Self, String> {
        let mut scene = Self::empty(dir, "", Output::Live(LiveOutput::open()?))?;
        if let Some(theme) = scene.bank.menu_theme() {
            println!("OpenYR: menu theme {}", theme.sample);
            scene.emit(0, theme);
        }
        Ok(scene)
    }

    fn empty(dir: &str, map: &str, output: Output) -> Result<Self, String> {
        let fs = AssetFs::open(Path::new(dir)).map_err(|e| e.to_string())?;
        let scene = Self {
            bank: SoundBank::new(fs, map),
            output,
            entities: BTreeMap::new(),
            cooldowns: BTreeMap::new(),
            low_power: false,
            no_money: false,
            outcome: Outcome::Ongoing,
            error: None,
        };
        Ok(scene)
    }

    fn remember(&mut self, session: &Session) {
        for (id, kind, owner) in session
            .sim
            .units()
            .iter()
            .map(|u| (u.id, u.kind, u.owner))
            .chain(
                session
                    .sim
                    .buildings()
                    .iter()
                    .map(|b| (b.id, b.kind, b.owner)),
            )
        {
            if let Some(kind) = session.kinds.get(usize::from(kind)) {
                self.entities.insert(
                    id,
                    Entity {
                        kind: kind.clone(),
                        owner,
                    },
                );
            }
        }
    }

    fn emit(&mut self, tick: u64, cue: Cue) {
        match &mut self.output {
            Output::Live(output) => output.play(cue.playback),
            Output::Capture(recorder) => {
                if let Err(e) = recorder.cue(tick, cue) {
                    self.error.get_or_insert(e.to_string());
                }
            }
        }
    }

    fn event(&mut self, tick: u64, name: &str, bus: Bus, cooldown: u64) {
        if self.cooldowns.get(name).is_some_and(|until| tick < *until) {
            return;
        }
        if let Some(cue) = self.bank.cue(name, bus) {
            self.cooldowns
                .insert(name.into(), tick.saturating_add(cooldown));
            self.emit(tick, cue);
        }
    }

    fn voice(&mut self, tick: u64, id: u32, key: &str) {
        let Some(entity) = self.entities.get(&id) else {
            return;
        };
        if entity.owner != OWNER_PLAYER {
            return;
        }
        let name = self.bank.rules.value(&entity.kind, key).map(str::to_string);
        if let Some(name) = name {
            self.event(tick, &name, Bus::Response, 3);
        }
    }

    pub fn select(&mut self, session: &Session, ids: impl IntoIterator<Item = u32>) {
        if let Some(id) = ids.into_iter().find(|id| {
            self.entities
                .get(id)
                .is_some_and(|e| e.owner == OWNER_PLAYER)
        }) {
            self.voice(session.sim.tick(), id, "VoiceSelect");
        }
    }

    /// One acknowledgment per group order, with independent EVA queueing.
    pub fn commands(&mut self, session: &Session, commands: &[Command]) {
        let tick = session.sim.tick();
        if let Some((id, key)) = commands.iter().find_map(|cmd| match *cmd {
            Command::Move { id, .. } | Command::Harvest { id } => Some((id, "VoiceMove")),
            Command::Attack { id, .. } | Command::AttackMove { id, .. } => {
                Some((id, "VoiceAttack"))
            }
            _ => None,
        }) {
            self.voice(tick, id, key);
        }
        for cmd in commands {
            if let Command::Produce { factory, .. } = cmd {
                let role = factory.role();
                if session
                    .sim
                    .buildings()
                    .iter()
                    .any(|b| b.owner == OWNER_PLAYER && b.role == role)
                {
                    let name = if *factory == Factory::Infantry {
                        "EVA_Training"
                    } else {
                        "EVA_Building"
                    };
                    self.event(tick, name, Bus::Eva, 15);
                }
            }
        }
    }

    /// Call after stepping and draining events, before the next sim tick.
    pub fn update(&mut self, session: &Session, events: &[Event]) {
        self.remember(session);
        let tick = session.sim.tick();
        for event in events {
            match *event {
                Event::RepairStarted { .. } => self.event(tick, "EVA_Repairing", Bus::Eva, 15),
                Event::Sold { .. } => self.event(tick, "EVA_StructureSold", Bus::Eva, 15),
                Event::Fired { shooter, .. } => {
                    let report = self
                        .entities
                        .get(&shooter)
                        .and_then(|e| self.bank.rules.value(&e.kind, "Primary"))
                        .and_then(|w| self.bank.rules.value(w, "Report"))
                        .map(str::to_string);
                    if let Some(report) = report {
                        for name in report.split(',').map(str::trim) {
                            self.event(tick, name, Bus::Effect, 1);
                        }
                    }
                }
                Event::Produced { id, .. }
                    if session
                        .sim
                        .units()
                        .iter()
                        .any(|u| u.id == id && u.owner == yr_sim::OWNER_PLAYER) =>
                {
                    self.event(tick, "EVA_UnitReady", Bus::Eva, 15)
                }
                Event::Ready { .. } => {
                    self.event(tick, "EVA_ConstructionComplete", Bus::Eva, 15);
                }
                Event::Placed { .. } => {
                    // `[AudioVisual] BuildingSlam=`: the placement thud.
                    let slam = self
                        .bank
                        .rules
                        .value("AudioVisual", "BuildingSlam")
                        .map(str::to_string);
                    if let Some(name) = slam {
                        self.event(tick, &name, Bus::Effect, 1);
                    }
                }
                Event::Died { id } => {
                    let sound = self
                        .entities
                        .get(&id)
                        .and_then(|e| self.bank.rules.value(&e.kind, "DieSound"))
                        .map(str::to_string);
                    if let Some(name) = sound {
                        self.event(tick, &name, Bus::Effect, 1);
                    }
                    if self
                        .entities
                        .get(&id)
                        .is_some_and(|e| e.owner == OWNER_PLAYER)
                    {
                        self.event(tick, "EVA_UnitLost", Bus::Eva, 60);
                    }
                }
                _ => {}
            }
        }
        let low_power = session.sim.low_power();
        if low_power && !self.low_power {
            self.event(tick, "EVA_LowPower", Bus::Eva, 60);
        }
        self.low_power = low_power;
        let no_money = session.sim.credits() == 0 && !session.sim.production().is_empty();
        if no_money && !self.no_money {
            self.event(tick, "EVA_InsufficientFunds", Bus::Eva, 60);
        }
        self.no_money = no_money;
        if session.outcome != self.outcome {
            let event = match session.outcome {
                Outcome::Won => "EVA_YouAreVictorious",
                Outcome::Lost => "EVA_YouHaveLost",
                Outcome::Ongoing => "",
            };
            self.event(tick, event, Bus::Eva, 60);
            self.outcome = session.outcome;
        }
        self.entities
            .retain(|id, _| session.sim.target_pos(*id).is_some());
        if let Output::Capture(recorder) = &mut self.output {
            if let Err(e) = recorder.tick() {
                self.error.get_or_insert(e.to_string());
            }
        }
    }

    pub fn set_muted(&self, muted: bool) {
        if let Output::Live(output) = &self.output {
            output.set_muted(muted);
        }
    }

    pub fn toggle_mute(&self) {
        if let Output::Live(output) = &self.output {
            println!(
                "OpenYR: audio {} ({} output frames, {} with signal)",
                if output.toggle_mute() { "muted" } else { "on" },
                output.frames_played(),
                output.signal_frames()
            );
        }
    }

    pub fn frames_played(&self) -> u64 {
        match &self.output {
            Output::Live(output) => output.frames_played(),
            Output::Capture(_) => 0,
        }
    }

    pub fn finish(self) -> Result<(), String> {
        if let Some(error) = self.error {
            return Err(error);
        }
        if let Output::Capture(recorder) = self.output {
            recorder.finish().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

struct Recorder {
    mixer: Mixer,
    wav: std::io::BufWriter<std::fs::File>,
    log: std::io::BufWriter<std::fs::File>,
    bytes: u32,
}

impl Recorder {
    fn new(dir: &Path) -> std::io::Result<Self> {
        let mut wav = std::io::BufWriter::new(std::fs::File::create(dir.join("audio.wav"))?);
        wav.write_all(&yr_formats::audio::encode_wav(&yr_formats::audio::Pcm {
            sample_rate: CAPTURE_RATE,
            channels: 2,
            samples: vec![],
        }))?;
        let mut log = std::io::BufWriter::new(std::fs::File::create(dir.join("audio-cues.tsv"))?);
        writeln!(log, "tick\tbus\tevent\tsample")?;
        Ok(Self {
            mixer: Mixer::new(CAPTURE_RATE),
            wav,
            log,
            bytes: 0,
        })
    }

    fn cue(&mut self, tick: u64, cue: Cue) -> std::io::Result<()> {
        writeln!(
            self.log,
            "{tick}\t{:?}\t{}\t{}",
            cue.playback.bus, cue.event, cue.sample
        )?;
        self.mixer.play(cue.playback);
        Ok(())
    }

    fn tick(&mut self) -> std::io::Result<()> {
        let count = CAPTURE_RATE / crate::session::TICKS_PER_SEC as u32;
        self.bytes = self
            .bytes
            .checked_add(count * 4)
            .filter(|n| *n <= u32::MAX - 36)
            .ok_or_else(|| std::io::Error::other("audio proof exceeds RIFF size limit"))?;
        for _ in 0..count {
            for sample in self.mixer.next_frame() {
                self.wav
                    .write_all(&((sample * 32767.0) as i16).to_le_bytes())?;
            }
        }
        Ok(())
    }

    fn finish(mut self) -> std::io::Result<()> {
        self.wav.seek(SeekFrom::Start(4))?;
        self.wav.write_all(&(self.bytes + 36).to_le_bytes())?;
        self.wav.seek(SeekFrom::Start(40))?;
        self.wav.write_all(&self.bytes.to_le_bytes())?;
        self.wav.flush()?;
        self.log.flush()
    }
}
