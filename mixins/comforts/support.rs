// Game-side glue for the sleeping-bag slice: the config registered through the
// pin's mod-config hooks, the per-sleeper record, and what each sleep hook
// does with it. No `//!`/`#![…]`: the planner injects this as a plain module.

use std::collections::BTreeMap;
use std::sync::Mutex;

use horizon_core::random::RandomSource;

use crate::game::comforts::{
    DEFAULT_CONFIG, MOD_ID, Config, SleepCheck, TimeUse, break_roll, is_day, is_sleeping_bed,
    may_sleep, parse_config, time_check, wake_time,
};
use crate::game::native_configs::{NativeConfig, NativeConfigFile, NativeConfigKind};

/// Upstream's `restMultiplier` default (2.0): a sleeper is tired for half the
/// time it slept. Not configurable in this slice.
const REST_MULTIPLIER: f64 = 2.0;

/// Upstream breaks the bag only after this long asleep
/// (`ComfortsEvents.onWakeUp`, `timeSlept > 500`).
const BREAK_AFTER_TICKS: i64 = 500;

/// `player_deep_sleep` fires at this many ticks asleep; by then the game has
/// counted that many ticks since rest.
const DEEP_SLEEP_TICKS: i32 = 100;

/// `Level.removeBlock`'s flags: neighbour and client updates, no drops.
const REMOVE_FLAGS: i32 = 3;

/// The loaded server config; `None` before the first load, which behaves as
/// the defaults so a world never loses sleeping.
static CONFIG: Mutex<Option<Config>> = Mutex::new(None);

/// One player's sleep state: the sleep in progress (if any) and the time it is
/// tired until. Upstream keeps this in a player capability; the port keeps it
/// for the session.
#[derive(Default)]
struct PlayerSleep {
    sleeper: Option<Sleeper>,
    /// The day time before which another sleep is refused.
    tired_until: i64,
}

struct Sleeper {
    /// The dimension the bag is in (`minecraft:overworld`).
    dimension: String,
    /// The day time the player lay down at.
    started: i64,
    /// Whether this block's time use can skip the day (a hammock; false for a
    /// sleeping bag at NIGHT).
    day_sleeping: bool,
    /// The time since rest before the game resets it, when
    /// `sleepingBagsStopPhantoms` is off.
    phantom_before: Option<i32>,
}

static PLAYERS: Mutex<BTreeMap<u128, PlayerSleep>> = Mutex::new(BTreeMap::new());

fn players() -> std::sync::MutexGuard<'static, BTreeMap<u128, PlayerSleep>> {
    PLAYERS.lock().expect("comforts sleep lock poisoned")
}

/// The current config, or upstream's defaults before it is read.
pub fn config() -> Config {
    CONFIG
        .lock()
        .expect("comforts config lock poisoned")
        .clone()
        .unwrap_or_default()
}

fn set_config(config: Option<Config>) {
    *CONFIG.lock().expect("comforts config lock poisoned") = config;
}

/// Head of `Game::native_register_configs` (API 397): the server config,
/// written from upstream's defaults when `config/comforts-server.toml` is
/// missing.
pub fn on_register_configs(configs: &mut Vec<NativeConfig>) {
    configs.push(NativeConfig::new(
        MOD_ID,
        NativeConfigKind::Server,
        DEFAULT_CONFIG,
    ));
}

fn is_our_config(config: &NativeConfigFile) -> bool {
    config.mod_id == MOD_ID && config.kind == NativeConfigKind::Server
}

/// A config file was read (`config_loading`, `config_reloading`).
pub fn on_config_loaded(config: &NativeConfigFile) {
    if !is_our_config(config) {
        return;
    }
    set_config(Some(parse_config(&config.text)));
}

/// The server dropped its configs (`config_unloading`): back to the defaults,
/// with no sleeper left over from that world.
pub fn on_config_unloaded(config: &NativeConfigFile) {
    if !is_our_config(config) {
        return;
    }
    set_config(None);
    players().clear();
}

