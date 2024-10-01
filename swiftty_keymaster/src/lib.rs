pub struct KeyEncoder {}

pub struct KeyEncoderEvent {
    pub action: Action,
    pub modifiers: ModifiersState,
}

pub enum Action {
    Press,
    Release,
    Repeat,
}

pub struct ModifiersState {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

impl KeyEncoder {}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
