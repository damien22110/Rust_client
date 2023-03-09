use std::env;

use futures_util::{future, pin_mut, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tungstenite::protocol::WebSocketConfig;
use serde_json::json;
use tokio_tungstenite::{connect_async_with_config,tungstenite::client::IntoClientRequest, tungstenite::protocol::Message};


#[tokio::main]
async fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    // On récupère l'adresse du serveur à laquelle se connecter

    let ws_config = WebSocketConfig {
        // Set tous les autres paramètres à leur valeur par défaut
        ..WebSocketConfig::default()
    };
    // On crée une requête HTTP pour se connecter au serveur
    let request = "wss://sbofpngzjueqtbhbuwig.supabase.co/realtime/v1?apikey=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InNib2Zwbmd6anVlcXRiaGJ1d2lnIiwicm9sZSI6ImFub24iLCJpYXQiOjE2NzQxMzI1OTAsImV4cCI6MTk4OTcwODU5MH0.lUDgT0rPYWi8HGt9wPpgZk92Yk6NvBkDAKRNPBDvG4k&log_level=info&vsn=1.0.0".to_string().into_client_request().unwrap();
    
    // On crée un channel pour envoyer les messages du stdin vers le serveur
    // et un autre pour recevoir les messages du serveur vers le stdout
    let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();

    // On lance un thread qui va lire le stdin et envoyer les messages
    tokio::spawn(read_stdin(stdin_tx));
    // On se connecte au serveur
    let (ws_stream, _) = connect_async_with_config(request, Some(ws_config)).await.expect("Failed to connect");
    // On affiche un message de confirmation
    println!("WebSocket handshake has been successfully completed");

    // On récupère les deux parties du stream (read et write)
    let (write, read) = ws_stream.split();
    let stdin_to_ws = stdin_rx.map(Ok).forward(write);

    // On créer un handler pour les messages reçus du serveur
    let ws_to_stdout = read.for_each(|msg| async move {
        println!("{:?}",msg);
    });
    // On attend que l'un des deux streams se termine
    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;
}

// On lit le stdin et on envoie les messages sur le channel
async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
    let mut stdin = tokio::io::stdin();
    // on créer la structure de données à envoyer
    let data = json!({
        "topic": "realtime:public",
        "event": "phx_join",
        "payload": {},
        "ref": "1",
    });
    // on la transforme en string
    let data = data.to_string();
    // on l'envoi au serveur
    tx.unbounded_send(Message::text(data)).unwrap();

    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx.unbounded_send(Message::binary(buf)).unwrap();
    }
}