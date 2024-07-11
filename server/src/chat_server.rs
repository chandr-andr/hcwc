use serde::{Deserialize, Serialize};

use actix::{Actor, Context, Handler, Message, Recipient};
use bytes::Bytes;
use rand::{rngs::ThreadRng, Rng};
use redis::Commands;

use crate::{
    connections_manager::ConnectionManager,
    responses::{ChatMessageResult, ConnectResult, JRPCResponse, JoinError, JoinResult},
};

#[derive(Debug)]
pub struct ChatServer {
    connection_manager: ConnectionManager,
    rng: ThreadRng,
    redis_pool: r2d2::Pool<redis::Client>,
    nats_conn: async_nats::Client,
    chat_uuid: String,
}

impl ChatServer {
    pub fn new(
        redis_conn: r2d2::Pool<redis::Client>,
        nats_conn: async_nats::Client,
        chat_uuid: String,
    ) -> ChatServer {
        ChatServer {
            connection_manager: ConnectionManager::new(),
            rng: rand::thread_rng(),
            redis_pool: redis_conn,
            chat_uuid: chat_uuid,
            nats_conn: nats_conn,
        }
    }
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

/// New chat session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<JRPCResponse>,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

/// Join room, if room does not exists create new one.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    /// Client ID
    pub id: usize,

    /// recipient name
    pub recipient: usize,
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    /// Id of the client session
    pub id: usize,
    /// Peer message
    pub msg: String,
    /// Recipient
    pub recipient: usize,
}

impl Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let id = self.rng.gen::<usize>();

        self.redis_pool
            .get()
            .unwrap()
            .set::<&str, &str, String>(format!("hcwc.user.{}", id).as_str(), &self.chat_uuid)
            .unwrap();

        self.connection_manager.add_connection(id, msg.addr.clone());

        msg.addr.do_send(JRPCResponse::new(
            None,
            Some(ConnectResult { id: id }),
            None::<()>,
        ));

        id
    }
}

impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        let redis_conn = self.redis_pool.get();

        if let Ok(mut redis_conn) = redis_conn {
            let deleted_connection: Result<usize, redis::RedisError> =
                redis_conn.srem::<&str, &usize, usize>("connections", &msg.id);
            match deleted_connection {
                Err(_) => {
                    println!("Problem with redis");
                }
                Ok(deleted_connection) => {
                    if deleted_connection == 0 {
                        println!("Cannot find user in redis")
                    } else {
                        println!("User deleted")
                    }
                }
            }
        }

        self.connection_manager.remove_connection(&msg.id);
        if let Some(recipient) = self.connection_manager.retrieve_connection(&msg.id) {
            recipient.do_send(JRPCResponse::new(None, None::<()>, None::<()>));
        }
    }
}

impl Handler<JRPCResponse> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: JRPCResponse, _: &mut Context<Self>) {
        let jrpc_result =
            serde_json::from_value::<ChatMessageResult>(msg.result.clone().unwrap()).unwrap();
        if let Some(addr) = self
            .connection_manager
            .retrieve_connection(&jrpc_result.recipient)
        {
            addr.do_send(msg);
        }
    }
}

impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        let nats_conn_copy = self.nats_conn.clone();
        tokio::spawn(async move {
            nats_conn_copy
                .publish(
                    "message.publish",
                    Bytes::from(serde_json::to_string(&msg).unwrap()),
                )
                .await
                .unwrap();
        });
    }
}

impl Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        let Join { id, recipient } = msg;
        let mut error_join: bool = false;
        let mut error_message: &str = "";
        let redis_conn = self.redis_pool.get();

        if let Ok(mut redis_conn) = redis_conn {
            let is_recipient_exist: Result<bool, redis::RedisError> =
                redis_conn.exists::<&str, bool>(format!("hcwc.user.{}", recipient).as_str());

            if is_recipient_exist
                .as_ref()
                .is_ok_and(|is_exist| is_exist == &false)
                || is_recipient_exist.is_err()
            {
                error_join = true;
                error_message = "Recipient doesn't exist"
            }
        } else {
            error_join = true;
            error_message = "Cannot connect to the redis"
        }

        if let Some(send_to_user) = self.connection_manager.retrieve_connection(&id) {
            if error_join {
                send_to_user.do_send(JRPCResponse::new(
                    None,
                    None::<()>,
                    Some(JoinError {
                        error_message: error_message,
                    }),
                ))
            } else {
                send_to_user.do_send(JRPCResponse::new(
                    None,
                    Some(JoinResult {
                        joined_user: recipient,
                    }),
                    None::<()>,
                ))
            }
        }
    }
}
