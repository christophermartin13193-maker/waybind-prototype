use std::collections::HashMap;

mod entity;

use input::event::keyboard::{KeyState, KeyboardEventTrait};
use xkbcommon_rs::{
    self, Context, Keymap, State,
    xkb_state::{KeyDirection, StateComponent},
};

struct ModMask {
    pub m_super: u32,
    pub m_ctrl: u32,
}

impl ModMask {
    fn new(keymap: &Keymap) -> Self {
        let m_super = 1u32 << keymap.mod_get_index("Mod4").expect("Mod4 isn't a key") as u32;
        let m_ctrl = 1u32 << keymap.mod_get_index("Control").expect("Control isn't a key") as u32;
        Self { m_super, m_ctrl }
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct Shortcut {
    pub modifiers: u32,
    pub key: String,
}

mod inp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = input::Libinput::new_with_udev(inp::Interface);
    input
        .udev_assign_seat("seat0")
        .expect("Échec de l'assignation du seat0");

    let keymap = Keymap::new_from_names(Context::new(0)?, None, 0)?;
    let mod_mask = ModMask::new(&keymap);
    let mut state = State::new(keymap);

    let mut map: HashMap<Shortcut, String> = HashMap::new();

    let modifiers = mod_mask.m_ctrl | mod_mask.m_super;
    let key = String::from("XK_q");
    let shortcut = Shortcut { modifiers, key };

    map.insert(shortcut, String::from("kitty"));

    println!("Démon en écoute des événements libinput...");

    loop {
        input.dispatch().unwrap();

        for event in &mut input {
            match event {
                input::Event::Keyboard(kb_event) => {
                    let raw_keycode = kb_event.key();
                    let key_state = kb_event.key_state();
                    let kc = xkbcommon_rs::keycode::RawKeycode::from(raw_keycode + 8);

                    let direction = match key_state {
                        KeyState::Pressed => KeyDirection::Down,
                        KeyState::Released => KeyDirection::Up,
                    };

                    // Update
                    state.update_key(kc, direction);

                    let active_mods = state.serialize_mods(StateComponent::MODS_EFFECTIVE);
                    let active_ctl = active_mods & mod_mask.m_ctrl;
                    let active_super = active_mods & mod_mask.m_super;
                    let key = state.key_get_one_sym(kc);

                    if let Some(key) = key && direction == KeyDirection::Down {
                        println!("key : {}", key.name().unwrap_or("Unknown"));
                        let shortcut = Shortcut { modifiers: active_mods, key: key.name().unwrap_or("Unknown").to_string() };
                        if let Some(action) = map.get(&shortcut) {
                            print!("{action}");
                            std::process::Command::new(action)
                                .env("DISPLAY", std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()))
                                .env("WAYLAND_DISPLAY", std::env::var("WAYLAND_DISPLAY").unwrap_or_default())
                                .env("XAUTHORITY", std::env::var("XAUTHORITY").unwrap_or_default())
                                .spawn()
                                .expect("Échec du lancement de kitty");
                        }
                    }
                    // if active_ctl != 0 { println!("Control is active")}
                    // if active_super != 0 { println!("Mod4 is active")}
                }
                _ => (),
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
