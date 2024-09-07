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
use skedgy::{SkedgyHandler, Metadata};


struct MyContext {
    // Add any context data here
}

#[derive(Clone)]
struct MyTask;

impl SkedgyHandler for MyTask {
    async fn handle(&self, context: &MyContext, _: Metadata) {
        println!("Task is running!");
    }
}
```

### 2. Create a Scheduler

Initialize a scheduler with a tick interval:

```rust
use skedgy::{Skedgy, SkedgyConfig};
use std::time::Duration;

let skedgy = Skedgy::new(
    SkedgyConfig {
        look_ahead_duration: Duration::from_millis(50),
    },
    MyContext {}
);
```

### 3. Schedule Tasks

You can schedule tasks to run at specific times, after a delay, or using cron expressions.

```rust
let task =
    SkedgyTask::anonymous()
        .r#in(Duration::from_secs(5)
        .handler(MyTask)
        .build()
        .expect("Failed to build task");

skedgy.schedule(task).await.expect("Failed to schedule task");
```

### 4. Run the Scheduler

The scheduler is automatically run when it is created and will continue to process tasks until the program ends.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
