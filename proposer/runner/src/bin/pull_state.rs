use fastlog_core::base_types::PublicKeyBytes;
use fastlog_core::messages::PullStateClockRequest;
use fastlog_core::serialize::{deserialize_message, serialize_pull_state_request};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use types::configuration::DEFAULT_MAX_DATAGRAM_SIZE;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let interval = if args.len() > 1 {
        args[1].parse::<u64>().ok()
    } else {
        None
    };

    let base_port = 9100;
    let num_shards = 4;
    let addresses: Arc<Vec<SocketAddr>> = Arc::new(
        (0..num_shards)
            .map(|i| format!("127.0.0.1:{}", base_port + i).parse().unwrap())
            .collect(),
    );

    let (tx, mut rx) = mpsc::channel(num_shards);
    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await?);

    tokio::spawn(async move {
        while let Some((index, value)) = rx.recv().await {
            println!("Received result: index={}, value={}", index, value);
        }
    });

    loop {
        for (index, addr) in addresses.iter().enumerate() {
            let tx = tx.clone();
            let socket = socket.clone();
            let addr = addr.clone();
            tokio::spawn(async move {
                let req = PullStateClockRequest {
                    sender: PublicKeyBytes([1; 32]),
                    shard_id: index as u32,
                };
                let buf = serialize_pull_state_request(&req);
                socket.send_to(&buf, &addr).await.expect("send error");

                let mut buf = [0; DEFAULT_MAX_DATAGRAM_SIZE];
                let (_len, _addr) = socket.recv_from(&mut buf).await.expect("recv error");
                let (index, value) = parse_response(&buf);
                tx.send((index, value)).await.expect("msg");
            });
        }
        sleep(Duration::from_millis(interval.unwrap_or_else(|| { 500 }))).await;
    }
}

fn parse_response(buf: &[u8]) -> (u32, u64) {
    let result = deserialize_message(buf).unwrap();
    match result {
        fastlog_core::serialize::SerializedMessage::PullStateResp(resp) => {
            (resp.shard_id, resp.total_counts.into())
        }
        _ => {
            println!("not support");
            (0, 0)
        }
    }
}
