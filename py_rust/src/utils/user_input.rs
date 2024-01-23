use tokio::{signal, sync::mpsc};

use crate::prelude::*;

struct InteractiveStdin {
    chan: mpsc::Receiver<std::io::Result<String>>,
}

impl InteractiveStdin {
    fn new() -> Self {
        let (send, recv) = mpsc::channel(16);
        std::thread::spawn(move || {
            for line in std::io::stdin().lines() {
                if send.blocking_send(line).is_err() {
                    return;
                }
            }
        });
        InteractiveStdin { chan: recv }
    }

    /// Get the next line from stdin.
    ///
    /// Returns `Ok(None)` if stdin has been closed.
    ///
    /// This method is cancel safe.
    async fn next_line(&mut self) -> std::io::Result<Option<String>> {
        self.chan.recv().await.transpose()
    }
}

pub async fn confirm(prompt: &str) -> Result<bool, Zerr> {
    loop {
        println!("\n{} Enter 'y' to confirm, 'n' to decline.", prompt);

        let shutdown = signal::ctrl_c();
        let mut stdin = InteractiveStdin::new();

        tokio::select! {
            _ = shutdown => {
                break Ok(false);
            }

            res = stdin.next_line() => {
                if let Some(line) = res.change_context(Zerr::InternalError)? {
                    match line.as_str() {
                        "y" | "Y" | "yes" | "Yes" | "YES" => break Ok::<_, error_stack::Report<Zerr>>(true),
                        "n" | "N" | "no" | "No" | "NO" => break Ok(false),
                        _ => {
                            continue;
                        }
                    }
                } else {
                    break Ok(false);
                }
            }
        }
    }
}

pub fn sync_confirm(prompt: &str) -> Result<bool, Zerr> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .change_context(Zerr::InternalError)?
        .block_on(async { confirm(prompt).await.change_context(Zerr::InternalError) })
}
