// Comforts' sleep rules as a pure, testable library: the time-of-day window a
// sleeping bag may be used in, the wake time after a night (or day) sleep, the
// phantom-rest and bag-break rules, and the config file. This file is both the
// standalone Cargo library and the injected `crate::game::comforts` module.
//
// Ported from Comforts 9.0.5+1.21.1 (LGPL-3.0-or-later, local-only until the
// maintainer approves publishing). Behaviour extracted from the mod's public
// source (`ComfortsEvents.checkTime`/`getWakeTime`/`onSleep`, `ComfortsConfig`,
// `SleepingBagBlock.canRest`), reimplemented in Rust.

/// The mod id its config file and block ids use.
pub const MOD_ID: &str = "comforts";

/// The default server config, as `config/comforts-server.toml`: upstream's
/// own defaults and key spelling (`ComfortsConfig.java:68-152`).
pub const DEFAULT_CONFIG: &str = "\
# Comforts — server rules. Disabled features are upstream's own defaults.
# The time of day a sleeping bag can be used: NONE, DAY, NIGHT or DAY_OR_NIGHT.
sleepingBagUse = \"NIGHT\"
# What percentage of players must sleep to skip the day. -1 uses the
# playersSleepingPercentage game rule.
daySleepingPercentage = -1
# Ticks added to (or removed from) the time players wake at after sleeping
# through the night.
dayWakeTimeOffset = 0
# Ticks added to (or removed from) the time players wake at after sleeping
# through the day.
nightWakeTimeOffset = 0
# Refuse another sleep for a while after one.
restrictSleeping = false
# A sleep in a sleeping bag resets the time since rest (phantoms).
sleepingBagsStopPhantoms = true
# Percent chance a sleeping bag breaks when its sleeper wakes.
sleepingBagBreakChance = 0
";

/// Which hours a block may be slept in (`ComfortsConfig.ComfortsTimeUse`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeUse {
    /// Never.
    None,
    /// Inside the day span only.
    Day,
    /// Vanilla's own rule (night or a thunderstorm).
    Night,
    /// The day span, and vanilla's rule at night.
    DayOrNight,
}

impl TimeUse {
    /// The value upstream's file spells, case-insensitively.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().trim_matches('"').to_ascii_uppercase().as_str() {
            "NONE" => Some(Self::None),
            "DAY" => Some(Self::Day),
            "NIGHT" => Some(Self::Night),
            "DAY_OR_NIGHT" => Some(Self::DayOrNight),
            _ => None,
        }
    }

    /// The value [`format_config`] writes.
    #[cfg(test)]
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Day => "DAY",
            Self::Night => "NIGHT",
            Self::DayOrNight => "DAY_OR_NIGHT",
        }
    }
}

/// What the port's sleep check says about a time of day
/// (`ComfortsEvents.Result`): grant the sleep, refuse it, or let vanilla's
/// own checks decide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SleepCheck {
    /// Let the player sleep even where vanilla would refuse.
    Allow,
    /// Leave vanilla's answer alone.
    Default,
    /// Refuse with the "not possible now" message.
    Deny,
}

/// The port's config, with upstream's defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub sleeping_bag_use: TimeUse,
    pub day_wake_time_offset: i64,
    pub night_wake_time_offset: i64,
    pub restrict_sleeping: bool,
    pub sleeping_bags_stop_phantoms: bool,
    /// Percent, 0 to 100.
    pub sleeping_bag_break_chance: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sleeping_bag_use: TimeUse::Night,
            day_wake_time_offset: 0,
            night_wake_time_offset: 0,
            restrict_sleeping: false,
            sleeping_bags_stop_phantoms: true,
            sleeping_bag_break_chance: 0.0,
        }
    }
}

/// The day span upstream measures before its offsets: `(100, 11900)`.
const DAY_START: i64 = 100;
const DAY_END: i64 = 11_900;

