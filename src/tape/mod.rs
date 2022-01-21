/*! This module provides the methods needed to create, manipulate and access the tapes
which the turing machine uses as memory.

The types in this module will depend on a compile time factor determining the width of each cell,
the cells being the smallest units of a tape, each cell made up of an arbitrary (the width) number
of u8 characters

Tapes store symbols as u8 values but perform no manipulation on the values themselves.

When a tape is created it will need an u8 value to treat as the empty symbol (the default value
of the tape) and an array of u8 symbols to initialize the tape.
*/

use crate::tape::cache::{Cache, ShiftRet, Side};
use crate::tape::cells::Cell;

mod cache;
mod cells;

#[derive(Debug)]
pub struct Tape<const W: usize> {
    cache: Cache<W>,
    empty: u8,
    focus: Cell<W>,
    left: Cell<W>,
    right: Cell<W>,
}

impl<const W: usize> Tape<W> {
    /// This method creates a tape using the given empty symbol and the given slice to initialize
    /// the value of the tape.
    /// The resulting tape will be positioned so that the first read will return the first symbol in
    /// the slice
    pub fn create(empty: u8, init: &[u8]) -> Self {
        let mut curr: Cell<W> = cells::cells_from_slice(init, empty);

        let mut head = curr.focus();
        let right = curr;

        let left = cells::empty_cell();

        let empty_buff = [empty; W];
        let cache = Cache::new(
            if let Some(x) = head.read() {
                x
            } else {
                empty_buff.clone()
            },
            empty_buff,
        );

        Tape {
            cache,
            empty,
            left,
            right,
            focus: head,
        }
    }

    /// Read the symbol under the cursor
    pub fn read(&self) -> u8 {
        self.cache.read()
    }

    /// Replace the symbol under the cursor with the given one
    /// Return the old symbol
    pub fn write(&mut self, symb: u8) -> u8 {
        self.cache.write(symb)
    }

    /// Move the cursor in the given direction
    pub fn shift(&mut self, direction: Movement) {
        let res = self.cache.shift(direction);
        match res {
            ShiftRet::Stay => {}
            ShiftRet::InCache(dir) => self.tape_shift(dir),
            ShiftRet::OutCacheFail(dir) => {
                match dir {
                    Side::Left => {
                        let buff = if let Some(x) = self.left.read() {
                            x
                        } else {
                            [self.empty; W]
                        };
                        let buff = self.cache.shift_flush(direction, &buff);
                        if let Some(x) = buff {
                            self.right.write(x)
                        }
                    }
                    Side::Right => {
                        let buff = if let Some(x) = self.right.read() {
                            x
                        } else {
                            [self.empty; W]
                        };
                        let buff = self.cache.shift_flush(direction, &buff);
                        if let Some(x) = buff {
                            self.left.write(x)
                        }
                    }
                }
                self.tape_shift(dir)
            }
        }
    }

    fn tape_shift(&mut self, direction: Side) {
        match direction {
            Side::Left => {
                let mut curr_tmp = self.left.focus();
                std::mem::swap(&mut self.focus, &mut curr_tmp);
                self.right.shift(curr_tmp);
            }
            Side::Right => {
                let mut curr_tmp = self.right.focus();
                std::mem::swap(&mut self.focus, &mut curr_tmp);
                self.left.shift(curr_tmp);
            }
        }
    }

    /// Split the tape in `branches` independent copies
    pub fn split(mut self, branches: usize) -> Vec<Self> {
        if let Some(x) = self.cache.flush_current() {
            self.focus.write(x);
        }

        if let (side, Some(x)) = self.cache.flush_other() {
            match side {
                Side::Left => {
                    self.left.write(x);
                }
                Side::Right => {
                    self.right.write(x);
                }
            }
        }

        let Tape {
            cache,
            empty,
            focus,
            left,
            right,
        } = self;

        let mut foc_refs = focus.make_refs(branches);
        let mut left_refs = left.make_refs(branches);
        let mut right_refs = right.make_refs(branches);

        let mut ret = Vec::new();

        for _ in 0..branches {
            let focus = foc_refs.pop().unwrap();
            let right = right_refs.pop().unwrap();
            let left = left_refs.pop().unwrap();

            ret.push(Tape {
                cache: cache.clone(),
                empty,
                focus,
                right,
                left,
            })
        }
        ret
    }
}

#[derive(Copy, Clone)]
pub enum Movement {
    Left,
    Right,
    Stay,
}
