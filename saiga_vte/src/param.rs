use std::ops::Index;

pub const MAX_PARAMS: usize = 16;
pub const MAX_SUBPARAMS: usize = MAX_PARAMS * 2;
pub const PARAM_SEPARATOR: u8 = b';';
pub const SUBPARAM_SEPARATOR: u8 = b':';

pub type Subparam = u16;

#[derive(Default, Debug)]
pub struct Param {
    array: [Subparam; MAX_SUBPARAMS],
    len: usize,
}

impl From<Subparam> for Param {
    fn from(subparam: Subparam) -> Self {
        let mut param = Param::default();

        param.push(subparam);

        param
    }
}

impl Param {
    pub const fn clear(&mut self) {
        self.len = 0
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn push(&mut self, subparam: Subparam) {
        if self.is_full() {
            return;
        }

        self.array[self.len] = subparam;
        self.len += 1;
    }

    pub const fn is_full(&self) -> bool {
        self.len == MAX_SUBPARAMS
    }

    pub fn as_slice(&self) -> &[Subparam] {
        &self.array[..self.len]
    }
}

impl Index<usize> for Param {
    type Output = Subparam;

    fn index(&self, index: usize) -> &Self::Output {
        &self.array[index]
    }
}

#[derive(Default, Debug)]
pub struct Params {
    array: [Param; MAX_PARAMS],
    len: usize,
}

impl Params {
    pub fn clear(&mut self) {
        for sub in &mut self.array {
            sub.clear()
        }

        self.len = 0
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub fn as_slice(&self) -> &[Param] {
        &self.array[..self.len]
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn is_full(&self) -> bool {
        self.len == MAX_PARAMS
    }

    pub const fn next_param(&mut self) {
        if self.is_full() {
            return;
        }

        self.len += 1;
    }

    pub fn push_subparam(&mut self, subparam: Subparam) {
        self.array[self.len].push(subparam);
    }
}

impl Index<usize> for Params {
    type Output = Param;

    fn index(&self, index: usize) -> &Self::Output {
        &self.array[index]
    }
}