/// Upstream's clamp on both wake offsets (`defineInRange(…, -2000, 2000)`).
const WAKE_OFFSET_LIMIT: i64 = 2_000;

fn clamp_i64(value: i64, low: i64, high: i64) -> i64 {
    value.clamp(low, high)
}

/// One `key = value` line's value, with a trailing comment and quotes removed.
fn value_of(line: &str) -> Option<(&str, &str)> {
    let (key, rest) = line.split_once('=')?;
    let mut value = rest.trim();
    if let Some(at) = value.find('#') {
        value = value[..at].trim();
    }
    Some((key.trim(), value.trim_matches('"')))
}

/// The port's config file. Unknown keys and values that cannot be read keep
/// the default; numbers are clamped to upstream's ranges, so a hand-edited
/// file can never make every sleep fail.
#[must_use]
pub fn parse_config(text: &str) -> Config {
    let mut config = Config::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = value_of(line) else {
            continue;
        };
        match key {
            "sleepingBagUse" | "sleeping_bag_use" => {
                if let Some(use_) = TimeUse::parse(value) {
                    config.sleeping_bag_use = use_;
                }
            }
            // Accepted for upstream parity. The day-sleep slice is deferred
            // and the pin has no sleep-count override, so nothing reads it.
            "daySleepingPercentage" | "day_sleeping_percentage" => {}
            "dayWakeTimeOffset" | "day_wake_time_offset" => {
                if let Ok(n) = value.parse::<i64>() {
                    config.day_wake_time_offset =
                        clamp_i64(n, -WAKE_OFFSET_LIMIT, WAKE_OFFSET_LIMIT);
                }
            }
            "nightWakeTimeOffset" | "night_wake_time_offset" => {
                if let Ok(n) = value.parse::<i64>() {
                    config.night_wake_time_offset =
                        clamp_i64(n, -WAKE_OFFSET_LIMIT, WAKE_OFFSET_LIMIT);
                }
            }
            "restrictSleeping" | "restrict_sleeping" => {
                if let Ok(b) = value.parse::<bool>() {
                    config.restrict_sleeping = b;
                }
            }
            "sleepingBagsStopPhantoms" | "sleeping_bags_stop_phantoms" => {
                if let Ok(b) = value.parse::<bool>() {
                    config.sleeping_bags_stop_phantoms = b;
                }
            }
            "sleepingBagBreakChance" | "sleeping_bag_break_chance" | "break_chance" => {
                if let Ok(n) = value.parse::<f64>()
                    && n.is_finite()
                {
                    config.sleeping_bag_break_chance = n.clamp(0.0, 100.0);
                }
            }
            _ => {}
        }
    }
    config
}

/// The config file in upstream's key spelling, parsed back by
/// [`parse_config`].
#[cfg(test)]
fn format_config(config: &Config) -> String {
    format!(
        "\
# Comforts — server rules. Disabled features are upstream's own defaults.
# The time of day a sleeping bag can be used: NONE, DAY, NIGHT or DAY_OR_NIGHT.
sleepingBagUse = \"{}\"
# What percentage of players must sleep to skip the day. -1 uses the
# playersSleepingPercentage game rule.
daySleepingPercentage = -1
# Ticks added to (or removed from) the time players wake at after sleeping
# through the night.
dayWakeTimeOffset = {}
# Ticks added to (or removed from) the time players wake at after sleeping
# through the day.
nightWakeTimeOffset = {}
# Refuse another sleep for a while after one.
restrictSleeping = {}
# A sleep in a sleeping bag resets the time since rest (phantoms).
sleepingBagsStopPhantoms = {}
# Percent chance a sleeping bag breaks when its sleeper wakes.
sleepingBagBreakChance = {}
",
        config.sleeping_bag_use.as_str(),
        config.day_wake_time_offset,
        config.night_wake_time_offset,
        config.restrict_sleeping,
        config.sleeping_bags_stop_phantoms,
        config.sleeping_bag_break_chance,
    )
}

