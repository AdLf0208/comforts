//! Comforts — the one `impl Hooks` at the crate root: a sleeping bag that sleeps
//! through the night and never becomes the player's respawn point.
//!
//! The retired `.hnmod` named ten stamped hooks and one `content_root` target. In the SDK
//! package each is one method here, and the content half is simply `assets/` + `data/`:
//!
//! | Retired injection (pin `1d63e0a5`) | SDK 0.2 |
//! |---|---|
//! | `embedded-content` (`content_root` HEAD, the AE2 write-out) | **gone**: one flat `.hmod` carries the content |
//! | `register-configs` / `config-loading` / `config-reloading` / `config-unloading` | `Hooks::register_configs` / `config_loading` / `config_reloading` / `config_unloading` |
//! | `can-player-sleep` | `Hooks::can_player_sleep` |
//! | `bed-enter` | `Hooks::player_bed_enter` |
//! | `bed-leave` | `Hooks::player_bed_leave` |
//! | `deep-sleep` | `Hooks::player_deep_sleep` |
//! | `spawn-point` (`player_spawn_point_set`, with a `cause`) | `Hooks::player_set_spawn` (no cause; the block at `pos` answers it) |
//! | `sleep-finished` (`sleep_finished_time` v1, wrote `new_time`) | `Hooks::sleep_finished_time` v2 (`ClockAdjustment::Absolute`) |
//!
//! The rules and every number live in [`runtime`], byte-identical to the retired bundle's
//! `runtime.rs` and tested by `cargo test` without the game; this file only reads the game,
//! calls `runtime`, and writes the result back.
use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

use mod_api::{
    self, BlockUpdateFlags, CanPlayerSleepEvent, ClockAdjustment, Config as ModConfig, ConfigFile,
    ConfigKind, ConfigLoadingEvent, ConfigReloadingEvent, ConfigUnloadingEvent, Flow, Hooks,
    HostSide, PlayerBedEnterEvent, PlayerBedLeaveEvent, PlayerDeepSleepEvent, PlayerSetSpawnEvent,
    RegisterConfigsEvent, ServerHost, SleepFinishedTimeEvent, SoundSource, StatType, Text,
};
use runtime::{Config, TimeUse};

pub mod runtime;

/// Upstream's `restMultiplier` default (2.0): a sleeper is tired for half the time it
/// slept. Not configurable in this slice.
const REST_MULTIPLIER: f64 = 2.0;

/// Upstream breaks the bag only after this long asleep (`ComfortsEvents.onWakeUp`,
/// `timeSlept > 500`).
const BREAK_AFTER_TICKS: i64 = 500;

/// `player_deep_sleep` fires at this many ticks asleep; by then the game has counted
/// that many ticks since rest.
const DEEP_SLEEP_TICKS: i32 = 100;

/// The vanilla custom stat the phantom timer reads (`ServerStatsCounter`,
/// `minecraft:time_since_rest`).
const TIME_SINCE_REST: &str = "minecraft:time_since_rest";

/// The stats category upstream's `Stats.CUSTOM` uses.
const CUSTOM_STAT: StatType = StatType::Custom;

/// How the sleep hook answers vanilla's problem (`runtime::SleepCheck` plus
/// `restrictSleeping`), as the hook's `problem` field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Problem {
    /// Leave whatever vanilla (and earlier mods) decided.
    Keep,
    /// Clear vanilla's problem: the config lets this sleep happen anyway.
    Clear,
    /// Refuse with vanilla's "not possible now" message.
    Now,
    /// Refuse without vanilla's message, and tell the sleeper they are not sleepy.
    NotSleepy,
}

/// One player's sleep in progress.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Sleeper {
    /// The level the bag is in (`minecraft:overworld`).
    dimension: String,
    /// The clock time the player lay down at.
    started: i64,
    /// Whether this sleep skips the day (a hammock, or `DAY`/`DAY_OR_NIGHT`).
    day_sleeping: bool,
    /// The time since rest before the game resets it, when
    /// `sleepingBagsStopPhantoms` is off.
    phantom_before: Option<i32>,
}

