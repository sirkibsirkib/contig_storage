# contig_storage

```rust
let mut storage = ContigStorage::<u128>::new(512);
let k5 = storage.add(5);
assert_eq!(storage.get(&k5), Some(&5));
storage.clear();
assert_eq!(storage.get(&k5), None);
```

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