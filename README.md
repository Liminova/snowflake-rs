# snowflake-rs

> This crate is as completed as my current knowledge allows. It's free of unsafe code for extreme performance tweaks, has zero dependencies, and the compiler doesn't scream at me. Going forward, expect only Rust edition bumps and bug fixes (as if there were any).

## Usage

```bash
cargo add --git https://github.com/Liminova/snowflake-rs
```

```rust
use snowflake_rs::Snowflake;

let datacenter_id = 1;
let worker_id = 1;
let sequence = 0;

let snowflake = Snowflake::new(datacenter_id, worker_id, sequence).build().unwrap();

let id = snowflake.generate_id();
```

## License

This project is licensed under the [MIT license](LICENSE).
