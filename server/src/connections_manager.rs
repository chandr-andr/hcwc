use std::collections::HashMap;

use actix::Recipient;

use crate::responses::JRPCResponse;

#[derive(Debug)]
pub struct ConnectionManager {
    connections: HashMap<usize, Recipient<JRPCResponse>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    pub fn add_connection(&mut self, connection_id: usize, connection: Recipient<JRPCResponse>) {
        self.connections.insert(connection_id, connection);
    }

    pub fn remove_connection(&mut self, connection_id: &usize) {
        self.connections.remove(&connection_id);
    }

    pub fn retrieve_connection(&self, connection_id: &usize) -> Option<&Recipient<JRPCResponse>> {
        self.connections.get(&connection_id)
    }
}
