use super::*;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use rand::Rng;

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
struct Data {
    c: char,
    _pad: [u128; 1],
}
impl Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.c.fmt(f)
    }
}
impl Data {
    fn new(c: char) -> Self {
        Self { c, _pad: [0; 1] }
    }
}

#[test]
fn eg_test() {
    let mut storage = ContigStorage::<u128>::new(512);
    let k5 = storage.add(5).unwrap();

    assert_eq!(storage.get(&k5), Some(&5));
    assert_eq!(storage.get(&k5), Some(&5));
    assert_eq!(storage.get(&k5), Some(&5));

    assert_eq!(storage.remove(&k5), Some(5));
    assert_eq!(storage.remove(&k5), None);

    let k9 = storage.add(9).unwrap();
    assert_eq!(storage.get_slice().len(), 1);
    assert_eq!((&storage).into_iter().count(), 1);
    storage.clear();
    assert_eq!(storage.remove(&k9), None);
    assert_eq!(storage.get_slice().len(), 0);
    assert_eq!(0, storage.drain().count());
    let _k1 = storage.add(1).unwrap();
    let _k2 = storage.add(2).unwrap();
    let _k3 = storage.add(3).unwrap();
    storage.remove(&_k1);
    assert_ne!(vec![&1, &2, &3], storage.iter().collect::<Vec<_>>());
    storage.clear();

    let k1 = storage.add(1).unwrap();
    assert_eq!(storage[&k1], 1);
}

#[test]
fn slicing() {
    let mut storage = ContigStorage::new(100);
    let nothing: [u128; 0] = [];
    assert_eq!(&nothing, storage.get_slice());
    let rng = 0u128..100;
    let keys: Vec<_> = rng.clone().map(|x| storage.add(x).unwrap()).collect();
    let expected: Vec<_> = rng.clone().collect();
    assert_eq!(&expected[..], storage.get_slice());
    for (k, v) in keys.into_iter().zip(rng) {
        assert_eq!(storage.remove(&k), Some(v));
        println!("{:?}", (k, v));
    }
}

#[test]
fn use_after_clear() {
    let mut storage = ContigStorage::new(10);
    let ka = storage.add('a').unwrap();
    println!("{:?}", &storage);
    storage.clear();
    // ka is invalid
    let _ka2 = storage.add('b').unwrap();

    println!("{:?}", &storage);
    assert_eq!(storage.get(&ka), None);
}

#[test]
fn correct() {
    const VALUES: usize = 26;
    const MOVES: usize = 5000;

    use rand::SeedableRng;
    let mut rng = rand::rngs::SmallRng::from_seed([4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let mut storage = ContigStorage::new(VALUES);

    let mut unstored: Vec<Data> = (0..VALUES)
        .map(|x| Data::new((x as u8 + 97) as char))
        .collect();
    let mut stored: Vec<Data> = vec![];
    let mut keys: HashMap<Data, Key> = HashMap::new();

    for _i in 0..MOVES {
        let mut did_something = false;
        match rng.gen::<f32>() {
            x if x < 0.5 => {
                unstored.shuffle(&mut rng);
                if let Some(num) = unstored.pop() {
                    println!("ADD, {:?}", num);
                    stored.push(num);
                    keys.insert(num, storage.add(num).unwrap());
                    did_something = true;
                }
            }
            _ => {
                stored.shuffle(&mut rng);
                if let Some(num) = stored.pop() {
                    println!("REM, {:?}", num);
                    let k = keys.remove(&num).unwrap();
                    let val: Data = storage.remove(&k).unwrap();
                    unstored.push(val);
                    if val != num {
                        println!("{:?} != {:?}", val, num);
                        println!("{:?}", &storage);
                        panic!();
                    }
                    did_something = true;
                }
            }
        }
        if did_something {
            println!("{:?}", &storage);
        }
    }
}

#[test]
fn big_test() {
    const VALUES: usize = 1000;
    const MOVES: usize = 50000;

    use rand::SeedableRng;
    let mut rng = rand::rngs::SmallRng::from_seed([4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let mut storage = ContigStorage::new(VALUES);

    let mut unstored: Vec<usize> = (0..VALUES).collect();
    let mut stored: Vec<usize> = vec![];
    let mut keys: HashMap<usize, Key> = HashMap::new();

    for _i in 0..MOVES {
        match rng.gen::<f32>() {
            x if x < 0.5 => {
                unstored.shuffle(&mut rng);
                if let Some(num) = unstored.pop() {
                    stored.push(num);
                    keys.insert(num, storage.add(num).unwrap());
                }
            }
            _ => {
                stored.shuffle(&mut rng);
                if let Some(num) = stored.pop() {
                    let k = keys.remove(&num).unwrap();
                    let val = storage.remove(&k).unwrap();
                    unstored.push(val);
                    if val != num {
                        println!("{:?} != {:?}", val, num);
                        println!("{:?}", &storage);
                        panic!();
                    }
                }
            }
        }
    }
    println!("{:?}", storage);
}

#[test]
fn slice_index_of() {
    //TODO check indices are correct
}

#[test]
fn benching() {
    use std::collections::HashMap;
    use std::time::Instant;
    const SIZE: usize = 10_000;
    const HALFSIZE: usize = SIZE / 2;
    type Data = usize;
    let mut order: Vec<usize> = (0..SIZE).collect();
    order.shuffle(&mut rand::thread_rng());
    ////////////////////////

    let t = Instant::now();
    let mut storage = ContigStorage::<Data>::new(SIZE);
    println!("MY alloc {:?}", t.elapsed());

    let t = Instant::now();
    let mut vec: Vec<Data> = Vec::with_capacity(SIZE);
    println!("vec alloc {:?}", t.elapsed());

    let t = Instant::now();
    let mut hashmap: HashMap<Data, Data> = HashMap::with_capacity(SIZE);
    println!("hashmap alloc {:?}", t.elapsed());
    /////////////////////////

    let t = Instant::now();
    let keys: Vec<_> = (0..SIZE).map(|v| storage.add(v).unwrap()).collect();
    println!("MY push {:?}", t.elapsed());

    let t = Instant::now();
    for v in 0..SIZE {
        vec.push(v);
    }
    println!("vec push {:?}", t.elapsed());

    let t = Instant::now();
    for v in 0..SIZE {
        hashmap.insert(v, v);
    }
    println!("hashmap insert {:?}", t.elapsed());
    ////////////////////////////

    let t = Instant::now();
    for &index in &order[..HALFSIZE] {
        let _v = index as Data;
        let k = &keys[index];
        let _v2 = storage.remove(k).unwrap();
    }
    println!("MY remove {:?}", t.elapsed());

    let t = Instant::now();
    for &index in &order[..HALFSIZE] {
        let v = index as Data;
        let k = &v;
        let _v2 = hashmap.remove(k).unwrap();
    }
    println!("hashmap remove {:?}", t.elapsed());

    let t = Instant::now();
    for &index in &order[HALFSIZE..] {
        let v = index;
        storage.add(v).unwrap();
    }
    println!("MY push (contents dirty) {:?}", t.elapsed());

    let t = Instant::now();
    storage.clear();
    println!("MY clear {:?}", t.elapsed());
}
