# Skedgy - Asynchronous Task Scheduler

Skedgy is a lightweight, asynchronous task scheduler for Rust. It allows you to schedule tasks to run at specific times, either using a `DateTime` or a `Duration`. You can also schedule tasks to repeat at specific intervals using cron expressions.

## Features

- ⌚️ Schedule tasks to run at specific times, either using a `DateTime` or a `Duration`.
- ♻️ Schdule tasks using cron expressions to repeat at specific intervals.
- 🚀 Asynchronous execution using `tokio`.
- 📦 De/Serializable state using `serde`.
- 📏 Precision of ~10ms, but this is configurable.
- 🔥 Optimized scheduling algorithm.

## Installation

```bash
cargo add skedgy
```

or add the following to your `Cargo.toml`:

```toml
[dependencies]
skedgy = "0.0.2"
```

## Usage

```rust
use skedgy::{Skedgy, SkedgyHandler, Metadata};
use std::error::Error;

// Define a custom context to store state
#[derive(Debug, Clone)]
struct CustomContext {
    count: AtomicUsize
}

impl CustomContext {
    fn add(&self, amount: usize) {
        self.count.fetch_add(amount, Ordering::Relaxed);
        println!("Count: {}", self.count.load(Ordering::Relaxed));
    }

}

// Define a task to increment the count in the custom context
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    amount: usize
}

// Implement the SkedgyHandler trait for the task
impl SkedgyHandler for Task {
    type Context = CustomContext;
    async fn handle(&self, ctx: Self::Context, _: Metadata) {
        ctx.add(self.amount);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<Error>> {
    // Create a new Skedgy instance with a custom context
    let skedgy = Skedgy::new(
        SkedgyConfig {
            look_ahead_duration: Duration::from_millis(10),
        },
        CustomContext {
            count: AtomicUsize::new(0)
        }
    );

    // Schedule a task to run in 1 second
    let task = SkedgyTask::anonymous()
        .r#in(Duration::from_secs(1))
        .handler(Task { amount: 1 })
        .build()?;
    skegdy.schedule(task).await?;

    // Sleep for 2 seconds to allow the task to run
    tokio::time::sleep(Duration::from_secs(2)).await;
    Ok(())
}
```

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