/// Head of `Game::native_can_player_sleep` (API 393): the bag's time window
/// decides, then `restrictSleeping` may refuse a player who just slept.
#[expect(
    private_interfaces,
    reason = "the injected module is pub(crate) under crate::game; Game is private there"
)]
pub fn on_sleep_check(
    game: &mut crate::game::Game<'_>,
    player: u128,
    _name: &str,
    pos: [i32; 3],
    problem: &mut Option<String>,
) {
    if !is_sleeping_bed(&game.native_block(pos)) {
        return;
    }
    let config = config();
    let now = game.native_day_time();
    match time_check(&config, now.rem_euclid(24_000)) {
        SleepCheck::Allow => *problem = None,
        SleepCheck::Deny => *problem = Some("not_possible_now".to_owned()),
        SleepCheck::Default => {}
    }
    if problem.is_none() {
        let tired_until = players()
            .get(&player)
            .map_or(0, |player| player.tired_until);
        if !may_sleep(config.restrict_sleeping, tired_until, now) {
            *problem = Some("other_problem".to_owned());
            game.tell_sleeper(
                "capability.comforts.not_sleepy",
                "You are not sleepy yet",
                true,
            );
        }
    }
}

/// Head of `Game::native_player_bed_enter` (API 251): the checks passed, so
/// remember this sleeper. Never cancels a sleep the config allowed.
#[expect(
    private_interfaces,
    reason = "the injected module is pub(crate) under crate::game; Game is private there"
)]
pub fn on_bed_enter(
    game: &mut crate::game::Game<'_>,
    player: u128,
    _name: &str,
    pos: [i32; 3],
    _cancel: &mut bool,
) {
    if !is_sleeping_bed(&game.native_block(pos)) {
        return;
    }
    let config = config();
    let phantom_before = (!config.sleeping_bags_stop_phantoms)
        .then(|| game.stats.get("custom", "minecraft:time_since_rest"));
    let sleeper = Sleeper {
        dimension: game.dim.id().to_owned(),
        started: game.native_day_time(),
        day_sleeping: matches!(
            config.sleeping_bag_use,
            TimeUse::Day | TimeUse::DayOrNight
        ),
        phantom_before,
    };
    let mut players = players();
    let record = players.entry(player).or_default();
    record.sleeper = Some(sleeper);
}

/// `Game::native_player_deep_sleep` (API 256): the game has reset the time
/// since rest; put the pre-sleep value back so phantoms still come when
/// `sleepingBagsStopPhantoms` is off. Exact to a tick, because this fires 100
/// ticks after the reset.
#[expect(
    private_interfaces,
    reason = "the injected module is pub(crate) under crate::game; Game is private there"
)]
pub fn on_deep_sleep(game: &mut crate::game::Game<'_>, player: u128, _name: &str) {
    let before = {
        let mut players = players();
        players
            .get_mut(&player)
            .and_then(|record| record.sleeper.as_mut())
            .and_then(|sleeper| sleeper.phantom_before.take())
    };
    let Some(before) = before else {
        return;
    };
    let current = game.stats.get("custom", "minecraft:time_since_rest");
    let delta = before.saturating_add(DEEP_SLEEP_TICKS) - current;
    if delta != 0 {
        game.stats
            .add("custom", "minecraft:time_since_rest", delta);
    }
}

/// `Game::native_player_bed_leave` (API 252): the sleeper woke. The bag may
/// break (`sleepingBagBreakChance`), and a phantom-rest value that was not
/// restored at deep sleep is restored now.
#[expect(
    private_interfaces,
    reason = "the injected module is pub(crate) under crate::game; Game is private there"
)]
pub fn on_bed_leave(
    game: &mut crate::game::Game<'_>,
    player: u128,
    _name: &str,
    pos: [i32; 3],
) {
    let sleeper = players()
        .get_mut(&player)
        .and_then(|record| record.sleeper.take());
    let Some(sleeper) = sleeper else {
        return;
    };
    let config = config();
    let now = game.native_day_time();
    let slept = now.saturating_sub(sleeper.started);
    if slept > 0 {
        let tired = now + (slept as f64 / REST_MULTIPLIER) as i64;
        if let Some(record) = players().get_mut(&player) {
            record.tired_until = tired;
        }
    }
    if !is_sleeping_bed(&game.native_block(pos)) {
        return;
    }
    // `Level.random` (upstream's `level.random.nextDouble()`): the pin's
    // inherent `random` is a trait method, so read the world's random directly.
    let roll = game.world.random.next_double();
    if slept > BREAK_AFTER_TICKS && break_roll(config.sleeping_bag_break_chance, roll) {
        break_bag(game, pos);
        game.tell_sleeper(
            "block.comforts.sleeping_bag.broke",
            "Your sleeping bag broke",
            true,
        );
    } else if let Some(before) = sleeper.phantom_before {
        // Woke before deep sleep, so the value was never put back: restore it
        // now (the ticks slept are lost from the counter, under a thousand).
        let current = game.stats.get("custom", "minecraft:time_since_rest");
        let delta = before - current;
        if delta != 0 {
            game.stats
                .add("custom", "minecraft:time_since_rest", delta);
        }
    }
}

