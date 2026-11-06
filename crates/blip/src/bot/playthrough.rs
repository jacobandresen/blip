//! Playthroughs: named bot runs, each showing one theme of a game, with the
//! stats that prove it. A game lists them in its `bot.rs`:
//!
//! ```ignore
//! pub const PLAYTHROUGHS: &[Playthrough] = &[
//!     Playthrough {
//!         name: "the snake eats and grows into level 2",
//!         goals: &[Goal::AtLeast("level", 2.0)],
//!         ..Playthrough::DEFAULT
//!     },
//! ];
//! ```
//!
//! and hands them over at startup with `blip::bot::playthroughs(bot::PLAYTHROUGHS)`.
//! `BLIP_BOT_LIST=1` prints their names; `BLIP_BOT_PLAYTHROUGH=<name>` runs one,
//! ending as soon as its goals are met, and its RESULT line says
//! `playthrough=pass` or `playthrough=fail missing=<goal>`.

use std::collections::BTreeMap;

/// One named run of the game's bot.
pub struct Playthrough {
    /// What the run shows, as a player would say it.
    pub name: &'static str,
    /// The level it starts on, from 1.
    pub level: u32,
    /// Game seconds it may take.
    pub seconds: f32,
    /// The game's own bot switches for this run: `("PLAYERS", "2")` is `BLIP_BOT_PLAYERS=2`.
    pub settings: &'static [(&'static str, &'static str)],
    /// What must be true by the end.
    pub goals: &'static [Goal],
    /// The random seed: the same seed plays the same game every time.
    pub seed: u64,
}

/// Something a playthrough must show, read from the bot's stats.
#[derive(Debug)]
pub enum Goal {
    /// The stat gets at least this high.
    AtLeast(&'static str, f64),
    /// The stat stays at most this (zero deaths, say).
    AtMost(&'static str, f64),
    /// The game ends this way: `won` or `over`.
    Ends(&'static str),
}

impl Playthrough {
    pub const DEFAULT: Self = Self { name: "", level: 1, seconds: 120.0, settings: &[], goals: &[], seed: 1 };

    /// The value of one of this run's settings, the start level included.
    pub(super) fn setting(&self, name: &str) -> Option<String> {
        if name == "LEVEL" && self.level > 1 { return Some(self.level.to_string()); }
        self.settings.iter().find(|(k, _)| *k == name).map(|(_, v)| v.to_string())
    }

    /// The goals not met, given how the game ended and its stats.
    fn missing(&self, outcome: &str, stats: &BTreeMap<String, f64>) -> Vec<String> {
        let stat = |k: &str| stats.get(k).copied().unwrap_or(0.0);
        self.goals.iter().filter_map(|goal| match *goal {
            Goal::AtLeast(k, v) if stat(k) < v => Some(format!("{k}>={v}")),
            Goal::AtMost(k, v) if stat(k) > v => Some(format!("{k}<={v}")),
            Goal::Ends(o) if outcome != o => Some(format!("ends={o}")),
            _ => None,
        }).collect()
    }

    /// Has the run shown what it set out to, so it can stop now? Only runs
    /// with something to reach stop early; ones that must survive play on.
    pub(super) fn met(&self, stats: &BTreeMap<String, f64>) -> bool {
        let reaches = self.goals.iter().any(|g| matches!(g, Goal::AtLeast(..)));
        let waits = self.goals.iter().any(|g| matches!(g, Goal::Ends(_)));
        reaches && !waits && self.missing("", stats).is_empty()
    }

    /// The end of the RESULT line: ` playthrough=pass`, or what was missing.
    pub(super) fn verdict(&self, outcome: &str, stats: &BTreeMap<String, f64>) -> String {
        let missing = self.missing(outcome, stats);
        if missing.is_empty() { " playthrough=pass".into() } else { format!(" playthrough=fail missing={}", missing.join(",")) }
    }
}

/// Hand the game's playthroughs to the bot, first thing in `main`.
/// `BLIP_BOT_LIST=1` prints them and ends; `BLIP_BOT_PLAYTHROUGH=<name>` runs one.
pub fn playthroughs(list: &'static [Playthrough]) {
    if std::env::var_os("BLIP_BOT_LIST").is_some() {
        for p in list { println!("PLAYTHROUGH {}", p.name); }
        std::process::exit(0);
    }
    let Ok(name) = std::env::var("BLIP_BOT_PLAYTHROUGH") else { return };
    let Some(p) = list.iter().find(|p| p.name == name) else {
        let names: Vec<_> = list.iter().map(|p| p.name).collect();
        panic!("no playthrough called {name:?}; there are {names:?}");
    };
    super::with(|r| {
        r.on = true;
        r.maxt = p.seconds;
        r.playthrough = Some(p);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN: Playthrough = Playthrough {
        name: "climbs without dying",
        goals: &[Goal::AtLeast("level", 3.0), Goal::AtMost("deaths", 0.0)],
        ..Playthrough::DEFAULT
    };

    #[test]
    fn a_playthrough_passes_once_every_goal_is_met() {
        let mut stats = BTreeMap::from([("level".to_string(), 2.0)]);
        assert!(!RUN.met(&stats));
        assert_eq!(RUN.verdict("timeout", &stats), " playthrough=fail missing=level>=3");
        stats.insert("level".into(), 3.0);
        assert!(RUN.met(&stats));
        stats.insert("deaths".into(), 1.0);
        assert_eq!(RUN.verdict("goals", &stats), " playthrough=fail missing=deaths<=0");
    }
}
