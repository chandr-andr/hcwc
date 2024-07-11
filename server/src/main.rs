use actix::{clock::Instant, Actor, Addr};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use redis::Commands;

mod cache;
mod chat_server;
mod chat_session;
mod connections_manager;
mod mq_messages;
mod requests;
mod responses;
mod subscriber;

/// Entry point for our websocket route
async fn chat_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<chat_server::ChatServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        chat_session::ChatSession {
            session_id: 0,
            name: None,
            hb: Instant::now(),
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

fn startup_redis(server_uuid: &str) -> std::io::Result<r2d2::Pool<redis::Client>> {
    let redis_client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let redis_pool = r2d2::Pool::new(redis_client).unwrap();

    redis_pool
        .get()
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "Cannot get redis connection",
            )
        })?
        .sadd::<&str, &str, usize>("hcwc_servers", server_uuid)
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "Cannot add server uuid to available servers in redis",
            )
        })?;

    Ok(redis_pool)
}

fn shutdown_redis(redis_pool: r2d2::Pool<redis::Client>, server_uuid: &str) -> std::io::Result<()> {
    redis_pool
        .get()
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "123123123"))?
        .srem::<&str, &str, usize>("hcwc_servers", server_uuid)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "123123123"))?;
    redis_pool
        .get()
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "123123123"))?
        .del::<&str, usize>(&format!("hcwc.server.{}", server_uuid))
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "123123123"))?;
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let instance_uuid = uuid::Uuid::new_v4().to_string();
    let redis_pool = startup_redis(&instance_uuid)?;
    let nats_client = async_nats::connect("localhost").await.unwrap();
    let server =
        chat_server::ChatServer::new(redis_pool.clone(), nats_client, instance_uuid.clone())
            .start();
    let server_clone = server.clone();
    let uuid_clone = instance_uuid.clone();

    tokio::spawn(async move { subscriber::subscriber(server_clone, uuid_clone).await });

    let res = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(server.clone()))
            .route("/ws/", web::get().to(chat_route))
            .route("/", web::get().to(HttpResponse::Ok))
    })
    .workers(6)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await;

    shutdown_redis(redis_pool, &instance_uuid)?;

    res
}
