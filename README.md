# contig storage

A `ContigStorage<T>` is a collection of `T` where `T: Copy`. Like [slotmap](https://crates.io/crates/slotmap),
inserting a value returns a `usize` key, which can be used to access the value later. Most importantly, the contents of the storage can always be accessed as a contiguous slice `&[T]` or `&mut [T]`. 

This structure was originally envisitioned with the purpose of buffering transform matrices to be sent to the GPU.

## Example
```rust
let mut storage = ContigStorage::<u128>::new(512);
let k5: usize = storage.add(5);
assert_eq!(storage.get(k5), Some(&5));
storage.clear();
assert_eq!(storage.get(k5), None);
```

## Slice condition

The implementation makes use of an _untagged union_ to store bookkeeping data in-place of an empty buffer. This introduces a requirement: Your data is only stored contiguously _and densely_ if the size of `T` >= the size of `usize`. If this is not the case, you can still use the structure for everything else, but `get_slice()` will *panic*.

## Properties

* Returns `None` if accessed with an invalid key in 1-(N/M) of cases, where N is the number of elements stored, and M is std::usize::MAX. 
* Can be iterated over  

## ABA problem \*resistance\*

It is _resistant_ to the ABA problem by relying on large random hashes (in a way that hardly impacts performance). Each access with an invalid key has a `L/M` probability of erroneously returning data where `L` is the number of elements in the store, and `M` is 2^64. If you want to avoid this problem: don't use the wrong keys :^)


Most importantly, _if the size of `T` is >= that of `usize`_, all the contained data can be accessed as a contiguous slice `&[T]` or `&mut [T]`. 

Much like , 


Structure that is able to store Copy values.
allows addition and removal in constant time.
can be accessed as a contiguous slice.
elements in the slice and in iteration are NOT in order.


at all times, stores data in a contiguous array, allowing access of data as a slice (invariant properties)
does NOT guarantee order.


There's a storage.len() / (std::usize::MAX-1) probability of an invalid key retrieving some valid value.
No input will cause a panic unless the internal state is corrupted.
PANICS:
	compile time: attempt to use get_slice() where the stored type has a size < that of usize.
	runtime: on new() if you request a capacity of std::usize::MAX


TODO:
think of a nice opt_in method for repairing indirect keys? consider if worthwhile
find a nice way to discover the key set? use bits to iterate backwards over the array, collecting elements? should work
	may necessitate bit-marking indirection BEYOND len
write better docs so I can remember WTF is happening later
tests for slice indices being correct