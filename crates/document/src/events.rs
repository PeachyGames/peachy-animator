//! Event definitions and timed event keys. Events carry data for the game and
//! do not change the pose. Keys store only the values they override, exactly as
//! written, because consumers resolve omitted values (docs/slices/events.md).
use crate::Document;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventDef {
    pub name: String,
    pub int: i32,
    pub float: f32,
    pub string: String,
    pub audio: Option<String>,
    pub volume: f32,
    pub balance: f32,
}

impl EventDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            int: 0,
            float: 0.0,
            string: String::new(),
            audio: None,
            volume: 1.0,
            balance: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventKey {
    pub time: f32,
    pub event: usize,
    pub int: Option<i32>,
    pub float: Option<f32>,
    pub string: Option<String>,
    pub volume: Option<f32>,
    pub balance: Option<f32>,
}

impl Document {
    pub(crate) fn validate_events(&self) -> Result<(), String> {
        let mut names = std::collections::BTreeSet::new();
        for event in &self.events {
            if event.name.trim().is_empty() || !names.insert(&event.name) {
                return Err("Event names must be nonempty and unique".into());
            }
            if ![event.float, event.volume, event.balance]
                .iter()
                .all(|v| v.is_finite())
            {
                return Err(format!("Event {}: values must be finite", event.name));
            }
        }
        for animation in &self.animations {
            let mut previous = 0.0;
            for key in &animation.events {
                if key.event >= self.events.len()
                    || !key.time.is_finite()
                    || key.time < 0.0
                    || key.time < previous
                    || [key.float, key.volume, key.balance]
                        .iter()
                        .flatten()
                        .any(|v| !v.is_finite())
                {
                    return Err(format!(
                        "Animation {}: event keys need known events, finite values and nondecreasing times",
                        animation.name
                    ));
                }
                previous = key.time;
            }
        }
        Ok(())
    }
}
