/*!
This module provides a way to define the transitions between the states of the turing machine


*/

use crate::tape::Movement;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// The current state of the machine and tape
pub struct Input {
    pub state: usize,
    pub symb: u8,
}

impl Eq for Input {}

impl PartialEq<Self> for Input {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl PartialOrd<Self> for Input {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Input {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.state.cmp(&other.state) {
            Ordering::Equal => self.symb.cmp(&other.symb),
            r @ _ => r,
        }
    }
}

/// The actions to take according to the input
pub enum Output {
    None,
    Simple(Transition),
    Multi(Vec<Transition>),
}

impl Clone for Output {
    fn clone(&self) -> Self {
        match self {
            Output::None => Output::None,
            Output::Simple(trs) => Output::Simple(trs.clone()),
            Output::Multi(vtrs) => Output::Multi(vtrs.clone()),
        }
    }
}

/// How the state of the turing machine and tape will change
#[derive(Copy, Clone)]
pub struct Transition {
    /// The output state
    pub state: usize,
    /// The symbol to overwrite the input symbol with
    pub symb: u8,
    /// The movement of the tape following the write
    pub dir: Movement,
}

pub struct RuleStore {
    rules: BTreeMap<Input, Output>,
    states_backtrace: BTreeMap<usize, Vec<usize>>,
    states_dist: BTreeMap<usize, usize>,
    fin_s: BTreeSet<usize>,
}

impl RuleStore {
    pub fn new() -> Self {
        RuleStore {
            rules: Default::default(),
            states_backtrace: Default::default(),
            states_dist: Default::default(),
            fin_s: Default::default(),
        }
    }

    pub fn add_rule(
        &mut self,
        state_in: usize,
        symb_in: u8,
        symb_out: u8,
        state_out: usize,
        dir: Movement,
    ) {
        let new_t = Transition {
            state: state_out,
            symb: symb_out,
            dir,
        };
        let mut entry = self
            .rules
            .entry(Input {
                state: state_in,
                symb: symb_in,
            })
            .and_modify(|e| match e {
                Output::None => *e = Output::Simple(new_t),
                Output::Simple(tr) => {
                    let mut v = Vec::new();
                    v.push(*tr);
                    v.push(new_t);
                    *e = Output::Multi(v)
                }
                Output::Multi(v) => v.push(new_t),
            })
            .or_insert(Output::Simple(new_t));
        self.states_backtrace
            .entry(state_out)
            .and_modify(|v| v.push(state_in))
            .or_insert(vec![state_in]);
    }

    pub fn get(&self, state_in: usize, symb_in: u8) -> Output {
        self.rules
            .get(&Input {
                state: state_in,
                symb: symb_in,
            })
            .unwrap_or(&Output::None)
            .clone()
    }

    pub fn add_final(&mut self, final_state: usize) {
        self.fin_s.insert(final_state);
    }

    pub fn is_final(&self, state: usize) -> bool {
        self.fin_s.contains(&state)
    }

    pub fn compute_dist(&mut self) {
        let mut queue = &mut self.fin_s;
        let mut next_q = BTreeSet::new();
        let mut tmp_q = BTreeSet::new();
        let mut dist = 0;
        while !queue.is_empty() {
            for &i in queue.iter() {
                self.states_dist.entry(i).or_insert(dist);
                next_q.extend(self.states_backtrace.get(&i).unwrap_or(&vec![]));
            }
            dist += 1;
            tmp_q = next_q;
            queue = &mut tmp_q;
            next_q = BTreeSet::new();
        }
    }

    /// Returns the distance from the state to a final state
    pub fn distance(&self, state: usize) -> usize {
        *self.states_dist.get(&state).unwrap()
    }
}