/// One player's sleep state: the sleep in progress and the clock time another sleep is
/// refused until. Upstream keeps this in a player capability; the port keeps it for the
/// session, because the SDK's saved player data is not needed for a timer that resets on
/// restart (recorded in the notes).
#[derive(Default)]
struct PlayerSleep {
    sleeper: Option<Sleeper>,
    tired_until: i64,
}

#[derive(Default)]
struct Mod {
    config: Mutex<Option<Config>>,
    players: Mutex<BTreeMap<u128, PlayerSleep>>,
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// The server config `config/comforts-server.toml`, written from upstream's defaults
    /// when the file is missing (a world's `serverconfig/` copy wins).
    fn register_configs(&self, _host: HostSide, event: &mut RegisterConfigsEvent<'_>) {
        event.configs.push(ModConfig::new(
            runtime::MOD_ID,
            ConfigKind::Server,
            runtime::DEFAULT_CONFIG,
        ));
    }

    /// The file was read: keep the parsed rules.
    fn config_loading(&self, _host: HostSide, event: &mut ConfigLoadingEvent<'_>) {
        self.read_config(event.config);
    }

    /// The file changed on disk: apply it without a restart.
    fn config_reloading(&self, _host: HostSide, event: &mut ConfigReloadingEvent<'_>) {
        self.read_config(event.config);
    }

    /// The server stopped: drop the rules and the sleepers of that world.
    fn config_unloading(&self, _host: &mut dyn ServerHost, event: &mut ConfigUnloadingEvent<'_>) {
        self.drop_config(event.config);
    }

    /// The bag's time window (`ComfortsEvents.checkTime`) decides whether vanilla's
    /// refusal stands, and `restrictSleeping` may refuse a player who just slept.
    fn can_player_sleep(&self, host: &mut dyn ServerHost, event: &mut CanPlayerSleepEvent<'_>) {
        let Some(level) = host.level(host.dimension()) else {
            return;
        };
        let Some(block) = level.block(host, event.pos) else {
            return;
        };
        if !runtime::is_sleeping_bed(&block.id(host)) {
            return;
        }
        let Some(now) = level.day_time(host) else {
            return;
        };
        let tired_until = self.tired_until(event.player);
        match problem_after(&self.config(), event.problem.as_deref(), now, tired_until) {
            Problem::Keep => {}
            Problem::Clear => *event.problem = None,
            Problem::Now => *event.problem = Some("not_possible_now".to_owned()),
            Problem::NotSleepy => {
                *event.problem = Some("other_problem".to_owned());
                if let Some(player) = host.player(event.player) {
                    let _ = player.action_bar(
                        host,
                        Text::translate("capability.comforts.not_sleepy", Vec::new()),
                    );
                }
            }
        }
    }

    /// The checks passed and the player is entering the bag: remember the sleeper. Never
    /// cancels a sleep the config allowed.
    fn player_bed_enter(
        &self,
        host: &mut dyn ServerHost,
        event: &mut PlayerBedEnterEvent<'_>,
    ) -> Flow {
        let Some(level) = host.level(host.dimension()) else {
            return Flow::Continue;
        };
        let Some(block) = level.block(host, event.pos) else {
            return Flow::Continue;
        };
        if !runtime::is_sleeping_bed(&block.id(host)) {
            return Flow::Continue;
        }
        let Some(started) = level.day_time(host) else {
            return Flow::Continue;
        };
        let config = self.config();
        let phantom_before = if config.sleeping_bags_stop_phantoms {
            None
        } else {
            host.player(event.player)
                .and_then(|player| player.stat(host, CUSTOM_STAT, TIME_SINCE_REST))
        };
        self.begin_sleep(
            event.player,
            &config,
            host.dimension(),
            started,
            phantom_before,
        );
        Flow::Continue
    }

    /// 100 ticks asleep: the game has reset the time since rest, so put the pre-sleep
    /// value back when `sleepingBagsStopPhantoms` is off. Exact to a tick.
    fn player_deep_sleep(&self, host: &mut dyn ServerHost, event: &mut PlayerDeepSleepEvent<'_>) {
        let Some(before) = self.take_phantom_before(event.player) else {
            return;
        };
        let Some(player) = host.player(event.player) else {
            return;
        };
        let Some(current) = player.stat(host, CUSTOM_STAT, TIME_SINCE_REST) else {
            return;
        };
        let delta = deep_delta(before, current);
        if delta != 0 {
            let _ = player.award_stat(host, CUSTOM_STAT, TIME_SINCE_REST, delta);
        }
    }

