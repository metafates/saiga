use crate::executor::Executor;

/// There is no limit to the number of characters in a parameter string,
/// although a maximum of 16 parameters need be stored.
pub const MAX_OSC_PARAMS: usize = 16;

const PARAM_SEPARATOR: u8 = b';';

#[derive(Default)]
pub struct Handler {
    params: [(usize, usize); MAX_OSC_PARAMS],
    params_num: usize,
    raw: Vec<u8>,
}

impl Handler {
    pub fn start(&mut self) {
        self.raw.clear();
        self.params_num = 0;
    }

    pub fn put(&mut self, byte: u8) {
        let idx = self.raw.len();

        if byte != PARAM_SEPARATOR {
            self.raw.push(byte);
            return;
        }

        // handle param separator

        match self.params_num {
            MAX_OSC_PARAMS => return,

            0 => self.params[0] = (0, idx),

            param_idx => {
                let prev = self.params[param_idx - 1];

                self.params[param_idx] = (prev.1, idx)
            }
        }

        self.params_num += 1;
    }

    pub fn end<E: Executor>(&mut self, executor: &mut E, byte: u8) {
        let idx = self.raw.len();

        match self.params_num {
            MAX_OSC_PARAMS => (),

            0 => {
                self.params[0] = (0, idx);
                self.params_num += 1;
            }

            param_idx => {
                let prev = self.params[param_idx - 1];

                self.params[param_idx] = (prev.1, idx);
                self.params_num += 1;
            }
        }

        self.dispatch(executor, byte);
    }

    pub fn dispatch<E: Executor>(&self, executor: &mut E, byte: u8) {
        let slices: Vec<&[u8]> = self
            .params
            .iter()
            .map(|(start, end)| &self.raw[*start..*end])
            .collect();

        let params = &slices[..self.params_num];

        executor.osc_dispatch(params, byte == 0x07)
    }
}
