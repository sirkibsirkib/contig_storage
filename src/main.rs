
use std::fmt::Debug;
use std::iter;
use std::fmt;


#[derive(Copy, Clone)]
union Item<T: Copy + Clone> {
	value: T,
	indirection: usize,
}
impl<T> Item<T> where T: Copy {
	const NOTHING: usize = 0;
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


#[derive(Debug)]
struct Key(usize);

#[derive(Debug)]
enum SlotContents {
	Data, Indirection, Nothing,
}

struct ContigStorage<T: Copy> {
	data: Vec<Item<T>>,
	len: usize,
}
impl<T> Debug for ContigStorage<T> where T: Copy + Debug {
	fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
		for i in 0..self.capacity() {
			f.write_fmt(format_args!("{: >4}", i))?
		}
		f.write_str("\n [")?;
		for i in 0..self.capacity() {
			if i > 0 {
				f.write_str(",")?;
			}
			match self.slot_contents(i) {
				SlotContents::Data => unsafe { self.data[i].value.fmt(f)? },
				SlotContents::Indirection => {
					f.write_fmt(format_args!("@{}", unsafe { self.data[i].get_indirection() }))?
				},
				SlotContents::Nothing => f.write_str(" _ ")?,
			}
		}
		f.write_str("]\n")
	}
}
impl<T> ContigStorage<T> where T: Copy {
	fn capacity(&self) -> usize {
		self.data.len()
	}
	fn new(capacity: usize) -> Self {
		Self {
			data: iter::repeat(capacity).take(capacity).map(|_| Item { indirection: 0 } ).collect(),
			len: 0,
		}
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
	fn add(&mut self, value: T) -> Key {
		let boundary = self.len;
		// println!("slot contents {:?}", self.slot_contents(boundary));
		match self.slot_contents(boundary) {
			SlotContents::Nothing => {
				self.data[boundary].value = value;
				self.len += 1;
				// println!("slot contents {:?}", self.slot_contents(boundary));
				Key(boundary)
			},
			SlotContents::Indirection => {
				let real_location = unsafe { self.data[boundary].get_indirection() };
				// make boundary a direct mapping
				self.data[boundary].value = unsafe { self.data[boundary].value };
				// occupy the data previously reached by the indirection
				self.data[real_location].value = value;
				Key(real_location + 1)
			},
			SlotContents::Data => {
				// unreachable?
				panic!("Corrpution! Should NOT have data beyond the boundary!");
			},
		}
	}

	fn fill_hole(&mut self, index: usize) {
		let boundary = self.len;
		if boundary == index {
			// removed the boundary!
			self.data[index].set_nothing();
		} else {
			// removed something left of boundary
			// must move boundary into my slot and put indirection there
			self.data[index] = self.data[boundary];
			self.data[index].set_indirection(index);
		}
	}

	fn remove(&mut self, key: Key) -> T {
		let index = key.0;
		match self.slot_contents(index) {
			SlotContents::Nothing => {
				panic!("Invalid Key! Wrong Storage?");
			},
			SlotContents::Indirection => {
				let real_location = unsafe { self.data[index].get_indirection() };
				self.data[index].set_nothing(); // TODO not sure this is correct?
				// recursive call
				self.remove(Key(real_location))
			},
			SlotContents::Data => {
				let value = unsafe { self.data[index].value };
				self.len -= 1;
				self.fill_hole(index);
				value
			}
		}
	}

	fn access(&mut self, key: &mut Key) -> &mut T {
		let index = key.0;
		match self.slot_contents(index) {
			SlotContents::Nothing => {
				panic!("Invalid Key! Wrong Storage?");
			},
			SlotContents::Indirection => {
				key.0 = unsafe { self.data[index].get_indirection() };
				self.data[index].set_nothing(); // TODO not sure this is correct?
				self.access(key)
			},
			SlotContents::Data => {
				unsafe { &mut self.data[index].value }
			}
		}
	}
}


fn main() {
	let mut storage = ContigStorage::new(6);
	let mut k_a = storage.add('a');
	println!("{:?}", &storage);
	let mut k_b = storage.add('b');
	println!("{:?}", &storage);
}
