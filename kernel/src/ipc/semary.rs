
pub struct SemArray {
    pub mut key: i32,
    pub mut sems: Vec<Semaphore>,
}

pub struct SemBuf {
    pub mut sem_num: i16,
    pub mut sem_op: i16,
    pub mut sem_flg: i16,
}

pub struct 
lazy_static! {
    pub static ref KEY2SEM: RwLock<BTreeMap<usize, Arc<Mutex<SemArray>>>> =
        RwLock::new(BTreeMap::new());                                                   // not mentioned.
}

fn new_semary(key: i32, nsems: i32, semflg: i32) -> Arc<Mutex<SemArray>> {

    let mut key2sem_table = KEY2SEM.write();
    let mut sem_array_ref: Arc<Mutex<SemArray>>;

    if key2sem_table.get(key) == None {
        let mut semaphores: Vec<Semaphore> = Vec::new();
        for i in 0..nsems {
            semaphores.push(Semaphore::new());
        }

        let mut sem_array = SemArray::new();
        sem_array.key = key;
        sem_array.sems = semaphores;
        sem_array_ref = Arc::new(sem_array);
        key2sem_table.insert(key, sem_array_ref);
    } else {
        sem_array_ref = key2sem_table.get(key).clone();                               // no security check
    }

    sem_array_ref
}