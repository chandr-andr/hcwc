use std::time::Duration;

use actix::clock::Instant;
use actix::fut;
use actix::ActorContext;
use actix::ActorFutureExt;
use actix::ContextFutureSpawner;
use actix::Running;
use actix::StreamHandler;
use actix::WrapFuture;
use actix::{Actor, Addr};
use actix::{AsyncContext, Handler};
use actix_web_actors::ws;

use crate::chat_server::ClientMessage;
use crate::chat_server::Disconnect;
use crate::chat_server::Join;
use crate::chat_server::{ChatServer, Connect};
use crate::requests::JRPCJoinRequestParams;
use crate::requests::JRPCMessageRequestParams;
use crate::requests::JRPCRequest;
use crate::responses::JRPCResponse;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct ChatSession {
    pub session_id: usize,
    pub name: Option<String>,
    pub addr: Addr<ChatServer>,
    pub hb: Instant,
}

impl ChatSession {
    /// helper method that sends ping to client every 5 seconds (HEARTBEAT_INTERVAL).
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // notify chat server
                act.addr.do_send(Disconnect { id: act.session_id });

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for ChatSession {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    /// We register ws session with ChatServer
    fn started(&mut self, ctx: &mut Self::Context) {
        // we'll start heartbeat process on session start.

        // register self in chat server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        // HttpContext::state() is instance of WsChatSessionState, state is shared
        // across all routes within application
        self.hb(ctx);
        let addr = ctx.address();
        self.addr
            .send(Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.session_id = res,
                    // something is wrong with chat server
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify chat server
        self.addr.do_send(Disconnect {
            id: self.session_id,
        });
        Running::Stop
    }
}

/// Handle messages from chat server, we simply send it to peer websocket
impl Handler<JRPCResponse> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: JRPCResponse, ctx: &mut Self::Context) {
        ctx.text(serde_json::to_value(msg).unwrap().to_string());
    }
}

/// WebSocket message handler
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                let jrpc_request: JRPCRequest = serde_json::from_str(&text).unwrap();
                match jrpc_request.method {
                    "join" => {
                        let join_params = serde_json::from_value::<JRPCJoinRequestParams>(
                            jrpc_request.params.unwrap(),
                        )
                        .unwrap();
                        self.addr.do_send(Join {
                            id: self.session_id,
                            recipient: join_params.recipient,
                        });
                    }
                    "send_message" => {
                        let message_params = serde_json::from_value::<JRPCMessageRequestParams>(
                            jrpc_request.params.unwrap(),
                        )
                        .unwrap();
                        self.addr.do_send(ClientMessage {
                            id: self.session_id,
                            msg: message_params.message,
                            recipient: message_params.recipient,
                        })
                    }
                    _ => {}
                }
            }
            ws::Message::Binary(_) => println!("Unexpected binary"),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}
