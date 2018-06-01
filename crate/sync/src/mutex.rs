use std::thread;
use std::sync::{Mutex, Arc};
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
        let _left = table.forks[self.left].lock().unwrap();
        let _right = table.forks[self.right].lock().unwrap();

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

pub fn main() {
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
            for _ in 0..5 {
                p.think();
                p.eat(&table);
            }
        })
    }).collect();

    for h in handles {
        h.join().unwrap();
    }
}