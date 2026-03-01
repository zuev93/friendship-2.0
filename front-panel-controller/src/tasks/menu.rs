use embassy_executor::Spawner;
use heapless::String;

use crate::state::menu::{MenuCommand, MenuItemView, MenuScreen, MenuScreenSignal, MENU_EVENTS};

struct MenuItem {
    label: &'static str,
    kind: MenuItemKind,
}

enum MenuItemKind {
    Submenu(&'static [MenuItem]),
    Info(&'static str),
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
];

struct MenuState {
    active: bool,
    path: heapless::Vec<u8, 4>,
    cursor: u8,
}

impl MenuState {
    const fn new() -> Self {
        Self {
            active: false,
            path: heapless::Vec::new(),
            cursor: 0,
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
            MenuItemKind::Info(_) => break,
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
            MenuItemKind::Info(_) => break,
        }
        i += 1;
    }
    title
}

fn build_screen(state: &MenuState) -> MenuScreen {
    let items = current_items(&state.path);
    let title = current_title(&state.path);

    let mut views = heapless::Vec::new();
    for item in items {
        let is_submenu = matches!(item.kind, MenuItemKind::Submenu(_));
        let value = match &item.kind {
            MenuItemKind::Info(v) => String::try_from(*v).unwrap_or_default(),
            MenuItemKind::Submenu(_) => String::new(),
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

pub fn spawn_tasks(spawner: &Spawner, menu_screen_signal: &'static MenuScreenSignal) {
    spawner.must_spawn(menu_task(menu_screen_signal));
}

#[embassy_executor::task]
async fn menu_task(menu_screen_signal: &'static MenuScreenSignal) {
    let mut state = MenuState::new();

    loop {
        let cmd = MENU_EVENTS.receive().await;

        let changed = match cmd {
            MenuCommand::Ok => {
                if !state.active {
                    state.active = true;
                    state.cursor = 0;
                    state.path.clear();
                } else {
                    let items = current_items(&state.path);
                    if let Some(item) = items.get(state.cursor as usize) {
                        if matches!(item.kind, MenuItemKind::Submenu(_)) {
                            let _ = state.path.push(state.cursor);
                            state.cursor = 0;
                        }
                    }
                }
                true
            }
            MenuCommand::Cancel => {
                if !state.active {
                    false
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
            menu_screen_signal.signal(build_screen(&state));
        }
    }
}
