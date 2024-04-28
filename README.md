# ddia-in-rust
Things implemented so far:
- Log DB: Simplest DB that stores key, value pairs in a single file.
- Log DB with Hash index: Same DB as above but with an in-memory index that stores file offset for each key.
- Segmented version of the above: This one stores the records over multiple segments, and a background process merges older segments to save disk space.

All of the above are key-value stores that support set, get and delete.

To build,
```
cargo build
```
To run, 
```
cargo run
```