/// Both halves of the bag are removed without drops
/// (`ComfortsEvents.onWakeUp`), with wool's break sound.
fn break_bag(game: &mut crate::game::Game<'_>, head: [i32; 3]) {
    let state = game.world.block(head[0], head[1], head[2]);
    let facing = game.bed_forward(state);
    let foot = [
        head[0] - facing[0],
        head[1] - facing[1],
        head[2] - facing[2],
    ];
    let air = game.world.air;
    if is_sleeping_bed(&game.native_block(head)) {
        if let Err(e) = game.level_set_with_flags(head, air, REMOVE_FLAGS) {
            log::warn!("comforts: could not break the sleeping bag at {head:?}: {e}");
        }
    }
    if is_sleeping_bed(&game.native_block(foot)) {
        if let Err(e) = game.level_set_with_flags(foot, air, REMOVE_FLAGS) {
            log::warn!("comforts: could not break the sleeping bag at {foot:?}: {e}");
        }
    }
    let center = [
        f64::from(head[0]) + 0.5,
        f64::from(head[1]) + 0.5,
        f64::from(head[2]) + 0.5,
    ];
    game.audio
        .play("minecraft:block.wool.break", Some(center), 1.0, 1.0);
}

/// A server player's respawn point is about to change (API 277): a sleeping
/// bag never sets one (`ComfortsEvents.canSetSpawn`).
#[expect(
    private_interfaces,
    reason = "the injected module is pub(crate) under crate::game; Game is private there"
)]
pub fn on_spawn_point_set(
    game: &mut crate::game::Game<'_>,
    _player: u128,
    _name: &str,
    _dimension: &str,
    pos: [i32; 3],
    cause: &str,
    cancel: &mut bool,
) {
    if cause != "bed" {
        return;
    }
    if is_sleeping_bed(&game.native_block(pos)) {
        *cancel = true;
    }
}

/// The time everyone wakes at (API 308): a sleeping-bag sleeper uses
/// upstream's offsets. Sleeps in vanilla beds are left alone.
pub fn on_sleep_finished_time(
    dimension: &str,
    current: i64,
    new_time: &mut i64,
) {
    let day_sleeping = players()
        .values()
        .find_map(|record| match &record.sleeper {
            Some(sleeper) if sleeper.dimension == dimension => Some(sleeper.day_sleeping),
            _ => None,
        });
    let Some(day_sleeping) = day_sleeping else {
        return;
    };
    let target = wake_time(&config(), day_sleeping, is_day(current), current, *new_time);
    if target >= current {
        *new_time = target;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registered_config_is_upstreams_server_file() {
        let mut configs = Vec::new();
        on_register_configs(&mut configs);
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].mod_id, MOD_ID);
        assert_eq!(configs[0].kind, NativeConfigKind::Server);
        assert_eq!(configs[0].file, "comforts-server.toml");
        assert_eq!(configs[0].default, DEFAULT_CONFIG);
    }

    /// One test for the globals: another mod's file changes nothing, ours
    /// replaces the rules, and unloading drops them.
    #[test]
    fn config_files_replace_and_drop_the_rules() {
        let file = |mod_id: &str, text: &str| NativeConfigFile {
            mod_id: mod_id.to_owned(),
            kind: NativeConfigKind::Server,
            file: format!("{mod_id}-server.toml"),
            path: format!("config/{mod_id}-server.toml").into(),
            text: text.to_owned(),
        };
        set_config(None);
        on_config_loaded(&file("other", "sleepingBagBreakChance = 100\n"));
        assert_eq!(config(), Config::default(), "another mod's file is ignored");
        let ours = file(
            MOD_ID,
            "sleepingBagBreakChance = 40\nrestrictSleeping = true\n",
        );
        on_config_loaded(&ours);
        assert_eq!(config().sleeping_bag_break_chance, 40.0);
        assert!(config().restrict_sleeping);
        on_config_unloaded(&ours);
        assert_eq!(config(), Config::default());
    }
}
