use xkbcommon_rs::{xkb_state::{KeyDirection, StateComponent}, *};

pub struct Wrapper {
    state: State,
}

impl Wrapper {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let keymap = Keymap::new_from_names(Context::new(0).unwrap(), None, 0)?;
        let state = State::new(keymap);
        Ok(Self { state })
    }
    pub fn modify(&mut self, kc: u32, direction: KeyDirection) -> StateComponent {
        self.state.update_key(kc, direction)
    }
}