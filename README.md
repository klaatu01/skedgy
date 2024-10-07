# Skedgy - Asynchronous Task Scheduler

Skedgy is a lightweight, asynchronous task scheduler written in Rust. It allows you to schedule tasks to run at specific times, after certain delays, or based on cron expressions. Built on top of `tokio`, Skedgy is designed for efficient, non-blocking execution, making it an excellent choice for Rust applications that require scheduled task management.

## Features

- **Run Tasks at a Specific Time**: Schedule tasks to execute at any `DateTime<Utc>`.
- **Run Tasks After a Delay**: Schedule tasks to run after a specified `Duration`.
- **Cron Scheduling**: Use cron expressions for recurring tasks.
- **Asynchronous Execution**: Built with `tokio` for seamless async integration.
- **Dynamic Task Management**: Add, remove, and manage tasks at runtime.
- **Custom Context and Metadata**: Pass custom context and metadata to tasks for flexible execution.

## Installation

Add Skedgy to your `Cargo.toml`:

```toml
[dependencies]
skedgy = "0.1.0"
```

Or use Cargo to add it directly:

```bash
cargo add skedgy
```

## Usage

### Basic Example

```rust
use skedgy::{Skedgy, SkedgyConfig, SkedgyHandler, Metadata, SkedgyContext};
use chrono::Utc;
use std::time::Duration;
use async_trait::async_trait;
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Clone)]
struct MyContext;

#[derive(Clone)]
struct MyTaskHandler;

#[async_trait]
impl SkedgyHandler for MyTaskHandler {
    type Context = MyContext;

    async fn handle(&self, _ctx: &Self::Context, _metadata: Metadata) {
        println!("Task executed at {:?}", Utc::now());
    }
}

#[tokio::main]
async fn main() {
    let config = SkedgyConfig {
        look_ahead_duration: Duration::from_secs(60),
    };
    let context = MyContext;
    let skedgy = Skedgy::new(config, context);

    let handler = MyTaskHandler;

    // Schedule a task to run in 5 seconds
    skedgy
        .named("my_task")
        .duration(Duration::from_secs(5))
        .task(handler.clone())
        .await
        .expect("Failed to schedule task");

    // Keep the application running
    tokio::signal::ctrl_c().await.unwrap();
}
```

### Scheduling with Cron Expressions

```rust
// Schedule a recurring task using a cron expression
skedgy
    .named("cron_task")
    .cron("0/5 * * * * * *") // Every 5 seconds
    .task(handler.clone())
    .await
    .expect("Failed to schedule cron task");
```

### Removing a Scheduled Task

```rust
// Remove a scheduled task by name
skedgy
    .remove("my_task")
    .await
    .expect("Failed to remove task");
```

### Custom Context and Metadata

You can pass a custom context to your tasks for more flexible execution:

```rust
#[derive(Clone)]
struct MyContext {
    db_connection: Arc<Mutex<DbConnection>>,
}

#[derive(Clone)]
struct MyTaskHandler;

#[async_trait]
impl SkedgyHandler for MyTaskHandler {
    type Context = MyContext;

    async fn handle(&self, ctx: &Self::Context, metadata: Metadata) {
        let db = ctx.db_connection.lock().await;
        // Use the database connection
    }
}
```

## Testing

To run the test suite:

```bash
cargo test
```

The test suite includes tests for:

- Scheduling tasks at specific times.
- Scheduling tasks after delays.
- Scheduling tasks with cron expressions.
- Error handling for invalid cron expressions.
- Adding and removing tasks dynamically.

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository.
2. Create a new branch for your feature or bug fix.
3. Write your code and include tests.
4. Submit a pull request.

Please ensure your code adheres to the existing style and includes appropriate documentation.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