/// The day span `(start, end)`, both exclusive, after upstream's two offsets
/// (`ComfortsEvents.checkTime`): the start never drops below one tick and the
/// span always keeps one.
#[must_use]
pub fn day_span(config: &Config) -> (i64, i64) {
    let start = (DAY_START + config.day_wake_time_offset).max(1);
    let end = (DAY_END + config.night_wake_time_offset).max(start + 1);
    (start, end)
}

/// Whether the port grants, refuses or defers a sleep at `time_of_day`
/// (`0` to `23999`).
#[must_use]
pub fn time_check(config: &Config, time_of_day: i64) -> SleepCheck {
    let (start, end) = day_span(config);
    let in_day = time_of_day > start && time_of_day < end;
    match config.sleeping_bag_use {
        TimeUse::None => SleepCheck::Deny,
        TimeUse::Night => SleepCheck::Default,
        TimeUse::Day if in_day => SleepCheck::Allow,
        TimeUse::Day => SleepCheck::Deny,
        TimeUse::DayOrNight if in_day => SleepCheck::Allow,
        TimeUse::DayOrNight => SleepCheck::Default,
    }
}

/// `Level.isDay`: the Overworld clock before `13_000` of its day.
#[must_use]
pub fn is_day(day_time: i64) -> bool {
    day_time.rem_euclid(24_000) < 13_000
}

/// The time everyone wakes at (`ComfortsEvents.getWakeTime`): a day sleeper
/// wakes at the coming night (`11_999`) plus the night offset, everyone else
/// at vanilla's morning plus the day offset. Never before `current`.
#[must_use]
pub fn wake_time(
    config: &Config,
    day_sleeping: bool,
    day: bool,
    current: i64,
    vanilla_target: i64,
) -> i64 {
    if day_sleeping && day {
        let next_day = current + 24_000;
        let dusk = (next_day - next_day.rem_euclid(24_000)) - 12_001;
        (dusk + config.night_wake_time_offset).max(current)
    } else {
        (vanilla_target + config.day_wake_time_offset).max(current)
    }
}

/// Whether a sleeping bag breaks on this wake, from the configured percent
/// chance and a roll in `0..1` (`ComfortsEvents.onWakeUp`).
#[must_use]
pub fn break_roll(chance_percent: f64, roll: f64) -> bool {
    chance_percent / 100.0 > roll
}

/// Whether `block` is one of this port's sleeping beds. The pin recognises a
/// bed by an id ending in `_bed` (and so does the port's content), so the
/// family is the mod's namespace plus that suffix.
#[must_use]
pub fn is_sleeping_bed(block: &str) -> bool {
    block.starts_with("comforts:") && block.ends_with("_bed")
}

