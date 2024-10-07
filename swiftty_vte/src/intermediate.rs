/// X3.64 doesn’t place any limit on the number of intermediate characters allowed before a final character,
/// although it doesn’t define any control sequences with more than one.
/// Digital defined escape sequences with two intermediate characters,
/// and control sequences and device control strings with one.
const MAX_INTERMEDIATES: usize = 2;

#[derive(Default)]
pub struct Handler {
    array: [u8; MAX_INTERMEDIATES],
    index: usize,
}

impl Handler {
    pub fn as_slice(&self) -> &[u8] {
        &self.array[..self.index]
    }

    pub fn is_full(&self) -> bool {
        self.index == MAX_INTERMEDIATES
    }

    pub fn push(&mut self, byte: u8) {
        if self.is_full() {
            return;
        }

        self.array[self.index] = byte;
        self.index += 1;
    }

    pub fn clear(&mut self) {
        self.index = 0
    }
}
