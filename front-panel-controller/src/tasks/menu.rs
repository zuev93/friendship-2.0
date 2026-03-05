use core::fmt::Write;
use embassy_executor::Spawner;
use heapless::String;

use druzhba_front_panel_controller::state::error_log::ErrorLog;
use druzhba_front_panel_controller::state::menu::{MenuCommand, MenuItemView, MenuScreen, MenuScreenSignal, MENU_EVENTS};

struct MenuItem {
    label: &'static str,
    kind: MenuItemKind,
}

enum MenuItemKind {
    Submenu(&'static [MenuItem]),
    Info(&'static str),
    ErrorLog,
}

static ROOT_ITEMS: &[MenuItem] = &[
    MenuItem {
        label: "Radio Info",
        kind: MenuItemKind::Submenu(&[
            MenuItem { label: "Frequency", kind: MenuItemKind::Info("") },
            MenuItem { label: "Mode", kind: MenuItemKind::Info("") },
            MenuItem { label: "Band", kind: MenuItemKind::Info("") },
        ]),
    },
    MenuItem {
        label: "Hardware",
        kind: MenuItemKind::Submenu(&[
            MenuItem { label: "FW Version", kind: MenuItemKind::Info("0.1.0") },
        ]),
    },
    MenuItem {
        label: "Error Log",
        kind: MenuItemKind::ErrorLog,
    },
];

struct MenuState {
    active: bool,
    path: heapless::Vec<u8, 4>,
    cursor: u8,
    in_error_log: bool,
}

impl MenuState {
    const fn new() -> Self {
        Self {
            active: false,
            path: heapless::Vec::new(),
            cursor: 0,
            in_error_log: false,
        }
    }
}

fn current_items(path: &heapless::Vec<u8, 4>) -> &'static [MenuItem] {
    let mut items: &[MenuItem] = ROOT_ITEMS;
    let mut i = 0;
    while i < path.len() {
        let idx = path[i] as usize;
        match &items[idx].kind {
            MenuItemKind::Submenu(sub) => items = sub,
            MenuItemKind::Info(_) | MenuItemKind::ErrorLog => break,
        }
        i += 1;
    }
    items
}

fn current_title(path: &heapless::Vec<u8, 4>) -> &'static str {
    if path.is_empty() {
        return "Menu";
    }
    let mut items: &[MenuItem] = ROOT_ITEMS;
    let mut title = "Menu";
    let mut i = 0;
    while i < path.len() {
        let idx = path[i] as usize;
        title = items[idx].label;
        match &items[idx].kind {
            MenuItemKind::Submenu(sub) => items = sub,
            MenuItemKind::Info(_) | MenuItemKind::ErrorLog => break,
        }
        i += 1;
    }
    title
}

async fn build_screen(state: &MenuState, error_log: &ErrorLog) -> MenuScreen {
    let items = current_items(&state.path);
    let title = current_title(&state.path);

    if state.in_error_log {
        return build_error_log_screen(state, error_log).await;
    }

    let mut views = heapless::Vec::new();
    for item in items {
        let is_submenu = matches!(item.kind, MenuItemKind::Submenu(_) | MenuItemKind::ErrorLog);
        let value = match &item.kind {
            MenuItemKind::Info(v) => String::try_from(*v).unwrap_or_default(),
            MenuItemKind::Submenu(_) => String::new(),
            MenuItemKind::ErrorLog => {
                let mut s = String::new();
                let _ = write!(s, "{}", error_log.total());
                s
            }
        };
        let _ = views.push(MenuItemView {
            label: item.label,
            value,
            is_submenu,
        });
    }

    MenuScreen {
        title,
        items: views,
        cursor: state.cursor,
        active: state.active,
    }
}

async fn build_error_log_screen(state: &MenuState, error_log: &ErrorLog) -> MenuScreen {
    let mut entries = [""; 16];
    let count = error_log.recent(&mut entries).await;

    let mut views = heapless::Vec::new();
    if count == 0 {
        let _ = views.push(MenuItemView {
            label: "No errors",
            value: String::new(),
            is_submenu: false,
        });
    } else {
        for entry in entries[..count].iter().rev() {
            let _ = views.push(MenuItemView {
                label: entry,
                value: String::new(),
                is_submenu: false,
            });
        }
    }

    MenuScreen {
        title: "Error Log",
        items: views,
        cursor: state.cursor,
        active: state.active,
    }
}

pub fn spawn_tasks(spawner: &Spawner, menu_screen_signal: &'static MenuScreenSignal, error_log: &'static ErrorLog) {
    spawner.must_spawn(menu_task(menu_screen_signal, error_log));
}

#[embassy_executor::task]
async fn menu_task(menu_screen_signal: &'static MenuScreenSignal, error_log: &'static ErrorLog) {
    let mut state = MenuState::new();

    loop {
        let cmd = MENU_EVENTS.receive().await;

        let changed = match cmd {
            MenuCommand::Ok => {
                if !state.active {
                    state.active = true;
                    state.cursor = 0;
                    state.path.clear();
                    state.in_error_log = false;
                } else if !state.in_error_log {
                    let items = current_items(&state.path);
                    if let Some(item) = items.get(state.cursor as usize) {
                        match item.kind {
                            MenuItemKind::Submenu(_) => {
                                let _ = state.path.push(state.cursor);
                                state.cursor = 0;
                            }
                            MenuItemKind::ErrorLog => {
                                state.in_error_log = true;
                                state.cursor = 0;
                            }
                            MenuItemKind::Info(_) => {}
                        }
                    }
                }
                true
            }
            MenuCommand::Cancel => {
                if !state.active {
                    false
                } else if state.in_error_log {
                    state.in_error_log = false;
                    true
                } else if state.path.is_empty() {
                    state.active = false;
                    true
                } else {
                    state.cursor = state.path.pop().unwrap_or(0);
                    true
                }
            }
            MenuCommand::Scroll(delta) => {
                if !state.active {
                    false
                } else {
                    let items = current_items(&state.path);
                    let count = items.len() as u8;
                    if count == 0 {
                        false
                    } else {
                        let new_cursor = state.cursor as i8 + delta;
                        if new_cursor < 0 {
                            state.cursor = 0;
                        } else if new_cursor >= count as i8 {
                            state.cursor = count - 1;
                        } else {
                            state.cursor = new_cursor as u8;
                        }
                        true
                    }
                }
            }
        };

        if changed {
            menu_screen_signal.signal(build_screen(&state, error_log).await);
        }
    }
}
