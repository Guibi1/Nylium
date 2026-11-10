use crate::{Lifecycle, ServerAdapter};
use std::{
    future::Future,
    io::BufRead,
    sync::Arc,
    task::{Context, Poll, Waker},
};

/// Runs an adapter in a console, with an injectable signal future for deterministic tests.
pub async fn run<A, S>(adapter: Arc<A>, signal: S) -> Result<(), String>
where
    A: ServerAdapter,
    S: Future<Output = Result<(), String>> + Send + 'static,
{
    let mut signal = Box::pin(signal);
    // Register the signal handler before startup can modify server data.
    let waker = Waker::noop();
    if let Poll::Ready(result) = signal.as_mut().poll(&mut Context::from_waker(waker)) {
        adapter.stop().await?;
        return result;
    }

    let mut startup = Box::pin(adapter.start());
    let (interrupted, outcome) =
        smol::future::or(async { (false, startup.as_mut().await) }, async {
            (true, signal.as_mut().await)
        })
        .await;
    if interrupted {
        // Keep polling startup while stop requests cancellation and cleanup.
        let (started, stopped) = smol::future::zip(startup, adapter.stop()).await;
        outcome?;
        stopped?;
        started?;
        return Ok(());
    }
    if let Err(error) = outcome {
        adapter.stop().await?;
        return Err(error);
    }

    let commands = adapter.clone();
    std::thread::Builder::new()
        .name("console-input".into())
        .spawn(move || {
            for line in std::io::stdin().lock().lines() {
                match line {
                    Ok(line) if !line.trim().is_empty() => {
                        if let Err(error) = smol::block_on(commands.command(line)) {
                            eprintln!("{error}");
                        }
                    }
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("Console input: {error}");
                        break;
                    }
                }
            }
        })
        .map_err(|error| format!("Cannot start console input: {error}"))?;

    let changes = adapter.subscribe();
    let lifecycle = async {
        loop {
            match &adapter.snapshot().lifecycle {
                Lifecycle::Stopped => return Ok(()),
                Lifecycle::Failed(error) => return Err(error.clone()),
                _ => {}
            }
            changes
                .recv()
                .await
                .map_err(|_| "Server worker stopped".to_string())?;
        }
    };
    let outcome = smol::future::or(signal, lifecycle).await;
    let stopped = adapter.stop().await;
    outcome?;
    stopped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FieldValue, Section, ServerSnapshot, StatePublisher};
    use smol::channel::{Receiver, Sender, bounded};
    use std::sync::atomic::{AtomicBool, Ordering};

    struct SlowAdapter {
        state: StatePublisher,
        started: Sender<()>,
        cancelled: Receiver<()>,
        cancel: Sender<()>,
        stopped: AtomicBool,
        registered: Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl ServerAdapter for SlowAdapter {
        fn snapshot(&self) -> Arc<ServerSnapshot> {
            self.state.snapshot()
        }
        fn subscribe(&self) -> Receiver<()> {
            self.state.subscribe()
        }
        async fn start(&self) -> Result<(), String> {
            assert!(self.registered.load(Ordering::SeqCst));
            self.started.send(()).await.unwrap();
            self.cancelled.recv().await.unwrap();
            Ok(())
        }
        async fn stop(&self) -> Result<(), String> {
            self.stopped.store(true, Ordering::SeqCst);
            let _ = self.cancel.try_send(());
            Ok(())
        }
        async fn command(&self, _: String) -> Result<(), String> {
            unreachable!()
        }
        async fn set_field(&self, _: Section, _: String, _: FieldValue) -> Result<(), String> {
            unreachable!()
        }
    }

    #[test]
    fn signal_is_registered_before_start_and_cancels_slow_startup() {
        smol::block_on(async {
            let (started, starting) = bounded(1);
            let (cancel, cancelled) = bounded(1);
            let registered = Arc::new(AtomicBool::new(false));
            let adapter = Arc::new(SlowAdapter {
                state: StatePublisher::new(Default::default()),
                started,
                cancelled,
                cancel,
                stopped: AtomicBool::new(false),
                registered: registered.clone(),
            });
            let flag = registered.clone();
            let signal = async move {
                flag.store(true, Ordering::SeqCst);
                starting.recv().await.unwrap();
                Ok(())
            };
            smol::future::or(run(adapter.clone(), signal), async {
                smol::Timer::after(std::time::Duration::from_secs(2)).await;
                Err("Startup cancellation timed out".into())
            })
            .await
            .unwrap();
            assert!(registered.load(Ordering::SeqCst));
            assert!(adapter.stopped.load(Ordering::SeqCst));
        });
    }
}
