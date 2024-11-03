use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

#[derive(Serialize, Deserialize, Debug, Clone)]struct Message {
    from: String,
    content: String,
    timestamp: u64,
}

type SharedMessageLog = Arc<Mutex<Vec<Message>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let (tx, _) = broadcast::channel(10);
    let message_log: SharedMessageLog = Arc::new(Mutex::new(vec![]));

    loop {
        let (s, a) = listener.accept().await?;
        let tx = tx.clone();
        let rx = tx.subscribe();
        let message_log = Arc::clone(&message_log);

        tokio::spawn(async move {
            if let Err(e) = handle(s, a, tx, rx, message_log).await {
                println!("Error handling connection from {}: {:?}", a, e);
            }
        });
    }
}

async fn handle(
    mut socket: TcpStream,
    addr: std::net::SocketAddr,
    tx: broadcast::Sender<Message>,
    mut rx: broadcast::Receiver<Message>,
    message_log: SharedMessageLog,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![0; 1024];

    loop {
        tokio::select! {
            result = socket.read(&mut buffer) => {
                let bytes_read = result?;
                if bytes_read == 0 {
                    println!("Connection closed by client: {}", addr);
                    break;
                }

                let content = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                let message = Message {
                    from: addr.to_string(),
                    content,
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                };

                {
                    let mut log = message_log.lock().unwrap();
                    log.push(message.clone());
                }
                tx.send(message)?;
            }

            result = rx.recv() => {
                let msg = result?;
                let msg_json = serde_json::to_string(&msg)?;
                socket.write_all(msg_json.as_bytes()).await?;
            }
        }
    }

    Ok(())
}
