// Head of `Game::native_player_deep_sleep` (API 256): 100 ticks asleep, which is
// where `sleepingBagsStopPhantoms=false` puts the time since rest back.
fn deep_sleep(game: &mut crate::game::Game<'_>, player: u128, name: &str) {
    crate::game::comforts_state::on_deep_sleep(game, player, name);
}
