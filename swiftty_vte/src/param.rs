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
    pub fn clear(&mut self) {
        self.len = 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push(&mut self, subparam: Subparam) {
        if self.is_full() {
            return;
        }

        self.array[self.len] = subparam;
        self.len += 1;
    }

    pub fn is_full(&self) -> bool {
        self.len == MAX_SUBPARAMS
    }

    fn iter(&self) -> ParamIter<'_> {
        ParamIter::new(self)
    }

    pub fn to_slice(&self) -> &[Subparam] {
        &self.array[..self.len]
    }
}

pub struct ParamIter<'a> {
    param: &'a Param,
    index: usize,
}

impl<'a> ParamIter<'a> {
    fn new(param: &'a Param) -> Self {
        Self { param, index: 0 }
    }
}

impl<'a> Iterator for ParamIter<'a> {
    type Item = Subparam;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.param.len() {
            return None;
        }

        let subparam = self.param.array[self.index];

        self.index += 1;

        Some(subparam)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.param.len() - self.index;

        (remaining, Some(remaining))
    }
}

impl<'a> IntoIterator for &'a Param {
    type Item = Subparam;

    type IntoIter = ParamIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Default, Debug)]
pub struct Params {
    array: [Param; MAX_PARAMS],
    len: usize,
}

impl Params {
    pub fn clear(&mut self) {
        // TODO: optimize it
        for sub in self.array.iter_mut() {
            sub.clear()
        }

        self.len = 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_full(&self) -> bool {
        self.len == MAX_PARAMS
    }

    pub fn next_param(&mut self) {
        if self.is_full() {
            return;
        }

        self.len += 1;
    }

    pub fn push_subparam(&mut self, subparam: Subparam) {
        self.array[self.len].push(subparam);

        //println!("{:?}", self.array[index])
    }

    pub fn iter(&self) -> ParamsIter<'_> {
        ParamsIter::new(self)
    }
}

impl<'a> IntoIterator for &'a Params {
    type Item = &'a Param;

    type IntoIter = ParamsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct ParamsIter<'a> {
    params: &'a Params,
    index: usize,
}

impl<'a> ParamsIter<'a> {
    fn new(params: &'a Params) -> Self {
        Self { params, index: 0 }
    }
}

impl<'a> Iterator for ParamsIter<'a> {
    type Item = &'a Param;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.params.len() {
            return None;
        }

        let param = &self.params.array[self.index];

        self.index += 1;

        Some(param)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.params.len() - self.index;

        (remaining, Some(remaining))
    }
}
