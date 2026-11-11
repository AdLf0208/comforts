// Head of `Game::native_player_spawn_point_set` (API 277): a sleeping bag never
// becomes the player's respawn point, so its bed cause is cancelled.
fn spawn_point(
    game: &mut crate::game::Game<'_>,
    player: u128,
    name: &str,
    dimension: &str,
    pos: [i32; 3],
    cause: &str,
    cancel: &mut bool,
) {
    crate::game::comforts_state::on_spawn_point_set(
        game, player, name, dimension, pos, cause, cancel,
    );
}
