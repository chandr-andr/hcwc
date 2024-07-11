use bytes::Bytes;
use futures_util::stream::StreamExt;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientMessage {
    /// Id of the client session
    pub id: usize,
    /// Peer message
    pub msg: String,
    /// Recipient
    pub recipient: usize,
}

async fn retireve_servers(
    mut redis_connection: MultiplexedConnection,
    recipient: usize,
) -> Vec<String> {
    let results = {
        let mut res = redis_connection
            .scan_match::<String, String>(format!("hcwc.user.{}", recipient))
            .await
            .unwrap();

        let mut results: Vec<String> = vec![];
        while let Some(item) = res.next().await {
            results.push(item);
        }

        results
    };

    let servers = redis_connection
        .mget::<Vec<String>, Vec<String>>(results)
        .await
        .unwrap();

    servers
}

#[tokio::main]
async fn main() {
    let nc = async_nats::connect("localhost").await.unwrap();
    let redis = redis::Client::open("redis://127.0.0.1/").unwrap();
    let redis_connection = redis.get_multiplexed_async_connection().await.unwrap();
    let mut qsub = nc
        .queue_subscribe("message.publish", "my_group".to_string())
        .await
        .unwrap();

    loop {
        if let Some(msg) = qsub.next().await {
            let res = serde_json::from_slice::<ClientMessage>(&msg.payload).unwrap();
            let servers = retireve_servers(redis_connection.clone(), res.recipient).await;
            let nc_clone = nc.clone();
            tokio::spawn(async move {
                for server in servers {
                    nc_clone
                        .publish(
                            format!("message.{}.send", server),
                            Bytes::from(serde_json::to_string(&res).unwrap()),
                        )
                        .await
                        .unwrap();
                }
            });
        }
    }
}
