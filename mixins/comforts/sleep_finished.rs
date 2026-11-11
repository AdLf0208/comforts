// Head of `Game::native_sleep_finished_time` (API 308): the wake time a
// sleeping-bag sleeper gets, with upstream's day and night offsets.
fn sleep_finished(dimension: &str, current: i64, new_time: &mut i64) {
    crate::game::comforts_state::on_sleep_finished_time(dimension, current, new_time);
}
