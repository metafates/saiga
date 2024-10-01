pub const MAX_PARAMS: usize = 32;
pub const SEPARATOR: char = ';';

pub type Param = u16;

#[derive(Default)]
pub struct Params {
    list: [Param; MAX_PARAMS],
    len: usize,
}

impl Params {
    pub fn is_full(&self) -> bool {
        self.len == MAX_PARAMS
    }

    pub fn clear(&mut self) {
        self.len = 0
    }

    pub fn push(&mut self, param: Param) {
        self.list[self.len] = param;
        self.len += 1;
    }
}
