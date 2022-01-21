use crate::rules::{Output, RuleStore, Transition};
use crate::tape::Tape;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::mem::transmute;

pub struct NDTM<'a, const W: usize> {
    rules: &'a RuleStore,
    machines: MachineStore<W>,
    /// The last index used to number a machine
    last_idx: usize,
    pub some_undecided: bool,
    max_step: usize,
}

impl<'a, const W: usize> NDTM<'a, W> {
    pub fn new(tape: Tape<W>, rules: &'a RuleStore, max: usize) -> Self {
        let mut store = MachineStore::new();
        let first = TM::new(tape, 0, Option::None, 0, 0, rules.distance(0));
        store.push(first);
        NDTM {
            rules,
            last_idx: 0,
            machines: store,
            some_undecided: false,
            max_step: max,
        }
    }

    pub fn step(&mut self) -> StepResult {
        let machine = self.machines.pop();
        return if let Some(mut machine) = machine {
            let id = machine.idx;
            if machine.depth >= self.max_step {
                self.some_undecided |= true;
                return StepResult::Undecided { machine: id };
            }
            let step_res = machine.step(self.rules);
            match step_res {
                TMStepRes::Success => {
                    self.machines.push(machine);
                    StepResult::DetStep { machine: id }
                }
                TMStepRes::Failure => StepResult::BranchFail { machine: id },
                TMStepRes::Split(mut trs) => {
                    let (state, depth, mut split) = machine.split(trs.len());
                    let mut ret: Vec<usize> = Vec::with_capacity(trs.len());

                    let dist = self.rules.distance(state);
                    for tape in split {
                        self.last_idx += 1;
                        if let Some(trans) = trs.pop() {
                            let tm = TM::new(tape, state, Some(trans), self.last_idx, depth, dist);
                            ret.push(self.last_idx);
                            self.machines.push(tm)
                        } else {
                            panic!("Not enough rules for the split, impossible")
                        }
                    }
                    if !trs.is_empty() {
                        panic!("There should have been enough tapes")
                    }
                    StepResult::Split {
                        source: id,
                        new: ret,
                    }
                }
                TMStepRes::Recognized => StepResult::Success { machine: id },
            }
        } else {
            StepResult::FailAll
        };
    }

    pub fn fastforward(&mut self, steps: Option<usize>) -> Vec<StepResult> {
        let mut vec = Vec::new();
        let count = 0;
        loop {
            if let Some(m) = steps {
                if count >= m {
                    break;
                }
            }
            let r = self.step();
            match r {
                r @ StepResult::Success { .. } => {
                    vec.push(r);
                    break;
                }
                r @ StepResult::FailAll => {
                    vec.push(r);
                    break;
                }
                r @ _ => {
                    vec.push(r);
                }
            }
        }
        return vec;
    }
}

#[derive(Debug)]
pub enum StepResult {
    Undecided { machine: usize },
    DetStep { machine: usize },
    Split { source: usize, new: Vec<usize> },
    BranchFail { machine: usize },
    Success { machine: usize },
    FailAll,
}

/// The struct used to queue machines for executions
struct MachineStore<const W: usize> {
    heap: BinaryHeap<TM<W>>,
}

impl<const W: usize> MachineStore<W> {
    fn new() -> Self {
        MachineStore {
            heap: BinaryHeap::new(),
        }
    }
    /// Add a machine to the queue
    fn push(&mut self, machine: TM<W>) {
        self.heap.push(machine)
    }

    fn pop(&mut self) -> Option<TM<W>> {
        self.heap.pop()
    }
}

struct TM<const W: usize> {
    depth: usize,
    idx: usize,
    tape: Tape<W>,
    state: usize,
    distance: usize,
    instr_cache: Option<Transition>,
}

impl<const W: usize> TM<W> {
    fn new(
        tape: Tape<W>,
        state: usize,
        rule: Option<Transition>,
        id: usize,
        depth: usize,
        dist: usize,
    ) -> Self {
        TM {
            idx: id,
            tape,
            state,
            instr_cache: rule,
            depth,
            distance: dist,
        }
    }

    fn step(&mut self, rules: &RuleStore) -> TMStepRes {
        if let Some(trs) = self.instr_cache.take() {
            self.depth += 1;
            self.state = trs.state;
            self.distance = rules.distance(self.state);
            self.tape.write(trs.symb);
            self.tape.shift(trs.dir);

            return if rules.is_final(self.state) {
                TMStepRes::Recognized
            } else {
                TMStepRes::Success
            };
        }
        let rule = rules.get(self.state, self.tape.read());
        match rule {
            Output::None => TMStepRes::Failure,
            Output::Simple(trs) => {
                self.instr_cache = Some(trs);
                return self.step(rules);
            }
            Output::Multi(trs) => {
                return TMStepRes::Split(trs);
            }
        }
    }

    /// Split the machine, destroying it and creating a given number of tape
    /// copies. The first element of the return value is the source state of the machine,
    /// the second is the vector of tapes
    fn split(self, num: usize) -> (usize, usize, Vec<Tape<W>>) {
        let TM {
            tape,
            state,
            instr_cache,
            depth,
            ..
        } = self;
        if instr_cache.is_some() {
            panic!("Cache should always be empty if splitting the machine");
        }
        (state, depth, tape.split(num))
    }
}

enum TMStepRes {
    /// Successfully transitioned
    Success,
    /// No transition available
    Failure,
    /// There is a move available but it's non deterministic
    Split(Vec<Transition>),
    /// In a final state
    Recognized,
}

impl<const W: usize> Eq for TM<W> {}

impl<const W: usize> PartialEq<Self> for TM<W> {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl<const W: usize> PartialOrd<Self> for TM<W> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<const W: usize> Ord for TM<W> {
    fn cmp(&self, other: &Self) -> Ordering {
        let min_s = self.distance + self.depth;
        let min_o = other.distance + other.depth;

        let s_some = self.instr_cache.is_some();
        let o_some = other.instr_cache.is_some();

        if s_some && o_some {
            Ordering::Equal
        } else if s_some {
            Ordering::Greater
        } else if o_some {
            Ordering::Less
        } else if min_s == min_o {
            Ordering::Equal
        } else if min_s < min_o {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    }
}
