# Purpose

Welcome to my Rust learning log! I'll be compiling my insights and takeaways from my studies on this page.

## Rust Learning Journey

A collection of learning records and sample code exploring Rust, from basic syntax to advanced concepts.

### Table of Contents

* [Environment Setup](https://www.google.com/search?q=%23environment-setup)
* [Learning Roadmap](https://www.google.com/search?q=%23learning-roadmap)
* [Directory Overview](https://www.google.com/search?q=%23directory-overview)
* [Useful Commands](https://www.google.com/search?q=%23useful-commands)
* [References](https://www.google.com/search?q=%23references)

### Environment Setup

To run the code in this repository, you need the Rust toolchain installed.

**Bash**

```
# Install Rust (via rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

### Learning Roadmap

Progress tracker for my learning journey.

* [X] **Basic Syntax** : Variables, Data Types, Functions
* [X] **Ownership System** : Ownership, Borrowing, References
* [X] **Structs** : Data aggregation and implementation (`impl`)
* [X] **Enums** : State management and pattern matching (`match`)
* [X] **Error Handling** : `Option`, `Result`, and the `?` operator
* [ ] **Traits & Generics** : Shared behavior and polymorphic types
* [ ] **Concurrency** : Threads and Message Passing

### Directory Overview

| **Directory** | **Content**              | **Key Learning Points**        |
| ------------------- | ------------------------------ | ------------------------------------ |
| `01_basics`       | Variables, loops, control flow | Immutability vs Mutability (`mut`) |
| `02_ownership`    | Move semantics, borrowing      | Memory safety fundamentals           |
| `03_structs`      | Defining structs and methods   | Data structure design                |
| `04_enums`        | Enums and pattern matching     | Flexible state branching             |
| `exercises`       | Solutions to practice problems | Hands-on coding                      |

### Useful Commands

Cheat sheet for frequently used Cargo commands.

**Bash**

```
# Create a new project
cargo new project_name

# Build and run
cargo run

# Check only (faster than a full build)
cargo check

# Run tests
cargo test

# Generate and open documentation
cargo doc --open
```

## References

Resources I'm using to master Rust.

* [The Rust Programming Language](https://doc.rust-lang.org/book/) - The official bible.
* [Rust by Example](https://doc.rust-lang.org/rust-by-example/) - Learning through code samples.
* [Rustlings](https://github.com/rust-lang/rustlings) - Small exercises to get you used to reading and writing Rust code.
* [Zenn: Rust entrance](https://zenn.dev/mebiusbox/books/22d4c1ed9b0003) - good site of Rust.
