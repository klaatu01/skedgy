# Skedgy - Asynchronous Task Scheduler

Skedgy is a lightweight, asynchronous task scheduler written in Rust. It allows you to schedule tasks to run at specific times, after certain delays, or based on cron expressions. Built on top of `tokio`, Skedgy is designed for efficient, non-blocking execution, making it an excellent choice for Rust applications that require scheduled task management.

## Features

- **Run Tasks at a Specific Time**: Schedule tasks to execute at any `DateTime<Utc>`.
- **Run Tasks After a Delay**: Schedule tasks to run after a specified `Duration`.
- **Cron Scheduling**: Use cron expressions for recurring tasks.
- **Asynchronous Execution**: Built with `tokio` for seamless async integration.
- **Dynamic Task Management**: Add, remove, and manage tasks at runtime.
- **Custom Context and Metadata**: Pass custom context and metadata to tasks for flexible execution.
- **Dependency Injection**: Inject dependencies into tasks for easy access to shared resources.

## Installation

Add Skedgy to your `Cargo.toml`:

```toml
[dependencies]
skedgy = "0.0.3"
```

Or use Cargo to add it directly:

```bash
cargo add skedgy
```

## Usage

### Basic Example

```rust
use skedgy::{task, Skedgy};
use std::error::Error;
use std::time::Duration;

#[task]
async fn delayed_hello() {
    println!("Hello!");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let skedgy = Skedgy::builder().build();

    skedgy
        .duration(Duration::from_secs(5))
        .task(delayed_hello::new())
        .await?;

    // Keep the main task alive to allow the scheduled task to execute
    tokio::time::sleep(Duration::from_secs(6)).await;
    Ok(())
}
```

### Scheduling in a duration

```rust
skedgy
    .duration(Duration::from_secs(5))
    .task(delayed_hello::new())
    .await
    .expect("Failed to schedule task");
```

### Scheduling at a specific time

```rust
use chrono::{DateTime, Utc};

skedgy
    .datetime(chrono::Utc::now() + chrono::Duration::seconds(5))
    .task(delayed_hello::new())
    .await
    .expect("Failed to schedule task");
```

### Scheduling with Cron Expressions

```rust
// Schedule a recurring task using a cron expression
skedgy
    .named("cron_task")
    .cron("0/5 * * * * * *") // Every 5 seconds
    .task(delayed_hello::new())
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

### Adding dependencies to tasks

This can be useful if you want tasks to be able to acces some 'global' state, such as a database connection or a channel to send messages to.
You can only provide on dependency per type, so if you need multiple instances of the same type, I suggest wrapping them in a struct.

```rust
use async_channel::Sender;
use skedgy::{task, Dep, Skedgy};
use std::error::Error;
use std::time::Duration;

#[task]
async fn send_delayed_message(message: String, sender: Dep<Sender<String>>) {
    let _ = sender.inner().send(message.clone()).await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (sender, receiver) = async_channel::unbounded::<String>();
    let skedgy = Skedgy::builder()
        .manage(sender) // Add the sender to the dependency manager
        .build();

    skedgy
        .duration(Duration::from_secs(5))
        .task(send_delayed_message::new("Hello!".to_string()))
        .await?;

    let message = receiver.recv().await.unwrap();
    println!("Received message: {}", message);

    Ok(())
}
```

In this example the `send_delayed_message` task has a dependency on a `Sender<String>`.
The sender is added to the dependency manager when the `Skedgy` instance is created.
The procedural macro expands `send_delayed_message` out to the following code:

```rust
pub(crate) mod send_delayed_message {
    use super::*;
    use skedgy::BoxFuture;
    use skedgy::DependencyStore;
    use skedgy::Task;
    pub fn new(message: String) -> SendDelayedMessage {
        SendDelayedMessage { message: message }
    }
    pub struct SendDelayedMessage {
        message: String,
    }
    impl SendDelayedMessage {
        pub fn new(message: String) -> Self {
            Self { message: message }
        }
        pub async fn execute(&self, sender: impl Into<Dep<Sender<String>>>) -> () {
            let message = &self.message;
            let sender = sender.into();
            {
                let _ = sender.inner().send(message.clone()).await;
            }
        }
    }
    impl Task for SendDelayedMessage {
        fn run(&self, dep_store: &DependencyStore) -> BoxFuture<'_, ()> {
            let sender = dep_store.get::<Sender<String>>().unwrap();
            Box::pin(async move {
                self.execute(sender).await;
            })
        }
    }
}
```

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository.
2. Create a new branch for your feature or bug fix.
3. Write your code and include tests.
4. Submit a pull request.

Please ensure your code adheres to the existing style and includes appropriate documentation.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