    /// The sleeper woke: the bag may break (`sleepingBagBreakChance`), and a phantom-rest
    /// value that was not restored at deep sleep is restored now.
    fn player_bed_leave(&self, host: &mut dyn ServerHost, event: &mut PlayerBedLeaveEvent<'_>) {
        let Some(sleeper) = self.take_sleeper(event.player) else {
            return;
        };
        let Some(level) = host.level(&sleeper.dimension) else {
            return;
        };
        let Some(now) = level.day_time(host) else {
            return;
        };
        let config = self.config();
        let slept = now.saturating_sub(sleeper.started);
        if slept > 0 {
            self.set_tired_until(event.player, now + (slept as f64 / REST_MULTIPLIER) as i64);
        }
        let Some(block) = level.block(host, event.pos) else {
            return;
        };
        if !runtime::is_sleeping_bed(&block.id(host)) {
            return;
        }
        if should_break(&config, slept, level_roll(host, level)) {
            break_bag(host, level, event.pos);
            if let Some(player) = host.player(event.player) {
                let _ = player.action_bar(
                    host,
                    Text::translate("block.comforts.sleeping_bag.broke", Vec::new()),
                );
            }
        } else if let Some(before) = sleeper.phantom_before {
            // Woke before deep sleep, so the value was never put back: restore it now
            // (the ticks slept are lost from the counter, under a thousand).
            if let Some(player) = host.player(event.player)
                && let Some(current) = player.stat(host, CUSTOM_STAT, TIME_SINCE_REST)
            {
                let delta = leave_delta(before, current);
                if delta != 0 {
                    let _ = player.award_stat(host, CUSTOM_STAT, TIME_SINCE_REST, delta);
                }
            }
        }
    }

    /// A respawn point is about to be set: a sleeping bag never becomes one
    /// (`ComfortsEvents.canSetSpawn`). The retired hook named the `bed` cause; this one
    /// names the position, and the block there answers whether it was a bag.
    fn player_set_spawn(
        &self,
        host: &mut dyn ServerHost,
        event: &mut PlayerSetSpawnEvent<'_>,
    ) -> Flow {
        let Some(pos) = event.pos else {
            return Flow::Continue;
        };
        let Some(level) = host.level(event.dimension) else {
            return Flow::Continue;
        };
        let Some(block) = level.block(host, pos) else {
            return Flow::Continue;
        };
        if runtime::is_sleeping_bed(&block.id(host)) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// Enough players slept: a sleeping-bag sleeper gets upstream's day/night wake
    /// offsets (`ComfortsEvents.getWakeTime`). Sleeps in vanilla beds are left alone, and
    /// contract version 2 asks for a [`ClockAdjustment`] rather than a written time.
    fn sleep_finished_time(
        &self,
        _host: &mut dyn ServerHost,
        event: &mut SleepFinishedTimeEvent<'_>,
    ) -> Flow {
        let Some(day_sleeping) = self.day_sleeping_in(event.dimension) else {
            return Flow::Continue;
        };
        let target = wake_target(&self.config(), day_sleeping, event.current, event.new_time);
        *event.adjustment = ClockAdjustment::Absolute(target);
        Flow::Continue
    }
}

impl Mod {
    /// The current rules, or upstream's defaults before the file is read.
    fn config(&self) -> Config {
        self.config
            .lock()
            .expect("comforts config lock poisoned")
            .clone()
            .unwrap_or_default()
    }

    fn players(&self) -> MutexGuard<'_, BTreeMap<u128, PlayerSleep>> {
        self.players.lock().expect("comforts sleep lock poisoned")
    }

    /// A config file was read: only this mod's own server config replaces the rules.
    fn read_config(&self, config: &ConfigFile) {
        self.read_config_text(&config.mod_id, config.kind, &config.text);
    }

