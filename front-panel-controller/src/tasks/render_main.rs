use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;

use crate::hardware::Displays;
use crate::state::input::RadioStateSignal;
use crate::state::menu::MenuScreenSignal;
use crate::ui;

pub fn spawn_tasks(
    spawner: &Spawner,
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    radio_state_signal: &'static RadioStateSignal,
    menu_screen_signal: &'static MenuScreenSignal,
    display_index: usize,
) {
    spawner.must_spawn(render_main_task(
        displays,
        radio_state_signal,
        menu_screen_signal,
        display_index,
    ));
}

#[embassy_executor::task]
async fn render_main_task(
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    radio_state_signal: &'static RadioStateSignal,
    menu_screen_signal: &'static MenuScreenSignal,
    display_index: usize,
) {
    let mut menu_active = false;
    let mut last_radio_state = None;

    loop {
        match select(radio_state_signal.wait(), menu_screen_signal.wait()).await {
            Either::First(state) => {
                last_radio_state = Some(state);
                if menu_active {
                    continue;
                }
                let mut d = displays.lock().await;
                let display = &mut d.displays[display_index];
                ui::main_screen::render(&mut display.fb, &state);
                let front = display.fb.swap();
                if display.driver.draw(front).await.is_ok() {
            display.count_frame();
        }
            }
            Either::Second(menu) => {
                menu_active = menu.active;
                let mut d = displays.lock().await;
                let display = &mut d.displays[display_index];
                if menu.active {
                    ui::menu_screen::render(&mut display.fb, &menu);
                } else if let Some(state) = &last_radio_state {
                    ui::main_screen::render(&mut display.fb, state);
                } else {
                    continue;
                }
                let front = display.fb.swap();
                if display.driver.draw(front).await.is_ok() {
            display.count_frame();
        }
            }
        }
    }
}
