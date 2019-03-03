use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;

#[derive(Copy, Clone)]
union Item<T: Copy + Clone> {
    value: T,
    indirection: usize,
}
impl<T> Item<T>
where
    T: Copy,
{
    const NOTHING: usize = 0;
    const NOTHING_ITEM: Self = Self {
        indirection: Self::NOTHING,
    };

    unsafe fn get_indirection(&self) -> usize {
        self.indirection - 1
    }
    unsafe fn is_nothing(&self) -> bool {
        self.indirection == Self::NOTHING
    }
    fn set_indirection(&mut self, index: usize) {
        self.indirection = index + 1
    }
    fn set_nothing(&mut self) {
        self.indirection = Self::NOTHING
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Key(usize);

#[derive(Debug)]
enum SlotContents {
    Data,
    Indirection,
    Nothing,
}

pub struct ContigStorage<T: Copy + Sized> {
    data: Vec<Item<T>>,
    len: usize,
    largest_dirty: usize,
}
impl<T> Debug for ContigStorage<T>
where
    T: Copy + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for i in 0..self.capacity() {
            f.write_fmt(format_args!("{: >4}", i))?
        }
        f.write_str("\n [")?;
        for i in 0..self.capacity() {
            if i > 0 {
                f.write_str(if i == self.len { "|" } else { "," })?;
            }
            match self.slot_contents(i) {
                SlotContents::Data => unsafe { self.data[i].value.fmt(f)? },
                SlotContents::Indirection => f.write_fmt(format_args!("@{}", unsafe {
                    self.data[i].get_indirection()
                }))?,
                SlotContents::Nothing => f.write_str(" _ ")?,
            }
        }
        f.write_fmt(format_args!("] len: {}\n", self.len))
    }
}
impl<T> ContigStorage<T>
where
    T: Copy + Sized,
{
    #[allow(dead_code)]
    pub const ITER_OK: bool = std::mem::size_of::<T>() >= std::mem::size_of::<usize>();

    pub fn capacity(&self) -> usize {
        self.data.len()
    }
    pub fn new(capacity: usize) -> Self {
        if capacity == std::usize::MAX {
            panic!(format!(
                "ContigStorage can't support a capacity of {:?}",
                std::usize::MAX
            ));
        }
        let data = (0..capacity).map(|_| Item::NOTHING_ITEM).collect();
        Self { data, len: 0 , largest_dirty: 0 }
    }
    fn slot_contents(&self, index: usize) -> SlotContents {
        if index < self.len {
            SlotContents::Data
        } else {
            if unsafe { self.data[index].is_nothing() } {
                SlotContents::Nothing
            } else {
                SlotContents::Indirection
            }
        }
    }
    pub fn clear(&mut self) {
    	for x in self.data[0..self.largest_dirty].iter_mut() {
    		*x = Item::<T>::NOTHING_ITEM; 
    	}
    	self.len = 0;
    	self.largest_dirty = 0;
    }
    pub fn add(&mut self, value: T) -> Option<Key> {
        if self.len == self.capacity() {
            return None;
        }
        let boundary = self.len;
        self.largest_dirty = self.largest_dirty.max(self.len);
        match self.slot_contents(boundary) {
            SlotContents::Nothing => {
                self.data[boundary].value = value;
                self.len += 1;
                Some(Key(boundary))
            }
            SlotContents::Indirection => {
                let real_location = unsafe { self.data[boundary].get_indirection() };
                // make boundary a direct mapping
                self.data[boundary].value = unsafe { self.data[real_location].value };
                // occupy the data previously reached by the indirection
                self.data[real_location].value = value;
                self.len += 1;
                Some(Key(real_location))
            }
            SlotContents::Data => {
                panic!("Corruption! ContigStorage should NOT have data beyond the boundary!");
            }
        }
    }

    // invoked when index < len and index now logically contains Nothing. len is unchanged
    fn fill_hole(&mut self, index: usize) {
        let boundary = self.len - 1;
        if boundary == index {
            // removed the boundary!
            self.data[index].set_nothing();
        } else {
            // boundary now contains a data lement that is LEFT of len
            // must move boundary into my slot and put indirection there
            self.data[index].value = unsafe { self.data[boundary].value };
            self.data[boundary].set_indirection(index);
        }
        self.len -= 1;
    }

    pub fn remove(&mut self, key: Key) -> Option<T> {
        let index = key.0;
        match self.slot_contents(index) {
            SlotContents::Nothing => None,
            SlotContents::Indirection => {
                let real_location = unsafe { self.data[index].get_indirection() };
                self.data[index].set_nothing();
                // recursive call
                self.remove(Key(real_location))
            }
            SlotContents::Data => {
                let value = unsafe { self.data[index].value };
                self.fill_hole(index);
                Some(value)
            }
        }
    }

    pub fn get_mut(&mut self, key: &Key) -> Option<&mut T> {
        let index = key.0;
        match self.slot_contents(index) {
            SlotContents::Nothing => None,
            SlotContents::Indirection => {
                let real_location = unsafe { self.data[index].get_indirection() };
                self.get_mut(&Key(real_location))
            }
            SlotContents::Data => Some(unsafe { &mut self.data[index].value }),
        }
    }

    pub fn get(&mut self, key: &Key) -> Option<&T> {
        let index = key.0;
        match self.slot_contents(index) {
            SlotContents::Nothing => {
                panic!("Invalid Key! Wrong Storage?");
            }
            SlotContents::Indirection => {
                let real_location = unsafe { self.data[index].get_indirection() };
                self.get(&Key(real_location))
            }
            SlotContents::Data => Some(unsafe { &mut self.data[index].value }),
        }
    }

    pub fn get_slice(&self) -> &[T] {
        if !Self::ITER_OK {
            #[allow(dead_code)]
            panic!(
                "Size of your type {} and {}<{}. Values are NOT stored contiguously!",
                std::mem::size_of::<T>(),
                std::mem::size_of::<T>(),
                std::mem::size_of::<usize>()
            );
        }
        unsafe { std::mem::transmute(&self.data[..self.len]) }
    }
}

