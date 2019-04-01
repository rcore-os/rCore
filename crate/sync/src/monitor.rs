//! solve the five philosophers problem with monitor

use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

struct Philosopher {
    name: String,
    left: usize,
    right: usize,
}

impl Philosopher {
    fn new(name: &str, left: usize, right: usize) -> Philosopher {
        Philosopher {
            name: name.to_string(),
            left,
            right,
        }
    }

    fn eat(&self, table: &Table) {
        {
            let mut fork_status = table.fork_status.lock().unwrap();
            if fork_status[self.left] {
                fork_status = table.fork_condvar[self.left].wait(fork_status).unwrap()
            }
            fork_status[self.left] = true;
            if fork_status[self.right] {
                fork_status = table.fork_condvar[self.right].wait(fork_status).unwrap()
            }
            fork_status[self.right] = true;
        }
        println!("{} is eating.", self.name);
        thread::sleep(Duration::from_secs(1));
        {
            let mut fork_status = table.fork_status.lock().unwrap();
            fork_status[self.left] = false;
            fork_status[self.right] = false;
            table.fork_condvar[self.left].notify_one();
            table.fork_condvar[self.right].notify_one();
        }
    }

    fn think(&self) {
        println!("{} is thinking.", self.name);
        thread::sleep(Duration::from_secs(1));
    }
}

struct Table {
    fork_status: Mutex<Vec<bool>>,
    fork_condvar: Vec<Condvar>,
}

// the main function to test
pub fn main() {
    let table = Arc::new(Table {
        fork_status: Mutex::new(vec![false; 5]),
        fork_condvar: vec![
            Condvar::new(),
            Condvar::new(),
            Condvar::new(),
            Condvar::new(),
            Condvar::new(),
        ],
    });

    let philosophers = vec![
        Philosopher::new("1", 0, 1),
        Philosopher::new("2", 1, 2),
        Philosopher::new("3", 2, 3),
        Philosopher::new("4", 3, 4),
        Philosopher::new("5", 0, 4),
    ];

    let handles: Vec<_> = philosophers
        .into_iter()
        .map(|p| {
            let table = table.clone();

            thread::spawn(move || {
                for _ in 0..5 {
                    p.think();
                    p.eat(&table);
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
}
