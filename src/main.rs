use std::{collections::HashMap, sync::mpsc};

use input::event::keyboard::{KeyState, KeyboardEventTrait};
use xkbcommon_rs::{
    Context, Keymap, State,
    xkb_state::{KeyDirection, StateComponent},
};

mod entity;
mod inp;

fn build_map(mod_mask: entity::ModMask) -> entity::Map {
    let mut map = entity::Map::new();
    // build keys
    let bind1 = entity::Bind(mod_mask.m_ctrl | mod_mask.m_super, String::from("XK_h"));
    let bind2 = entity::Bind(mod_mask.m_ctrl | mod_mask.m_super, String::from("XK_y"));
    let bind3 = entity::Bind(mod_mask.m_ctrl | mod_mask.m_super, String::from("XK_u"));
    // build value1
    let shortcut1 = entity::ShortcutID {
        id: 0,
        exec: String::from("kitty"),
        action: entity::Action::Push,
    };
    let shortcut2 = entity::ShortcutID {
        id: 1,
        exec: String::from("mgba-qt"),
        action: entity::Action::Push,
    };
    let shortcuts1 = vec![shortcut1, shortcut2];
    // build value2
    let shortcut3 = entity::ShortcutID {
        id: 2,
        exec: String::from("kitty"),
        action: entity::Action::Hold(2000),
    };
    let shortcut4 = entity::ShortcutID {
        id: 3,
        exec: String::from("mgba-qt"),
        action: entity::Action::Hold(3000),
    };
    let shortcuts2 = vec![shortcut3, shortcut4];
    // build value3
    let shortcut5 = entity::ShortcutID {
        id: 4,
        exec: String::from("kitty"),
        action: entity::Action::Released(5),
    };
    let shortcut6 = entity::ShortcutID {
        id: 5,
        exec: String::from("mgba-qt"),
        action: entity::Action::Released(1000),
    };
    let shortcuts3 = vec![shortcut5, shortcut6];

    map.insert(bind1, shortcuts1);
    map.insert(bind2, shortcuts2);
    map.insert(bind3, shortcuts3);
    map
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // entity
    let keymap = Keymap::new_from_names(Context::new(0)?, None, 0)?;
    let mod_mask = entity::ModMask::init(&keymap);
    let map = build_map(mod_mask);
    let mut active_holds: HashMap<String, Vec<mpsc::Sender<()>>> = HashMap::new();
    let mut active_released: HashMap<String, mpsc::Sender<()>> = HashMap::new();
    let mut state = State::new(keymap);

    // mpsc
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    std::thread::spawn(move || {
        while let Ok(exec) = rx.recv() {
            match std::process::Command::new(exec)
                .env(
                    "DISPLAY",
                    std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()),
                )
                .env(
                    "WAYLAND_DISPLAY",
                    std::env::var("WAYLAND_DISPLAY").unwrap_or_default(),
                )
                .spawn()
            {
                Ok(_) => (),
                Err(err) => eprintln!("{err}"),
            }
        }
    });

    // input
    let mut input = input::Libinput::new_with_udev(inp::Interface);
    input
        .udev_assign_seat("seat0")
        .expect("Couldn't assign seat0");

    loop {
        input.dispatch().unwrap();

        for event in &mut input {
            match event {
                input::Event::Keyboard(kb_event) => {
                    // update state
                    let key_state = kb_event.key_state();
                    let kc = xkbcommon_rs::keycode::RawKeycode::from(kb_event.key() + 8);
                    let direction = match key_state {
                        KeyState::Pressed => KeyDirection::Down,
                        KeyState::Released => KeyDirection::Up,
                    };
                    state.update_key(kc, direction);

                    // current bind
                    let current_mods = state.serialize_mods(StateComponent::MODS_EFFECTIVE);
                    let current_key = if let Some(key) = state.key_get_one_sym(kc)
                        && let Some(name) = key.name()
                    {
                        name.to_string()
                    } else {
                        continue;
                    };
                    let current_bind = entity::Bind(current_mods, current_key);

                    match map.get(&current_bind) {
                        None => (),
                        Some(shortcuts) => {
                            let mut released_action: Vec<(u64, String)> = Vec::new();

                            for shortcut in shortcuts {
                                match direction {
                                    KeyDirection::Down => {

                                        match shortcut.action {
                                            entity::Action::Push => tx.send(shortcut.exec.clone()).unwrap(),
                                            entity::Action::Hold(ms) => {
                                                let exec = shortcut.exec.clone();
                                                let (hold_tx, hold_rx) = mpsc::channel::<()>();
                                                let main_tx = tx.clone();

                                                std::thread::spawn(move || {
                                                    const STEP: u64 = 10;
                                                    let mut elapsed: u64 = 0;

                                                    loop {
                                                        match hold_rx.recv_timeout(std::time::Duration::from_millis(STEP)) {
                                                            Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                                                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                                                                elapsed += STEP;
                                                                if elapsed >= ms {
                                                                    elapsed = 0;
                                                                    main_tx.send(exec.clone()).unwrap();
                                                                }
                                                            }
                                                        }
                                                    }
                                                });
                                                match active_holds.get_mut(&current_bind.1) {
                                                    None => { active_holds.insert(current_bind.1.clone(), vec![hold_tx]); },
                                                    Some(v) => v.push(hold_tx),

                                                }
                                            }
                                            entity::Action::Released(ms) => released_action.push((ms, shortcut.exec.clone())),
                                        }
                                    }
                                    KeyDirection::Up => {
                                        if let Some(v) = active_holds.remove(&current_bind.1) {
                                            for sender in v {
                                                let _ = sender.send(());
                                            }
                                        }
                                        if let Some(sender) = active_released.remove(&current_bind.1) {
                                            let _ = sender.send(());
                                        }
                                    }
                                }
                            }

                            if !released_action.is_empty() {
                                let main_tx = tx.clone();
                                let (released_tx, released_rx) = mpsc::channel::<()>();

                                std::thread::spawn(move || {
                                    let now = std::time::Instant::now();

                                    while let Ok(_) = released_rx.recv() {
                                        let elapsed = now.elapsed().as_millis() as u64;
                                        released_action.sort_by(|left, right| left.0.cmp(&right.0) );
                                        released_action.reverse();

                                        for (ms, exec) in &released_action {
                                            //println!("ms : {ms}\texec : {exec}");
                                            if elapsed >= *ms {
                                                //println!("success");
                                                let _ = main_tx.send(exec.clone());
                                                break;
                                            }
                                        }
                                    }
                                });
                                active_released.insert(current_bind.1.clone(), released_tx);
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