#[allow(unused_variables)]
fn main() {
    let mut storage = ContigStorage::new(6);
    let ka = storage.add('a').unwrap();
    let kb = storage.add('b').unwrap();
    let kc = storage.add('c').unwrap();
    println!("{:?}", &storage);
    assert_eq!(storage.remove(kb).unwrap(), 'b');
    println!("{:?}", &storage);
    assert_eq!(storage.remove(ka).unwrap(), 'a');
    println!("{:?}", &storage);
}

#[cfg(test)]
mod tests {
    use super::*;
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
    #[allow(deprecated)]
    fn correct() {
        const VALUES: usize = 20;
        const MOVES: usize = 77;

        let mut rng = rand::thread_rng();
        let mut storage = ContigStorage::new(VALUES);

        let mut unstored: Vec<Data> = (0..VALUES)
            .map(|x| Data::new((x as u8 + 97) as char))
            .collect();
        let mut stored: Vec<Data> = vec![];
        let mut keys: HashMap<Data, Key> = HashMap::new();

        for i in 0..MOVES {
            let mut did_something = false;
            match rng.gen::<f32>() {
                x if x < 0.5 => {
                    rng.shuffle(&mut unstored);
                    if let Some(num) = unstored.pop() {
                        println!("ADD, {:?}", num);
                        stored.push(num);
                        keys.insert(num, storage.add(num).unwrap());
                        did_something = true;
                    }
                }
                _ => {
                    rng.shuffle(&mut stored);
                    if let Some(num) = stored.pop() {
                        println!("REM, {:?}", num);
                        let k = keys.remove(&num).unwrap();
                        let val: Data = storage.remove(k).unwrap();
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
}
