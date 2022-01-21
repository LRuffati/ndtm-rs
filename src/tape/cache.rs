/*!
This module defines how the tape cache works

Problem: Cells are necessarily stored in the heap and always behind one layer of indirection for the
tape. If we imagine a machine spending a lot of time moving across cell boundaries (moving back and
forth between two neighboring cells) then we will spend a lot of time following the pointer.

In addition to this the Cells will be mutable behind Rcs and Refcells, inducing read/write overheads.

By keeping a cache we can avoid interacting with the actual cells for as long as we don't move out
of the cell

By making the cache two cells wide we can avoid interaction with a cell (after first access) until
it moves out of the cache window, avoiding multiple pointer accesses in the case of a tape moving
between two cells

Goals, be Cell agnostic, let the Tape deal with writing to the actual cells
*/

use crate::tape::Movement;

#[derive(Copy, Clone, Debug)]
pub struct Cache<const W: usize> {
    buffer_l: [u8; W], // Can't use [u8; W*2]
    buffer_r: [u8; W],
    cursor: usize,
    current: Side,
    dirty: (bool, bool),
}

impl<const W: usize> Cache<W> {
    /// Create a new cache, should only be used when creating a tape from an external array,
    /// the cache can be copied when splitting a Tape (splitting a tape doesn't change the active
    /// side nor the cursor position)
    pub fn new(current: [u8; W], left: [u8; W]) -> Self {
        let mut buffer = [0u8; W];
        buffer[0..W].copy_from_slice(&left);
        let b_l = buffer.clone();
        buffer[0..W].copy_from_slice(&current);
        let b_r = buffer.clone();

        Cache {
            buffer_l: b_l,
            buffer_r: b_r,
            cursor: 0,
            current: Side::Right,
            dirty: (false, false),
        }
    }

    /// Read the symbol at the current position of the active cell
    pub fn read(&self) -> u8 {
        match self.current {
            Side::Left => self.buffer_l[self.cursor],
            Side::Right => self.buffer_r[self.cursor],
        }
    }

    /// Write the symbol to the current position and return the previous symbol
    pub fn write(&mut self, symb: u8) -> u8 {
        match self.current {
            Side::Left => {
                let old = self.buffer_l[self.cursor];
                self.buffer_l[self.cursor] = symb;
                if old != symb {
                    self.dirty.0 = true
                }
                old
            }
            Side::Right => {
                let old = self.buffer_r[self.cursor];
                self.buffer_r[self.cursor] = symb;
                if old != symb {
                    self.dirty.1 = true
                }
                old
            }
        }
    }

    pub fn shift(&mut self, dir: Movement) -> ShiftRet {
        match dir {
            Movement::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    ShiftRet::Stay
                } else if self.current == Side::Right {
                    self.cursor = W - 1;
                    self.current = Side::Left;
                    ShiftRet::InCache(Side::Left)
                } else {
                    ShiftRet::OutCacheFail(Side::Left)
                }
            }
            Movement::Right => {
                if self.cursor < (W - 1) {
                    self.cursor += 1;
                    ShiftRet::Stay
                } else if self.current == Side::Left {
                    self.cursor = 0;
                    self.current = Side::Right;
                    ShiftRet::InCache(Side::Right)
                } else {
                    ShiftRet::OutCacheFail(Side::Right)
                }
            }
            Movement::Stay => ShiftRet::Stay,
        }
    }

    /// Only to be called if `shift` returned OutCacheFail(x)
    ///
    /// If x is Left then new_content should be the contents of the cell to the left of
    /// current in the tape and the return will be the contents of the cell to the right
    ///
    /// If the return value is Option::None then the cell wasn't written to
    pub fn shift_flush(&mut self, dir: Movement, new_content: &[u8; W]) -> Option<[u8; W]> {
        match dir {
            Movement::Left => {
                if !(self.cursor == 0 && self.current == Side::Left) {
                    panic!("Shouldn't have called shift_flush")
                } else {
                    self.current = Side::Left;
                    self.cursor = W - 1;
                    let o_right = self.buffer_r.clone();
                    self.buffer_r = self.buffer_l.clone();
                    self.buffer_l[0..W].copy_from_slice(new_content);
                    if self.dirty.1 {
                        self.dirty.1 = self.dirty.0;
                        self.dirty.0 = false;
                        Some(o_right)
                    } else {
                        self.dirty.1 = self.dirty.0;
                        self.dirty.0 = false;
                        None
                    }
                }
            }
            Movement::Right => {
                if !(self.cursor == W - 1 && self.current == Side::Right) {
                    panic!("Shouldn't have called shift_flush")
                } else {
                    self.current = Side::Right;
                    self.cursor = 0;
                    let o_left = self.buffer_l.clone();
                    self.buffer_l = self.buffer_r.clone();
                    self.buffer_r[0..W].copy_from_slice(new_content);
                    if self.dirty.0 {
                        self.dirty.0 = self.dirty.1;
                        self.dirty.1 = false;
                        Some(o_left)
                    } else {
                        self.dirty.0 = self.dirty.1;
                        self.dirty.1 = false;
                        None
                    }
                }
            }
            Movement::Stay => {
                panic!("Erroneous call to shift_flush");
            }
        }
    }

    /// Returns the contents of the current cell to write to the cell
    /// If the cell hadn't been written to it returns None
    pub fn flush_current(&self) -> Option<[u8; W]> {
        match self.current {
            Side::Left => {
                if self.dirty.0 {
                    Some(self.buffer_l.clone())
                } else {
                    None
                }
            }
            Side::Right => {
                if self.dirty.1 {
                    Some(self.buffer_r.clone())
                } else {
                    None
                }
            }
        }
    }

    /// Returns which cell is in the cache (to the left or the right of current) and the
    /// up to date contents
    pub fn flush_other(&self) -> (Side, Option<[u8; W]>) {
        match self.current {
            Side::Left => (
                Side::Right,
                if self.dirty.1 {
                    Some(self.buffer_r.clone())
                } else {
                    None
                },
            ),
            Side::Right => (
                Side::Left,
                if self.dirty.0 {
                    Some(self.buffer_r.clone())
                } else {
                    None
                },
            ),
        }
    }
}

pub enum ShiftRet {
    /// The focus didn't change from the active cell
    Stay,
    /// The focus moved but within the cache
    InCache(Side),
    /// The focus should've moved but since it moved outside of the
    /// cache I need to call shift_flush providing the contents of the cell to
    /// the given side and expecting the new contents of the opposite side
    OutCacheFail(Side),
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Side {
    Left,
    Right,
}
