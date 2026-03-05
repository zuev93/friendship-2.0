use druzhba_common::PlatformMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use heapless::String;

pub use druzhba_common::protocol_types::MenuCommand;

pub static MENU_EVENTS: Channel<PlatformMutex, MenuCommand, 8> = Channel::new();

pub struct MenuItemView {
    pub label: &'static str,
    pub value: String<16>,
    pub is_submenu: bool,
}

pub struct MenuScreen {
    pub title: &'static str,
    pub items: heapless::Vec<MenuItemView, 16>,
    pub cursor: u8,
    pub active: bool,
}

pub type MenuScreenSignal = Signal<PlatformMutex, MenuScreen>;
