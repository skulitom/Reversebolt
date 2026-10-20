//! Synthetic benchmark worlds. Ground truth stays here, outside discovery.rs.
use reversebolt_core::discovery::Kind;
use serde::{Deserialize, Serialize};

pub const ARENA_BYTES: usize = 256 * 1024;
pub const FRAMES: usize = 20;

#[derive(Clone)]
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn bounded(&mut self, count: usize) -> usize {
        self.next() as usize % count
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Scenario {
    Plain,
    FloatNormalized,
    ScaledInteger,
    Inverted,
    DoubleOffset,
    NoisyObservation,
    Nonlinear,
    RotatingXor,
    Relocation,
    Constant,
    Unaligned,
    Coupled,
}

#[derive(Clone)]
pub struct World {
    pub bytes: Vec<u8>,
    scenario: Scenario,
    rng: Rng,
    offsets: [usize; 2],
    copies: [[usize; 2]; 2],
    distractors: Vec<usize>,
    stale: [usize; 2],
    kind: Kind,
    scale: f64,
    bias: f64,
    key: u32,
    step: usize,
}

impl World {
    pub fn new(seed: u64, scenario: Scenario) -> Self {
        let mut rng = Rng::new(seed);
        let mut bytes = vec![0_u8; ARENA_BYTES];
        for chunk in bytes.as_chunks_mut::<8>().0 {
            chunk.copy_from_slice(&rng.next().to_le_bytes());
        }
        // Unique random 16-byte slots. Discovery never receives this list.
        let mut used = std::collections::HashSet::new();
        let mut slot = || loop {
            let offset = rng.bounded(ARENA_BYTES / 16) * 16;
            if used.insert(offset) {
                break offset;
            }
        };
        let mut offsets = [slot(), slot()];
        let copies = [[slot(), slot()], [slot(), slot()]];
        let stale = [slot(), slot()];
        let distractors = (0..512).map(|_| slot()).collect();
        if matches!(scenario, Scenario::Unaligned) {
            offsets[0] += 1;
            offsets[1] += 3;
        }
        let kind = match scenario {
            Scenario::FloatNormalized => Kind::F32,
            Scenario::DoubleOffset => Kind::F64,
            _ => Kind::I32,
        };
        let scale = match scenario {
            Scenario::ScaledInteger => (rng.bounded(19) + 2) as f64,
            Scenario::Inverted => -((rng.bounded(9) + 2) as f64),
            Scenario::FloatNormalized => 1.0 / (100 + rng.bounded(100)) as f64,
            Scenario::DoubleOffset => 1.3 + rng.bounded(20) as f64 / 10.0,
            _ => 1.0,
        };
        let bias = match scenario {
            Scenario::ScaledInteger | Scenario::Inverted | Scenario::DoubleOffset => {
                1000.0 + rng.bounded(10000) as f64
            }
            _ => 0.0,
        };
        let mut world = Self {
            bytes,
            scenario,
            rng,
            offsets,
            copies,
            distractors,
            stale,
            kind,
            scale,
            bias,
            key: 0x379ac145,
            step: 0,
        };
        world.store(0, 83.0);
        world.store(1, 27.0);
        world.refresh([83.0, 27.0]);
        world
    }

    fn write_number(&mut self, offset: usize, kind: Kind, value: f64) {
        if let Some(bytes) = kind.encode(value) {
            self.bytes[offset..offset + bytes.len()].copy_from_slice(&bytes);
        }
    }

    fn store(&mut self, channel: usize, value: f64) {
        let encoded = match self.scenario {
            Scenario::Nonlinear => value * value,
            Scenario::RotatingXor => {
                let bytes = ((value as u32) ^ self.key).to_le_bytes();
                let offset = self.offsets[channel];
                self.bytes[offset..offset + 4].copy_from_slice(&bytes);
                return;
            }
            _ => value * self.scale + self.bias,
        };
        self.write_number(self.offsets[channel], self.kind, encoded);
    }

    fn load(&self, channel: usize) -> f64 {
        let raw = &self.bytes[self.offsets[channel]..];
        if matches!(self.scenario, Scenario::RotatingXor) {
            return (u32::from_le_bytes(raw[..4].try_into().expect("fixture word")) ^ self.key)
                as f64;
        }
        let value = self.kind.read(raw).unwrap_or(f64::NAN);
        match self.scenario {
            Scenario::Nonlinear => value.max(0.0).sqrt(),
            _ => (value - self.bias) / self.scale,
        }
    }

    fn refresh(&mut self, values: [f64; 2]) {
        for (channel, value) in values.iter().enumerate() {
            // Exact display copy and transformed telemetry copy; both non-authoritative.
            self.write_number(self.copies[channel][0], Kind::I32, value.round());
            self.write_number(self.copies[channel][1], Kind::F32, value * 2.5 + 43.0);
        }
    }

    pub fn observe(&self) -> [f64; 2] {
        [self.load(0), self.load(1)]
    }

    /// An idle simulation tick refreshes derived HUD/telemetry copies.
    pub fn idle(&mut self) -> [f64; 2] {
        let mut values = self.observe();
        if matches!(self.scenario, Scenario::Coupled) {
            values[1] = values[0] * 0.5;
            self.store(1, values[1]);
        }
        self.refresh(values);
        values
    }

    pub fn advance(&mut self) -> [Option<f64>; 2] {
        self.step += 1;
        let previous = self.observe();
        for (channel, value) in previous.iter().enumerate() {
            self.write_number(self.stale[channel], Kind::I32, value.round());
        }
        let mut values = [
            30.0 + self.rng.bounded(65) as f64,
            5.0 + self.rng.bounded(29) as f64,
        ];
        if matches!(self.scenario, Scenario::Constant) {
            values = [83.0, 27.0];
        }
        if matches!(self.scenario, Scenario::Coupled) {
            values[1] = values[0] * 0.5;
        }
        if matches!(self.scenario, Scenario::RotatingXor) {
            self.key = self.rng.next() as u32;
        }
        if matches!(self.scenario, Scenario::Relocation) && self.step == FRAMES / 2 + 1 {
            // New independent arena slots; old values remain stale.
            self.offsets.swap(0, 1);
        }
        for (channel, value) in values.iter().enumerate() {
            self.store(channel, *value);
        }
        for position in 0..self.distractors.len() {
            let offset = self.distractors[position];
            let random_value = self.rng.bounded(1000) as f64;
            self.write_number(
                offset,
                if position % 2 == 0 {
                    Kind::I32
                } else {
                    Kind::F32
                },
                random_value,
            );
        }
        self.refresh(values);
        let mut visible = values.map(Some);
        if matches!(self.scenario, Scenario::NoisyObservation) {
            if self.step == 4 || self.step == 14 {
                visible[0] = Some(values[0] + 31.0);
            }
            if self.step == 6 || self.step == 16 {
                visible[1] = None;
            }
        }
        visible
    }
}
