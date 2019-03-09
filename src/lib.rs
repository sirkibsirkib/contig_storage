use bit_vec::BitVec;
use rand::Rng;
use std::fmt::{self, Debug};

#[cfg(test)]
mod tests;

#[derive(Copy, Clone)]
union Item<T: Copy + Clone> {
    value: T,
    indirection: usize,
}
impl<T> Item<T>
where
    T: Copy,
{
    // const NOTHING_MASK: usize = std::usize::MAX ^ (std::usize::MAX >> 1);
    const NOTHING: usize = 0;
    const NOTHING_ITEM: Self = Self {
        indirection: Self::NOTHING,
    };

    unsafe fn get_indirection(&self) -> usize {
        self.indirection.wrapping_sub(1)
    }
    unsafe fn is_nothing(&self) -> bool {
        self.indirection == Self::NOTHING
    }
    fn set_indirection(&mut self, index: usize) {
        self.indirection = index.wrapping_add(1)
    }
    fn set_nothing(&mut self) {
        self.indirection = Self::NOTHING
    }
}


// THIS IS ONLY HERE SO I CAN CHANGE THE API MORE EASILY IF I NEED TO
pub type Key = usize;
trait Keylike {
    fn key_wrap(x: usize) -> Self;
    fn key_unwrap(self) -> Self;
}
impl Keylike for Key {
    fn key_wrap(x: usize) -> Self { x }
    fn key_unwrap(self) -> Self { self }
}


