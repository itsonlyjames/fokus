use tokio::{
    sync::{broadcast, mpsc::Sender},
    time::{Duration, sleep},
};

pub async fn countdown(seconds: u64, tx: Sender<u64>, mut running_rx: broadcast::Receiver<bool>) {
    let mut remaining = seconds;
    let mut is_running = true;

    while remaining > 0 {
        if is_running {
            tokio::select! {
                Ok(running) = running_rx.recv() => {
                    is_running = running;
                }
                _ = sleep(Duration::from_secs(1)) => {
                    remaining -= 1;
                    let _ = tx.send(remaining).await;
                }
            }
        } else {
            if let Ok(running) = running_rx.recv().await {
                is_running = running;
            }
        }
    }
    let _ = tx.send(0).await;
}
