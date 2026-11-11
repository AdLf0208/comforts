// Head of `Game::native_player_bed_enter` (API 251): the sleep checks passed,
// so remember the sleeper (start time, dimension, whether it skips the day).
fn bed_enter(
    game: &mut crate::game::Game<'_>,
    player: u128,
    name: &str,
    pos: [i32; 3],
    cancel: &mut bool,
) {
    crate::game::comforts_state::on_bed_enter(game, player, name, pos, cancel);
}
