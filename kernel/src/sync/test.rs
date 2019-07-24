//! Dining philosophers problem
//!
//! The code is borrowed from [RustDoc - Dining Philosophers](https://doc.rust-lang.org/1.6.0/book/dining-philosophers.html)

use crate::sync::Condvar;
use crate::sync::SleepLock as Mutex;
use crate::thread;
use alloc::vec;
use alloc::{sync::Arc, vec::Vec};
use core::time::Duration;
use log::*;

struct Philosopher {
    name: &'static str,
    left: usize,
    right: usize,
}

impl Philosopher {
    fn new(name: &'static str, left: usize, right: usize) -> Philosopher {
        Philosopher { name, left, right }
    }

    fn eat(&self, table: &Arc<dyn Table>) {
        table.eat(self.name, self.left, self.right);
    }

    fn think(&self) {
        println!("{} is thinking.", self.name);
        thread::sleep(Duration::from_secs(1));
    }
}

trait Table: Send + Sync {
    fn eat(&self, name: &str, left: usize, right: usize);
}

struct MutexTable {
    forks: Vec<Mutex<()>>,
}

impl Table for MutexTable {
    fn eat(&self, name: &str, left: usize, right: usize) {
        let _left = self.forks[left].lock();
        let _right = self.forks[right].lock();

        println!("{} is eating.", name);
        thread::sleep(Duration::from_secs(1));
    }
}

struct MonitorTable {
    fork_status: Mutex<Vec<bool>>,
    fork_condvar: Vec<Condvar>,
}

impl Table for MonitorTable {
    fn eat(&self, name: &str, left: usize, right: usize) {
        {
            let mut fork_status = self.fork_status.lock();
            if fork_status[left] {
                fork_status = self.fork_condvar[left].wait(fork_status);
            }
            fork_status[left] = true;
            if fork_status[right] {
                fork_status = self.fork_condvar[right].wait(fork_status);
            }
            fork_status[right] = true;
        }
        println!("{} is eating.", name);
        thread::sleep(Duration::from_secs(1));
        {
            let mut fork_status = self.fork_status.lock();
            fork_status[left] = false;
            fork_status[right] = false;
            self.fork_condvar[left].notify_one();
            self.fork_condvar[right].notify_one();
        }
    }
}

fn philosopher(table: Arc<dyn Table>) {
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
            trace!("philosopher start");

            thread::spawn(move || {
                for i in 0..5 {
                    p.think();
                    p.eat(&table);
                    println!("{} iter {} end.", p.name, i);
                }
            })
        })
        .collect();
    trace!("philosopher starting finish");

    for h in handles {
        h.join().expect("handle should not be none");
    }
    println!("philosophers dining end");
}

pub fn philosopher_using_mutex() {
    println!("philosophers using mutex");

    let table = Arc::new(MutexTable {
        forks: vec![
            Mutex::new(()),
            Mutex::new(()),
            Mutex::new(()),
            Mutex::new(()),
            Mutex::new(()),
        ],
    });
    philosopher(table);
}

pub fn philosopher_using_monitor() {
    println!("philosophers using monitor");

    let table = Arc::new(MonitorTable {
        fork_status: Mutex::new(vec![false; 5]),
        fork_condvar: vec![
            Condvar::new(),
            Condvar::new(),
            Condvar::new(),
            Condvar::new(),
            Condvar::new(),
        ],
    });
    philosopher(table);
}
