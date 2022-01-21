mod machine;
mod rules;
mod tape;

use crate::machine::{StepResult, NDTM};
use crate::rules::RuleStore;
use crate::tape::{Movement, Tape};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    println!("Ciao");
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("./bin input.txt")
    }
    let filename = &args[1];
    let file = File::open(filename).expect("Can't find");
    let reader = BufReader::new(file);

    let mut rules = RuleStore::new();
    let mut max_steps: usize = 0;

    let mut stage = 0;

    for line in reader.lines() {
        let line = line.unwrap();
        if line.contains("tr") {
            stage = 1;
            continue;
        }
        if line.contains("acc") {
            stage = 2;
            continue;
        }
        if line.contains("max") {
            stage = 3;
            continue;
        }
        if line.contains("run") {
            stage = 4;
            continue;
        }

        match stage {
            1 => {
                let v: Vec<&str> = line.split(' ').collect();
                dbg!(&v);
                rules.add_rule(
                    v[0].parse().unwrap(),
                    v[1].as_bytes()[0],
                    v[2].as_bytes()[0],
                    v[4].parse().unwrap(),
                    match v[3] {
                        "R" => Movement::Right,
                        "L" => Movement::Left,
                        "S" => Movement::Stay,
                        _ => {
                            panic!()
                        }
                    },
                );
            }
            2 => {
                rules.add_final(line.parse().unwrap());
                dbg!(&line);
            }
            3 => {
                max_steps = line.parse().unwrap();
                dbg!(max_steps);
            }
            _ => {
                break;
            }
        }
    }

    //rules.compute_dist();
    let mut input = String::new();
    while true {
        input.clear();
        match std::io::stdin().read_line(&mut input) {
            Ok(s) => {
                println!("{}", s);
                if s == 0 {
                    break;
                }
            }
            Err(_) => {
                break;
            }
        }
        input.pop();
        let slice = input.as_bytes();
        let mut machine: NDTM<5> = NDTM::new(Tape::create(b'_', slice), &rules, max_steps);
        let mut res = machine.fastforward(None);
        dbg!(&res);
        match res.pop() {
            None => {
                panic!()
            }
            Some(x) => match x {
                StepResult::Success { .. } => {
                    println!("Success")
                }
                StepResult::FailAll => {
                    if machine.some_undecided {
                        println!("Undecided")
                    } else {
                        println!("Failure")
                    }
                }
                _ => {
                    panic!()
                }
            },
        }
    }
}
