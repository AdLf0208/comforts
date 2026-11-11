// Head of `Game::native_register_configs` (API 397): list the server config,
// written from upstream's defaults when `config/comforts-server.toml` is
// missing.
fn register_configs(configs: &mut Vec<NativeConfig>) {
    crate::game::comforts_state::on_register_configs(configs);
}
