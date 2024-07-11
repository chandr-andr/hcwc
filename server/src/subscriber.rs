use actix::Addr;
use futures_util::stream::StreamExt;

use crate::{
    chat_server::{ChatServer, ClientMessage},
    responses::{ChatMessageResult, JRPCResponse},
};

pub async fn subscriber(chat_server: Addr<ChatServer>, server_uuid: String) {
    let nc = async_nats::connect("localhost").await.unwrap();
    let mut qsub = nc
        .queue_subscribe(
            format!("message.{}.send", server_uuid),
            "my_group".to_string(),
        )
        .await
        .unwrap();

    loop {
        // Receive a message.
        if let Some(msg) = qsub.next().await {
            let res = serde_json::from_slice::<ClientMessage>(&msg.payload).unwrap();
            chat_server.do_send(JRPCResponse::new(
                Some(0),
                Some(ChatMessageResult {
                    message: res.msg,
                    recipient: res.recipient,
                }),
                None::<()>,
            ));
            println!("{:?}", msg);
            println!("{:?}", chat_server);
        }
    }
}
