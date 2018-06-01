//! Dining philosophers problem
//!
//! The code is borrowed from [RustDoc - Dining Philosophers](https://doc.rust-lang.org/1.6.0/book/dining-philosophers.html)

use thread;
use core::time::Duration;
use alloc::{arc::Arc, Vec};
use sync::ThreadLock as Mutex;

struct Philosopher {
    name: &'static str,
    left: usize,
    right: usize,
}

impl Philosopher {
    fn new(name: &'static str, left: usize, right: usize) -> Philosopher {
        Philosopher {
            name,
            left,
            right,
        }
    }

    fn eat(&self, table: &Table) {
        let _left = table.forks[self.left].lock();
        let _right = table.forks[self.right].lock();

        println!("{} is eating.", self.name);
        thread::sleep(Duration::from_secs(1));
    }

    fn think(&self) {
        println!("{} is thinking.", self.name);
        thread::sleep(Duration::from_secs(1));
    }
}

struct Table {
    forks: Vec<Mutex<()>>,
}

pub fn philosopher() {
    let table = Arc::new(Table {
        forks: vec![
            Mutex::new(()),
            Mutex::new(()),
            Mutex::new(()),
            Mutex::new(()),
            Mutex::new(()),
        ]
    });

    let philosophers = vec![
        Philosopher::new("1", 0, 1),
        Philosopher::new("2", 1, 2),
        Philosopher::new("3", 2, 3),
        Philosopher::new("4", 3, 4),
        Philosopher::new("5", 0, 4),
    ];

    let handles: Vec<_> = philosophers.into_iter().map(|p| {
        let table = table.clone();

        thread::spawn(move || {
            for i in 0..5 {
                p.think();
                p.eat(&table);
                println!("{} iter {} end.", p.name, i);
            }
        })
    }).collect();

    for h in handles {
        h.join().unwrap();
    }
    println!("philosophers dining end");
}