/// `restrictSleeping`: a player whose tired time has not passed is refused
/// (`ComfortsEvents.onSleep`).
#[must_use]
pub fn may_sleep(restrict: bool, tired_until: i64, day_time: i64) -> bool {
    !restrict || tired_until <= day_time
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every default is upstream's (`ComfortsConfig.java:68-152`).
    #[test]
    fn the_defaults_are_upstreams() {
        let config = Config::default();
        assert_eq!(config.sleeping_bag_use, TimeUse::Night);
        assert_eq!(config.day_wake_time_offset, 0);
        assert_eq!(config.night_wake_time_offset, 0);
        assert!(!config.restrict_sleeping);
        assert!(config.sleeping_bags_stop_phantoms);
        assert_eq!(config.sleeping_bag_break_chance, 0.0);
        assert_eq!(parse_config(DEFAULT_CONFIG), Config::default());
    }

    /// Upstream's file spells its keys in camelCase.
    #[test]
    fn parsing_reads_upstream_camel_case_keys() {
        let text = "\
sleepingBagUse = \"DAY\"
daySleepingPercentage = 50
dayWakeTimeOffset = -100
nightWakeTimeOffset = 200
restrictSleeping = true
sleepingBagsStopPhantoms = false
sleepingBagBreakChance = 25
";
        let config = parse_config(text);
        assert_eq!(config.sleeping_bag_use, TimeUse::Day);
        assert_eq!(config.day_wake_time_offset, -100);
        assert_eq!(config.night_wake_time_offset, 200);
        assert!(config.restrict_sleeping);
        assert!(!config.sleeping_bags_stop_phantoms);
        assert_eq!(config.sleeping_bag_break_chance, 25.0);
    }

    /// snake_case is accepted too, so a hand-written file works.
    #[test]
    fn parsing_also_reads_snake_case_keys() {
        let text = "\
sleeping_bag_use = \"DAY_OR_NIGHT\"
break_chance = 10
";
        let config = parse_config(text);
        assert_eq!(config.sleeping_bag_use, TimeUse::DayOrNight);
        assert_eq!(config.sleeping_bag_break_chance, 10.0);
    }

    /// A broken or out-of-range file falls back to the defaults and known
    /// ranges; it never makes the game refuse everything.
    #[test]
    fn parsing_ignores_unknown_keys_and_clamps_values() {
        let text = "\
unknownKey = 3
sleepingBagUse = \"SOMETIMES\"
daySleepingPercentage = 500
dayWakeTimeOffset = -9000
nightWakeTimeOffset = 9000
sleepingBagBreakChance = 250
";
        let config = parse_config(text);
        assert_eq!(config.sleeping_bag_use, TimeUse::Night);
        assert_eq!(config.day_wake_time_offset, -2000);
        assert_eq!(config.night_wake_time_offset, 2000);
        assert_eq!(config.sleeping_bag_break_chance, 100.0);
    }

    /// The written default file is what the parser reads back.
    #[test]
    fn the_written_config_round_trips() {
        assert_eq!(format_config(&Config::default()), DEFAULT_CONFIG);
        let config = Config {
            sleeping_bag_use: TimeUse::Day,
            sleeping_bag_break_chance: 12.5,
            day_wake_time_offset: -40,
            restrict_sleeping: true,
            ..Config::default()
        };
        assert_eq!(parse_config(&format_config(&config)), config);
    }

    /// A sleeping bag defaults to NIGHT: vanilla's own rule decides, so the
    /// hook never refuses a night sleep and never grants a day one.
    #[test]
    fn the_night_use_defers_to_vanilla_at_night_and_day() {
        let config = Config::default();
        assert_eq!(time_check(&config, 18_000), SleepCheck::Default);
        assert_eq!(time_check(&config, 6_000), SleepCheck::Default);
    }

    /// A DAY block (a hammock) may sleep inside the day span and is refused
    /// outside it, which is the opposite of a bed.
    #[test]
    fn the_day_use_allows_the_day_span_and_refuses_the_night() {
        let config = Config {
            sleeping_bag_use: TimeUse::Day,
            ..Config::default()
        };
        assert_eq!(time_check(&config, 6_000), SleepCheck::Allow);
        assert_eq!(time_check(&config, 18_000), SleepCheck::Deny);
        // The span is exclusive at both ends (`ComfortsEvents.checkTime`).
        assert_eq!(time_check(&config, 100), SleepCheck::Deny);
        assert_eq!(time_check(&config, 11_900), SleepCheck::Deny);
        assert_eq!(time_check(&config, 101), SleepCheck::Allow);
    }

    /// `NONE` refuses every hour, as upstream's `ComfortsTimeUse.NONE`.
    #[test]
    fn the_none_use_never_sleeps() {
        let config = Config {
            sleeping_bag_use: TimeUse::None,
            ..Config::default()
        };
        assert_eq!(time_check(&config, 6_000), SleepCheck::Deny);
        assert_eq!(time_check(&config, 18_000), SleepCheck::Deny);
    }

    /// `DAY_OR_NIGHT` grants the day span and leaves the night to vanilla.
    #[test]
    fn the_day_or_night_use_grants_the_day_and_defers_at_night() {
        let config = Config {
            sleeping_bag_use: TimeUse::DayOrNight,
            ..Config::default()
        };
        assert_eq!(time_check(&config, 6_000), SleepCheck::Allow);
        assert_eq!(time_check(&config, 18_000), SleepCheck::Default);
    }

    /// The two offsets move the day span's ends (upstream clamps the start
    /// above zero and keeps the span at least one tick).
    #[test]
    fn the_day_span_follows_the_offsets() {
        assert_eq!(day_span(&Config::default()), (100, 11_900));
        let moved = Config {
            day_wake_time_offset: -500,
            night_wake_time_offset: 300,
            ..Config::default()
        };
        assert_eq!(day_span(&moved), (1, 12_200));
        let collapsed = Config {
            day_wake_time_offset: -2_000,
            night_wake_time_offset: -2_000,
            ..Config::default()
        };
        assert_eq!(day_span(&collapsed), (1, 9_900));
    }

    /// `getWakeTime`: a night sleep wakes at the morning plus the day offset;
    /// a day sleep at the coming night plus the night offset; never before now.
    #[test]
    fn the_wake_time_uses_the_matching_offset() {
        // A night sleeper at 18 000: vanilla's next morning is 24 000.
        assert_eq!(
            wake_time(&Config::default(), false, false, 18_000, 24_000),
            24_000
        );
        let later = Config {
            day_wake_time_offset: 500,
            ..Config::default()
        };
        assert_eq!(wake_time(&later, false, false, 18_000, 24_000), 24_500);
        let earlier = Config {
            day_wake_time_offset: -24_000,
            ..Config::default()
        };
        assert_eq!(
            wake_time(&earlier, false, false, 18_000, 24_000),
            18_000,
            "never before the current time"
        );
        // A day sleeper at 6 000 wakes at 11 999 (the coming night).
        let dusk = Config {
            night_wake_time_offset: -1,
            ..Config::default()
        };
        assert_eq!(wake_time(&dusk, true, true, 6_000, 24_000), 11_998);
        let later_dusk = Config {
            night_wake_time_offset: 200,
            ..Config::default()
        };
        assert_eq!(wake_time(&later_dusk, true, true, 6_000, 24_000), 12_199);
        // A day sleeper whose day has already ended uses the night rule.
        assert_eq!(wake_time(&later_dusk, true, false, 18_000, 24_000), 24_000);
    }

    /// A sleeping bag breaks after a long sleep with the configured chance.
    #[test]
    fn the_break_roll_uses_the_percentage() {
        assert!(!break_roll(0.0, 0.0));
        assert!(break_roll(100.0, 0.999));
        assert!(break_roll(50.0, 0.49));
        assert!(!break_roll(50.0, 0.5));
    }

    /// Only the port's own bag blocks are heard; a vanilla bed never is.
    #[test]
    fn only_comforts_beds_are_recognised() {
        assert!(is_sleeping_bed("comforts:sleeping_bed"));
        assert!(!is_sleeping_bed("minecraft:white_bed"));
        assert!(!is_sleeping_bed("comforts:sleeping_bag_white"));
        assert!(!is_sleeping_bed("comforts:hammock"));
    }

    /// `restrictSleeping`: a player who just slept is told to wait.
    #[test]
    fn the_restriction_refuses_until_the_tired_time() {
        assert!(may_sleep(false, 30_000, 20_000));
        assert!(!may_sleep(true, 30_000, 20_000));
        assert!(may_sleep(true, 20_000, 20_000));
        assert!(may_sleep(true, 0, 20_000));
    }

    /// `Level.isDay` splits the Overworld clock at 13 000.
    #[test]
    fn the_day_splits_at_thirteen_thousand() {
        assert!(is_day(6_000));
        assert!(is_day(12_999));
        assert!(!is_day(13_000));
        assert!(!is_day(18_000));
        assert!(!is_day(23_999));
        assert!(is_day(24_000));
    }
}
