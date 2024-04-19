use tokio_tungstenite::connect_async;
use url::Url;

pub async fn send_real_time_update(data: &str) {
    let (ws_stream, _) = connect_async(Url::parse("ws://127.0.0.1:8080").unwrap()).await.expect("Error connecting to WebSocket");
    let (write, _) = ws_stream.split();
    write.send(data.into()).await.expect("Error sending data");
}
