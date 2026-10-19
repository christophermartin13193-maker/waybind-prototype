use std::collections::HashMap;

use xkbcommon_rs::Keymap;

pub type ID = u64;
pub type Mask = u32;
pub type Map = HashMap<Bind, Vec<ShortcutID>>;

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Push,
    Hold(u64),
    Released(u64),
}

#[derive(Debug)]
pub struct ShortcutID {
    pub id: ID,
    pub exec: String,
    pub action: Action,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Bind(pub Mask, pub String);

#[derive(Debug, Clone, Copy)]
pub struct ModMask {
    pub m_super: u32,
    pub m_ctrl: u32,
}

impl ModMask {
    pub fn init(keymap: &Keymap) -> Self {
        const MOD4: &str = "Mod4";
        const CTRL: &str = "Control";

        let m_super = 1u32
            << keymap
                .mod_get_index(MOD4)
                .expect(&format!("{MOD4} is invalid"));
        let m_ctrl = 1u32
            << keymap
                .mod_get_index(CTRL)
                .expect(&format!("{CTRL} is invalid"));

        Self { m_super, m_ctrl }
    }
}
