// Head of `Game::native_config_unloading` (API 397): the server stopped, so
// drop its config and the sleepers of that world.
fn config_unloading(config: &NativeConfigFile) {
    crate::game::comforts_state::on_config_unloaded(config);
}
