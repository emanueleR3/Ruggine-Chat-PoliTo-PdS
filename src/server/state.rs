use std::collections::HashMap;
use ruggine::common::*;

/// Stato interno del server
pub struct ServerState {
    pub users: HashMap<UserId, User>,
    pub groups: HashMap<GroupId, Group>,
    pub invites: HashMap<String, GroupInvite>,
    pub messages: Vec<Message>,
    pub active_connections: usize,
    pub message_count: usize,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            groups: HashMap::new(),
            invites: HashMap::new(),
            messages: Vec::new(),
            active_connections: 0,
            message_count: 0,
        }
    }
}
