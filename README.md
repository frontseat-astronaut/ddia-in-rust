# ddia-in-rust
Things implemented so far:
- In-memory DB: Basically a hash index.
- Log DB: Simplest DB that stores key, value pairs in a single file.
- Log DB with Hash index: Same DB as above but with an in-memory index that stores file offset for each key.

All of the above are key-value stores that support set, get and delete.

To build,
```
cargo build
```
To run, 
```
cargo run
```