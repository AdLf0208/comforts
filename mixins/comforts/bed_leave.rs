// Head of `Game::native_player_bed_leave` (API 252): the sleeper woke, so the
// bag may break and the phantom rest may be restored.
fn bed_leave(game: &mut crate::game::Game<'_>, player: u128, name: &str, pos: [i32; 3]) {
    crate::game::comforts_state::on_bed_leave(game, player, name, pos);
}