#[derive(Debug)]
enum SlotContents {
    Data,
    Indirection,
    Nothing,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GrowBehavior {
    Doubling,
    None,
}

#[derive(Debug, Copy, Clone)]
pub struct FullError;

pub struct ContigStorage<T: Copy> {
    data: Vec<Item<T>>,
    len: usize,
    start_of_clean: usize,
    indirection_xor: usize,
    indirect_only_bitfield: BitVec,
    pub grow_behavior: GrowBehavior,
}
impl<T> Debug for ContigStorage<T>
where
    T: Copy + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for i in 0..self.capacity() {
            let x = if self.indirect_only_bitfield.get(i).unwrap() {
                "@"
            } else {
                " "
            };
            f.write_fmt(format_args!("{: >3}{}", i, x))?
        }
        f.write_str("\n[")?;
        for i in 0..self.capacity() {
            if i > 0 {
                f.write_str(if i == self.len { "|" } else { "," })?;
            }
            match self.slot_contents(i) {
                SlotContents::Data => unsafe { self.data[i].value.fmt(f)? },
                SlotContents::Indirection => f.write_fmt(format_args!("@{: <2}", unsafe {
                    self.data[i].get_indirection()
                }))?,
                SlotContents::Nothing => f.write_str(" _ ")?,
            }
        }
        f.write_fmt(format_args!(
            "] len: {}, xor: {:X}\n",
            self.len, self.indirection_xor
        ))
    }
}
impl<T> ContigStorage<T>
where
    T: Copy,
{
    fn copy_value(&self, index: usize) -> T {
        // HERE THERE BE UNSAFETY
        unsafe {
            self.data[index].value
            // self.data.get_unchecked(index).value
        }
    }
    fn get_value(&self, index: usize) -> &T {
        // HERE THERE BE UNSAFETY
        unsafe {
            &self.data[index].value
            // &self.data.get_unchecked(index).value
        }
    }
    fn get_mut_value(&mut self, index: usize) -> &mut T {
        // HERE THERE BE UNSAFETY
        unsafe {
            &mut self.data[index].value
            // &mut self.data.get_unchecked_mut(index).value
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn capacity(&self) -> usize {
        self.data.len()
    }
    pub fn new(mut capacity: usize, grow_behavior: GrowBehavior) -> Self {
        if std::mem::size_of::<Item<T>>() > std::mem::size_of::<T>() {
            panic!("Cannot store contiguously! Size of type ({} bytes) < size of usize ({})",
                std::mem::size_of::<T>(),
                std::mem::size_of::<Item<T>>());
        }

        if capacity == std::usize::MAX {
            capacity -= 1;
        }
        Self {
            data: std::iter::repeat(Item::NOTHING_ITEM).take(capacity).collect(),
            len: 0,
            grow_behavior,
            start_of_clean: 0,
            indirection_xor: rand::thread_rng().gen(),
            indirect_only_bitfield: BitVec::from_elem(capacity, false),
        }
    }
    fn slot_contents(&self, index: usize) -> SlotContents {
        if index < self.len {
            SlotContents::Data
        } else if unsafe { self.data[index].is_nothing() } {
            SlotContents::Nothing
        } else {
            SlotContents::Indirection
        }
    }
    pub fn clear(&mut self) {
        for x in self.data[0..self.start_of_clean].iter_mut() {
            *x = Item::<T>::NOTHING_ITEM;
        }
        self.len = 0;
        self.start_of_clean = 0;
        self.indirection_xor = rand::thread_rng().gen();
        self.indirect_only_bitfield.set_all();
        self.indirect_only_bitfield.negate();
    }
    pub fn invalidate_keys(&mut self) {
        self.indirection_xor = rand::thread_rng().gen();
    }
    pub fn assign_new_keys(&mut self) -> impl Iterator<Item=Key> + '_ {
        println!("LEN IS {} SOC IS {}", self.len, self.start_of_clean);
        //TODO test 
        for x in self.data[self.len..self.start_of_clean].iter_mut() {
            *x = Item::<T>::NOTHING_ITEM;
        }
        self.start_of_clean = self.len;
        self.indirection_xor = rand::thread_rng().gen();
        self.indirect_only_bitfield.set_all();
        self.indirect_only_bitfield.negate();

        (0..self.len)
        .map(move |i| Key::key_wrap(i ^ self.indirection_xor))
    }
    pub fn add(&mut self, value: T) -> Result<Key,FullError> {
        // println!("ADD");
        if self.len >= self.capacity() {
            if GrowBehavior::None == self.grow_behavior
            || self.capacity() == std::usize::MAX-1 {
                return Err(FullError);
            } else {
                // grow!
                let start = std::time::Instant::now();
                let new_capacity = self.capacity().saturating_add(2).saturating_mul(2).min(std::usize::MAX-1);
                let mut new_data = Vec::with_capacity(new_capacity);
                unsafe { new_data.set_len(new_capacity); }
                let e = self.len;
                for (src, dest) in self.data[0..e].iter().zip(new_data[0..e].iter_mut()) {
                    *dest = *src;
                }
                for dest in new_data[e..].iter_mut() {
                    *dest = Item::NOTHING_ITEM;
                }
                self.data = new_data;
                self.indirect_only_bitfield.grow(new_capacity, false);
                println!("{:?}", start.elapsed());
                println!("GREW. new capacity is {}", self.capacity());
            }
        }
        let boundary = self.len;
        self.start_of_clean = self.start_of_clean.max(self.len + 1);
        match self.slot_contents(boundary) {
            SlotContents::Nothing => {
                self.data[boundary].value = value;
                self.len += 1;
                Ok(Key::key_wrap(boundary ^ self.indirection_xor))
            }
            SlotContents::Indirection => {
                let real_location = unsafe { self.data[boundary].get_indirection() };
                // make boundary a direct mapping
                self.data[boundary].value = self.copy_value(real_location);
                self.indirect_only_bitfield.set(boundary, false);
                // occupy the data previously reached by the indirection
                self.data[real_location].value = value;
                self.indirect_only_bitfield.set(real_location, false);
                self.len += 1;
                Ok(Key::key_wrap(real_location ^ self.indirection_xor))
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
            self.indirect_only_bitfield.set(index, false);
        } else {
            // boundary now contains a data lement that is LEFT of len
            // must move boundary into my slot and put indirection there
            self.data[index].value = self.copy_value(boundary);
            self.indirect_only_bitfield.set(index, true);
            self.data[boundary].set_indirection(index);
        }
        self.len -= 1;
    }

    pub fn remove(&mut self, key: Key) -> Option<T> {
        let index = key.key_unwrap() ^ self.indirection_xor;
        if index >= self.capacity() {
            return None;
        }
        match self.slot_contents(index) {
            SlotContents::Nothing => None,
            SlotContents::Indirection => {
                let real_location = unsafe { self.data[index].get_indirection() };
                self.data[index].set_nothing();
                // recursive call
                // next layer will think its a direct access. permit it!
                self.indirect_only_bitfield.set(real_location, false);
                self.remove(Key::key_wrap(real_location ^ self.indirection_xor))
            }
            SlotContents::Data => {
                if self.indirect_only_bitfield.get(index).unwrap() {
                    // no direct access allowed >=[
                    return None;
                }
                let value = self.copy_value(index);
                self.fill_hole(index);
                Some(value)
            }
        }
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        let index = key.key_unwrap() ^ self.indirection_xor;
        if index >= self.capacity() {
            return None;
        }
        match self.slot_contents(index) {
            SlotContents::Nothing => None,
            SlotContents::Indirection => {
                let real_location = unsafe { self.data[index].get_indirection() };
                self.get_mut(Key::key_wrap(real_location ^ self.indirection_xor))
            }
            SlotContents::Data => Some(self.get_mut_value(index)),
        }
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        let index = key.key_unwrap() ^ self.indirection_xor;
        if index >= self.capacity() {
            return None;
        }
        match self.slot_contents(index) {
            SlotContents::Nothing => None,
            SlotContents::Indirection => {
                let real_location = unsafe { self.data[index].get_indirection() };
                self.get(Key::key_wrap(real_location ^ self.indirection_xor))
            }
            SlotContents::Data => Some(self.get_value(index)),
        }
    }

    pub fn get_slice(&self) -> &[T] {
        unsafe {
            &*(&self.data[..self.len] as *const [Item<T>] as *const [T])
        }
    }

    pub fn get_slice_index(&self, key: Key) -> Option<usize> {
        let index = key.key_unwrap() ^ self.indirection_xor;
        if index >= self.capacity() {
            return None;
        }
        match self.slot_contents(index) {
            SlotContents::Nothing => None,
            SlotContents::Indirection => {
                let real_location = unsafe { self.data[index].get_indirection() };
                Some(real_location)
            }
            SlotContents::Data => Some(index),
        }
    }

    pub fn drain(&mut self) -> ContigDrain<T> {
        ContigDrain(self, 0)
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data[0..self.len]
            .iter()
            .map(|item| unsafe { &item.value })
    }
}

pub struct ContigDrain<'a, T>(&'a mut ContigStorage<T>, usize)
where
    T: Copy;
impl<'a, T> Iterator for ContigDrain<'a, T>
where
    T: Copy,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.1 == self.0.len {
            self.0.clear();
            None
        } else {
            self.1 += 1;
            Some(self.0.copy_value(self.1 - 1))
        }
    }
}

impl<'a, T> IntoIterator for &'a ContigStorage<T>
where
    T: Copy,
{
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.get_slice().iter()
    }
}

impl<T> std::ops::Index<Key> for ContigStorage<T>
where
    T: Copy,
{
    type Output = T;
    fn index(&self, key: Key) -> &T {
        self.get(key)
            .expect("ContigStorage indexed with invalid key.")
    }
}
impl<T> std::ops::IndexMut<Key> for ContigStorage<T>
where
    T: Copy,
{
    fn index_mut(&mut self, key: Key) -> &mut T {
        self.get_mut(key)
            .expect("ContigStorage indexed with invalid key.")
    }
}