    /// The file's name, kind and text, so the matching rule is testable without
    /// building a non-exhaustive [`ConfigFile`].
    fn read_config_text(&self, mod_id: &str, kind: ConfigKind, text: &str) {
        if mod_id != runtime::MOD_ID || kind != ConfigKind::Server {
            return;
        }
        *self.config.lock().expect("comforts config lock poisoned") =
            Some(runtime::parse_config(text));
    }

    /// The server dropped its config: back to the defaults, with no sleeper left over
    /// from that world.
    fn drop_config(&self, config: &ConfigFile) {
        self.drop_config_of(&config.mod_id, config.kind);
    }

    /// The matching rule for [`Self::drop_config`], testable for the same reason as
    /// [`Self::read_config_text`].
    fn drop_config_of(&self, mod_id: &str, kind: ConfigKind) {
        if mod_id != runtime::MOD_ID || kind != ConfigKind::Server {
            return;
        }
        *self.config.lock().expect("comforts config lock poisoned") = None;
        self.players().clear();
    }

    fn begin_sleep(
        &self,
        player: u128,
        config: &Config,
        dimension: &str,
        started: i64,
        phantom_before: Option<i32>,
    ) {
        let sleeper = Sleeper {
            dimension: dimension.to_owned(),
            started,
            day_sleeping: matches!(config.sleeping_bag_use, TimeUse::Day | TimeUse::DayOrNight),
            phantom_before,
        };
        self.players().entry(player).or_default().sleeper = Some(sleeper);
    }

    fn take_sleeper(&self, player: u128) -> Option<Sleeper> {
        self.players()
            .get_mut(&player)
            .and_then(|record| record.sleeper.take())
    }

    fn take_phantom_before(&self, player: u128) -> Option<i32> {
        self.players()
            .get_mut(&player)
            .and_then(|record| record.sleeper.as_mut())
            .and_then(|sleeper| sleeper.phantom_before.take())
    }

    fn set_tired_until(&self, player: u128, tired: i64) {
        self.players().entry(player).or_default().tired_until = tired;
    }

    fn tired_until(&self, player: u128) -> i64 {
        self.players()
            .get(&player)
            .map_or(0, |record| record.tired_until)
    }

    /// Whether a sleeper of ours is in `dimension`, and whether their sleep skips the
    /// day. Upstream asks every sleeper's block; the dimension is what the wake time
    /// names.
    fn day_sleeping_in(&self, dimension: &str) -> Option<bool> {
        self.players()
            .values()
            .find_map(|record| match &record.sleeper {
                Some(sleeper) if sleeper.dimension == dimension => Some(sleeper.day_sleeping),
                _ => None,
            })
    }
}

/// The foot's position from the head and the bed's `facing` (the pin's own
/// `bed_forward` rule: `facing` points foot → head).
fn foot_pos(head: [i32; 3], facing: Option<&str>) -> [i32; 3] {
    let offset = match facing {
        Some("south") => [0, 0, 1],
        Some("west") => [-1, 0, 0],
        Some("east") => [1, 0, 0],
        _ => [0, 0, -1],
    };
    [
        head[0] - offset[0],
        head[1] - offset[1],
        head[2] - offset[2],
    ]
}

/// The hook's answer for a sleep attempt (`ComfortsEvents.checkTime`, then
/// `restrictSleeping`). A problem vanilla already found is never replaced by the
/// restriction: upstream only refuses a sleeper vanilla would have let in.
fn problem_after(config: &Config, vanilla: Option<&str>, now: i64, tired_until: i64) -> Problem {
    let (problem, allowed) = match runtime::time_check(config, now.rem_euclid(24_000)) {
        runtime::SleepCheck::Allow => (None, true),
        runtime::SleepCheck::Deny => return Problem::Now,
        runtime::SleepCheck::Default => (vanilla.map(str::to_owned), false),
    };
    if problem.is_none() && !runtime::may_sleep(config.restrict_sleeping, tired_until, now) {
        return Problem::NotSleepy;
    }
    if allowed {
        Problem::Clear
    } else {
        Problem::Keep
    }
}

