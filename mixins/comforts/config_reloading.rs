// Head of `Game::native_config_reloading` (API 397): the file changed on disk,
// so read it again without a restart.
fn config_reloading(config: &NativeConfigFile) {
    crate::game::comforts_state::on_config_loaded(config);
}
