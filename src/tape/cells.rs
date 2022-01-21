//! This module defines how cells and references to cells behave.

use std::cell::RefCell;
use std::rc::Rc;

struct ConcreteCell<const W: usize> {
    buffer: [u8; W],
}

#[derive(Debug)]
pub enum Cell<const W: usize> {
    Full {
        buffer: Rc<RefCell<[u8; W]>>,
        next: Link<W>,
    },
    Ghost {
        buffer: Rc<RefCell<[u8; W]>>,
        next: Link<W>,
    },
    Empty {
        next: Link<W>,
    },
}

#[derive(Debug)]
pub enum Link<const W: usize> {
    /// No neighbor was created here yet
    Edge,
    /// A link to the same tape
    Same(Rc<RefCell<Cell<W>>>),
    /// A link to a cell in a parent tape, will need to create a ghost when focusing
    Uncle(Rc<RefCell<Cell<W>>>),
    /// No link
    None,
}

impl<const W: usize> Link<W> {
    fn to_uncle(&self) -> Link<W> {
        match self {
            Link::Edge => Link::Edge,
            Link::Same(rc) => Link::Uncle(rc.clone()),
            Link::Uncle(rc) => Link::Uncle(rc.clone()),
            Link::None => Link::None,
        }
    }

    fn focus(self) -> Cell<W> {
        match self {
            Link::Edge => Cell::Empty {
                next: Link::<W>::Edge,
            },
            Link::Same(rc) => {
                let x = Rc::try_unwrap(rc).unwrap_or_else(|x| {
                    panic!("This should've been the only Rc to the cell");
                });
                x.into_inner()
            }
            Link::Uncle(rc) => {
                // Can't take ownership because I'm in a children tape so I don't know how many
                // references are pointing to the cell.
                // I need to match
                let x = match &*rc.borrow() {
                    Cell::Full { buffer, next } => Cell::Ghost {
                        buffer: buffer.clone(),
                        next: next.to_uncle(),
                    },
                    Cell::Ghost { buffer, next } => Cell::Ghost {
                        buffer: buffer.clone(),
                        next: next.to_uncle(),
                    },
                    Cell::Empty { next } => Cell::Empty {
                        next: next.to_uncle(),
                    },
                };
                x
            }
            Link::None => {
                panic!("This shouldn't have been None")
            }
        }
    }
}

impl<const W: usize> Cell<W> {
    /// Replaces the cell with the "next" cells and returns the original
    ///
    pub fn focus(&mut self) -> Cell<W> {
        let next = self.extract_next();
        let mut new = next.focus(); // this new is the next
        std::mem::swap(&mut new, self); // self is now the next and new is the value to return
        new
    }

    /// Replace next with none and return the original
    fn extract_next(&mut self) -> Link<W> {
        let mut nxt = Link::None;
        std::mem::swap(
            &mut nxt,
            match self {
                Cell::Full { next, .. } => next,
                Cell::Ghost { next, .. } => next,
                Cell::Empty { next, .. } => next,
            },
        );

        nxt
    }

    fn set_next(&mut self, next: Link<W>) {
        std::mem::replace(
            match self {
                Cell::Full { next, .. } => next,
                Cell::Ghost { next, .. } => next,
                Cell::Empty { next, .. } => next,
            },
            next,
        );
    }

    /// Opposite of focus, replaces self with the new cell and links the new head with the previous
    /// one
    pub fn shift(&mut self, mut new: Cell<W>) {
        std::mem::swap(self, &mut new);
        // Now self contains what was "new" before
        let next_l = Link::Same(Rc::new(RefCell::new(new)));
        self.set_next(next_l);
    }

    /// Reads the content of the cell
    pub fn read(&self) -> Option<[u8; W]> {
        match self {
            Cell::Full { buffer, .. } => Some(buffer.borrow().clone()),
            Cell::Ghost { buffer, .. } => Some(buffer.borrow().clone()),
            Cell::Empty { .. } => None,
        }
    }

    /// Write to the cell
    pub fn write(&mut self, buff: [u8; W]) {
        match self {
            Cell::Full { buffer, next } => {
                buffer.borrow_mut().copy_from_slice(&buff);
            }
            Cell::Ghost { buffer, next } => {
                let nxt = std::mem::replace(next, Link::None);
                *self = Cell::Full {
                    buffer: Rc::new(RefCell::new(buff)),
                    next: nxt,
                }
            }
            Cell::Empty { next } => {
                let nxt = std::mem::replace(next, Link::None);
                *self = Cell::Full {
                    buffer: Rc::new(RefCell::new(buff)),
                    next: nxt,
                }
            }
        }
    }

    /// Make `num` references to the cell. Takes an owned value so that the original only
    /// survives as references inside the children
    pub fn make_refs(self, num: usize) -> Vec<Cell<W>> {
        let mut v = Vec::new();
        match self {
            Cell::Full { buffer, next } => {
                for _ in 0..num {
                    v.push(Cell::Ghost {
                        buffer: buffer.clone(),
                        next: next.to_uncle(),
                    })
                }
            }
            Cell::Ghost { buffer, next } => {
                for _ in 0..num {
                    v.push(Cell::Ghost {
                        buffer: buffer.clone(),
                        next: next.to_uncle(),
                    })
                }
            }
            Cell::Empty { next } => {
                for _ in 0..num {
                    v.push(Cell::Empty {
                        next: next.to_uncle(),
                    })
                }
            }
        }
        v
    }
}

/// Create a full cell with the given content and given follower
fn full_cell<const W: usize>(buffer: [u8; W], next: Option<Cell<W>>) -> Cell<W> {
    Cell::Full {
        buffer: Rc::new(RefCell::new(buffer)),
        next: if let Some(x) = next {
            Link::Same(Rc::new(RefCell::new(x)))
        } else {
            Link::Edge
        },
    }
}

/// Create an empty cell (supposed to be at the edge of the tape)
pub fn empty_cell<const W: usize>() -> Cell<W> {
    Cell::Empty { next: Link::Edge }
}

/// Creates a chain of non empty cells using the given slice
pub fn cells_from_slice<const W: usize>(buff: &[u8], empty: u8) -> Cell<W> {
    dbg!(buff);
    let mut buff_tmp = [empty; W];
    let (full_cells, last_rem) = (buff.len() / W, buff.len() % W);

    let rem_sl = &buff[(W * full_cells)..];
    dbg!(rem_sl);
    buff_tmp[0..last_rem].copy_from_slice(rem_sl);

    let mut head = full_cell(buff_tmp, None);

    for i in (0..full_cells).rev() {
        dbg!(buff);
        let slice = &buff[(i * W)..((i + 1) * W)];
        dbg!(slice);
        buff_tmp.copy_from_slice(slice);
        head = full_cell(buff_tmp, Some(head));
    }
    head
}
