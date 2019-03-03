# contig_storage

```rust
let mut storage = ContigStorage::<u128>::new(512);
let k5 = storage.add(5);
assert_eq!(storage.get(&k5), Some(&5));

```

There's a storage.len() / (std::usize::MAX-1) probability of an invalid key retrieving some valid value.
No input will cause a panic unless the internal state is corrupted.
PANICS:
	compile time: attempt to use get_slice() where the stored type has a size < that of usize.
	runtime: on new() if you request a capacity of std::usize::MAX


TODO:
get the nice iter() construction working
get indexing[&key] working
coercion into slices working
think of a nice opt_in method for repairing indirect keys? consider if worthwhile