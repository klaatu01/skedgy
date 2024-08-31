# Skedgy - Asynchronous Task Scheduler

Skedgy is a lightweight, asynchronous task scheduler written in Rust. It allows you to schedule tasks to run at specific times, after certain delays, or based on cron expressions. Skedgy is built using `tokio` for asynchronous execution and is designed to be efficient and easy to use.

## Features

- **Run tasks at a specific time**: Schedule tasks to run at any `DateTime<Utc>`.
- **Run tasks after a delay**: Schedule tasks to run after a specified `Duration`.
- **Cron scheduling**: Schedule tasks using cron expressions for recurring tasks.
- **Error handling**: Comprehensive error handling to catch and log issues during scheduling or execution.

## Installation

```bash
cargo add skedgy
```

## Usage

### 1. Define Your Task Handler

Implement the `SkedgyHandler` trait for your task. This trait requires the `handle` method, which is an asynchronous function that defines the task's behavior.

```rust
use skedgy::SkedgyHandler;

#[derive(Clone)]
struct MyTask;

impl SkedgyHandler for MyTask {
    async fn handle(&self) {
        println!("Task is running!");
    }
}
```

### 2. Create a Scheduler

Initialize a scheduler with a tick interval:

```rust
use skedgy::{Skedgy, SkedgyConfig};
use std::time::Duration;

let config = SkedgyConfig {
    tick_interval: Duration::from_millis(100),
};
let mut scheduler = Skedgy::new(config).expect("Failed to create scheduler");
```

### 3. Schedule Tasks

You can schedule tasks to run at specific times, after a delay, or using cron expressions.

```rust
use chrono::Utc;

// Schedule a task to run at a specific time
scheduler.run_at(Utc::now() + chrono::Duration::seconds(10), MyTask).await.expect("Failed to schedule task");

// Schedule a task to run after a delay
scheduler.run_in(Duration::from_secs(5), MyTask).await.expect("Failed to schedule task");

// Schedule a task using a cron expression
scheduler.cron("0/1 * * * * * *", MyTask).await.expect("Failed to schedule cron task");
```

### 4. Run the Scheduler

The scheduler is automatically run when it is created and will continue to process tasks until the program ends.

## Running Tests

To run the tests, use the following command:

```sh
cargo test
```

Make sure to include the necessary dependencies in your `Cargo.toml`:

```toml
[dev-dependencies]
tokio = { version = "1", features = ["full", "macros"] }
async-trait = "0.1"
```

The test suite includes tests for scheduling tasks at specific times, after delays, and with cron expressions. It also checks error handling for invalid cron expressions.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
