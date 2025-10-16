use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Identificativo unico per un utente
pub type UserId = String;

/// Identificativo unico per un gruppo
pub type GroupId = String;

/// Struttura che rappresenta un utente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub connected: bool,
    pub joined_at: DateTime<Utc>,
}

impl User {
    pub fn new(username: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            username,
            connected: true,
            joined_at: Utc::now(),
        }
    }
}

/// Struttura che rappresenta un gruppo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
    pub members: Vec<UserId>,
    pub creator_id: UserId,
    pub created_at: String,
}

impl Group {
    pub fn new(name: String, creator_id: UserId) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            members: vec![creator_id.clone()],
            creator_id,
            created_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn add_member(&mut self, user_id: UserId) {
        if !self.members.contains(&user_id) {
            self.members.push(user_id);
        }
    }

    pub fn remove_member(&mut self, user_id: &UserId) {
        self.members.retain(|id| id != user_id);
    }
}

/// Struttura che rappresenta un messaggio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub sender: UserId,
    pub group_id: GroupId,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl Message {
    pub fn new(id: String, sender: UserId, group_id: GroupId, content: String) -> Self {
        Self {
            id, 
            sender,
            group_id,
            content,
            timestamp: Utc::now(),
        }
    }
}

/// Struttura per l'invito a un gruppo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInvite {
    pub id: String,
    pub group_id: GroupId,
    pub invited_user: UserId,
    pub inviter: UserId,
    pub timestamp: DateTime<Utc>,
}

impl GroupInvite {
    pub fn new(group_id: GroupId, invited_user: UserId, inviter: UserId) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            group_id,
            invited_user,
            inviter,
            timestamp: Utc::now(),
        }
    }
}

/// Statistiche delle performance del server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    pub timestamp: DateTime<Utc>,
    pub cpu_time_ms: u64,
    pub active_connections: usize,
    pub total_messages: usize,
    pub total_groups: usize,
    pub total_users: usize,
}

impl ServerStats {
    pub fn new(cpu_time_ms: u64, active_connections: usize, total_messages: usize, total_groups: usize, total_users: usize) -> Self {
        Self {
            timestamp: Utc::now(),
            cpu_time_ms,
            active_connections,
            total_messages,
            total_groups,
            total_users,
        }
    }
}

/// Struttura per i messaggi della chat (per l'interfaccia)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub content: String,
    pub username: String,
    pub timestamp: String,
}
