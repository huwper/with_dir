# with_dir

Blazingly fast utility library for temporarily changing the current working directory.

This library provides the following features:

1. Convenient scoped changing of directories
2. Global Reentrant mutex to prevent concurrent instances of WithDir from conflicting.

The mutex allows this to be safely used across multhreaded tests, where each test 
will be entering different directories as no two WithDir instances can exist on different threads.
However nested instances on the same thread can exist.

```rust
use with_dir::WithDir;
use std::path::Path;

let path = Path::new("src");

// enter that directory
WithDir::new(path).map(|_| {
    // Current working directory is now src
}).unwrap();
// cwd is reset
```

## Contributing

Contributions welcome.

## FAQ

### Is it good?

yes.

## License

See [LICENSE](./LICENSE)