/// The break rule: upstream's `timeSlept > 500` gate and the configured percentage. A
/// level whose random cannot be read never breaks the bag rather than always breaking it
/// (a missing roll is not a roll of zero).
fn should_break(config: &Config, slept: i64, roll: Option<f64>) -> bool {
    slept > BREAK_AFTER_TICKS
        && roll.is_some_and(|roll| runtime::break_roll(config.sleeping_bag_break_chance, roll))
}

/// The correction that puts the pre-sleep phantom timer back at deep sleep: the game has
/// counted [`DEEP_SLEEP_TICKS`] since the reset.
fn deep_delta(before: i32, current: i32) -> i32 {
    before
        .saturating_add(DEEP_SLEEP_TICKS)
        .saturating_sub(current)
}

/// The correction for a sleeper who woke before deep sleep: the game reset the timer and
/// never put it back, so the whole gap is restored (the ticks slept are lost).
fn leave_delta(before: i32, current: i32) -> i32 {
    before.saturating_sub(current)
}

/// The clock time everyone wakes at, from the sleeper's kind of sleep and the level's own
/// clock (`runtime::wake_time` with `is_day` read at the wake).
fn wake_target(config: &Config, day_sleeping: bool, current: i64, new_time: i64) -> i64 {
    runtime::wake_time(
        config,
        day_sleeping,
        runtime::is_day(current),
        current,
        new_time,
    )
}

/// A draw from the level's own random stream, stored back so vanilla's later draws move
/// as a Java mod's would (`Level.random`).
fn level_roll(host: &mut dyn ServerHost, level: mod_api::LevelRef) -> Option<f64> {
    use mod_api::RandomSource;

    let mut random = level.random(host)?;
    let roll = random.next_double();
    let _ = level.set_random(host, random);
    Some(roll)
}

