// Head of `Game::native_config_loading` (API 397): read our server config.
fn config_loading(config: &NativeConfigFile) {
    crate::game::comforts_state::on_config_loaded(config);
}
