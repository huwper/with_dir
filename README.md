# with_dir

Blazingly fast utility library for temporarily changing the current working directory.

```rust
use with_dir::WithDir;
use std::path::Path;

let path = Path::new("path/to/directory");

// enter that directory
if let Ok(cwd) = WithDir::new(path) {
    // Current working directory is now path/to/directory
};
// cwd is reset
```

## Contributing

Contributions welcome.

## FAQ

### Is it good?

yes.

## License

See [LICENSE](./LICENSE)

