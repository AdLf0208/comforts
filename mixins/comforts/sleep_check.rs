// Head of `Game::native_can_player_sleep` (API 393): the bag's time window and
// `restrictSleeping` decide whether vanilla's refusal stands.
fn sleep_check(
    game: &mut crate::game::Game<'_>,
    player: u128,
    name: &str,
    pos: [i32; 3],
    problem: &mut Option<String>,
) {
    crate::game::comforts_state::on_sleep_check(game, player, name, pos, problem);
}