/// Both halves of the bag are removed without drops (`ComfortsEvents.onWakeUp`), with
/// wool's break sound.
fn break_bag(host: &mut dyn ServerHost, level: mod_api::LevelRef, head: [i32; 3]) {
    let Some(air) = host.block_state_id("minecraft:air") else {
        return;
    };
    let facing = level.block_property(host, head, "facing");
    let flags = BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS;
    for pos in [head, foot_pos(head, facing.as_deref())] {
        let ours = level
            .block(host, pos)
            .is_some_and(|block| runtime::is_sleeping_bed(&block.id(host)));
        if ours {
            let _ = level.set_block(host, pos, air, flags);
        }
    }
    let center = [
        f64::from(head[0]) + 0.5,
        f64::from(head[1]) + 0.5,
        f64::from(head[2]) + 0.5,
    ];
    level.play_sound(
        host,
        "minecraft:block.wool.break",
        center,
        SoundSource::Blocks,
        1.0,
        1.0,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foot_is_one_step_back_from_the_head() {
        assert_eq!(foot_pos([3, 64, 5], Some("south")), [3, 64, 4]);
        assert_eq!(foot_pos([3, 64, 5], Some("east")), [2, 64, 5]);
        assert_eq!(foot_pos([3, 64, 5], Some("north")), [3, 64, 6]);
        assert_eq!(foot_pos([3, 64, 5], None), [3, 64, 6]);
    }

    #[test]
    fn the_glue_reads_only_its_own_server_file() {
        let module = Mod::default();
        module.read_config_text(
            "other",
            ConfigKind::Server,
            "sleepingBagBreakChance = 100\n",
        );
        assert_eq!(
            module.config(),
            Config::default(),
            "another mod's file is ignored"
        );
        module.read_config_text(
            "comforts",
            ConfigKind::Server,
            "sleepingBagBreakChance = 40\nrestrictSleeping = true\n",
        );
        assert_eq!(module.config().sleeping_bag_break_chance, 40.0);
        assert!(module.config().restrict_sleeping);
        module.drop_config_of("comforts", ConfigKind::Server);
        assert_eq!(module.config(), Config::default());
    }

    #[test]
    fn a_sleeper_is_recorded_with_the_configs_day_sleep() {
        let module = Mod::default();
        let night = Config::default();
        module.begin_sleep(7, &night, "minecraft:overworld", 18_000, None);
        let sleeper = module.take_sleeper(7).expect("recorded");
        assert_eq!(sleeper.dimension, "minecraft:overworld");
        assert_eq!(sleeper.started, 18_000);
        assert!(!sleeper.day_sleeping);
        assert!(module.take_sleeper(7).is_none(), "taken once");

        let day = Config {
            sleeping_bag_use: TimeUse::Day,
            ..Config::default()
        };
        module.begin_sleep(7, &day, "minecraft:overworld", 6_000, Some(1_200));
        assert_eq!(
            module.day_sleeping_in("minecraft:overworld"),
            Some(true),
            "a DAY sleeper skips the day"
        );
        let sleeper = module.take_sleeper(7).expect("recorded");
        assert!(sleeper.day_sleeping);
        assert_eq!(sleeper.phantom_before, Some(1_200));
        assert_eq!(
            module.day_sleeping_in("minecraft:overworld"),
            None,
            "the record was taken"
        );
    }

    #[test]
    fn the_deep_sleep_restores_the_pre_sleep_value() {
        assert_eq!(deep_delta(1_000, 1_900), -800);
        assert_eq!(deep_delta(1_000, 1_100), 0);
        assert_eq!(deep_delta(0, 190_000), -189_900);
    }

    #[test]
    fn the_early_wake_restores_what_it_can() {
        assert_eq!(leave_delta(1_000, 1_900), -900);
        assert_eq!(leave_delta(1_000, 1_000), 0);
    }

    #[test]
    fn the_bag_breaks_only_after_the_long_sleep_and_by_chance() {
        let sure = Config {
            sleeping_bag_break_chance: 100.0,
            ..Config::default()
        };
        let never = Config::default();
        assert!(should_break(&sure, 501, Some(0.5)));
        assert!(
            !should_break(&sure, 500, Some(0.5)),
            "upstream's gate is strictly over 500"
        );
        assert!(!should_break(&never, 5_000, Some(0.5)));
        assert!(
            !should_break(&sure, 5_000, None),
            "no level random, no break"
        );
    }

    #[test]
    fn the_spawn_problem_maps_the_config_onto_the_hook() {
        let none = Config {
            sleeping_bag_use: TimeUse::None,
            ..Config::default()
        };
        assert_eq!(problem_after(&none, None, 18_000, 0), Problem::Now);
        let night = Config::default();
        assert_eq!(
            problem_after(&night, Some("not_safe"), 18_000, 0),
            Problem::Keep
        );
        assert_eq!(problem_after(&night, None, 18_000, 0), Problem::Keep);
        let day = Config {
            sleeping_bag_use: TimeUse::Day,
            ..Config::default()
        };
        assert_eq!(
            problem_after(&day, Some("not_possible_now"), 6_000, 0),
            Problem::Clear,
            "an allowed day sleep clears what vanilla refused"
        );
        assert_eq!(
            problem_after(&day, Some("not_safe"), 6_000, 0),
            Problem::Clear,
            "upstream clears any problem it allows, the monster check included"
        );
        let day_restrict = Config {
            sleeping_bag_use: TimeUse::Day,
            restrict_sleeping: true,
            ..Config::default()
        };
        assert_eq!(
            problem_after(&day_restrict, Some("not_possible_now"), 6_000, 30_000),
            Problem::NotSleepy,
            "restrictSleeping applies after the window allows the sleep"
        );
        let restrict = Config {
            restrict_sleeping: true,
            ..Config::default()
        };
        assert_eq!(
            problem_after(&restrict, None, 18_000, 30_000),
            Problem::NotSleepy
        );
        assert_eq!(
            problem_after(&restrict, None, 18_000, 18_000),
            Problem::Keep
        );
        assert_eq!(
            problem_after(&restrict, Some("not_safe"), 18_000, 30_000),
            Problem::Keep,
            "a vanilla problem is not replaced by the restriction"
        );
    }

    #[test]
    fn the_wake_target_follows_the_sleepers_kind_of_sleep() {
        let config = Config::default();
        assert_eq!(wake_target(&config, false, 18_000, 24_000), 24_000);
        assert_eq!(wake_target(&config, true, 6_000, 11_999), 11_999);
        let later = Config {
            day_wake_time_offset: 500,
            ..Config::default()
        };
        assert_eq!(wake_target(&later, false, 18_000, 24_000), 24_500);
    }
}
