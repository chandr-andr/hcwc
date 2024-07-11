use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct PublishMessage {
    message: String,
    send_to: String,
}
