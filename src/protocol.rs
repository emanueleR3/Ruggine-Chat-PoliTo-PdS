use serde::{Deserialize, Serialize};
use crate::common::*;

/// Messaggi di protocollo per la comunicazione client-server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolMessage {
    // Autenticazione
    Register { username: String, password: String },
    Login { username: String, password: String },
    
    // Gestione gruppi e messaggi
    CreateGroup { name: String },
    JoinGroup { group_name: String },
    LeaveGroup { group_name: String },
    QuitGroup,
    InviteUser { username: String, group_name: String },
    SendMessage { content: String, group_name: String },
    ListGroups,
    ListUsers,
    ListGroupUsers { group_name: String },
    GoHome,
    
    // Utilità
    Help,
    Quit,

    // Risposte dal server
    AuthResult { success: bool, user_id: Option<UserId>, message: String },
    GroupCreated { group: Group },
    GroupJoined { group: Group, recent_messages: Vec<ChatMessage> },
    GroupLeft,
    GroupQuit,
    UserInvited { username: String },
    MessageReceived { message: Message, recent_messages: Vec<ChatMessage> },
    ReloadMessages { recent_messages: Vec<ChatMessage> },
    GroupListResponse { groups: Vec<Group> },
    UserListResponse { users: Vec<String> },
    Error { message: String },
    Ok { message: String },
    
    // Nuovi messaggi per l'interfaccia a comandi
    Success { message: String },
    UserList { users: Vec<String> },
    MessageList { messages: Vec<ChatMessage> },
    
    // Stati
    StateChanged { in_group: bool, group_name: Option<String> },

    // Heartbeat
    Ping,
    Pong,
}

/// Risultato della serializzazione/deserializzazione dei messaggi
pub type ProtocolResult<T> = Result<T, ProtocolError>;

/// Errori del protocollo
#[derive(Debug, Clone)]
pub enum ProtocolError {
    SerializationError(String),
    DeserializationError(String),
    NetworkError(String),
    InvalidMessage(String),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            ProtocolError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            ProtocolError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ProtocolError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Funzioni di utilità per il protocollo
impl ProtocolMessage {
    /// Serializza il messaggio in JSON
    pub fn to_json(&self) -> ProtocolResult<String> {
        serde_json::to_string(self)
            .map_err(|e| ProtocolError::SerializationError(e.to_string()))
    }

    /// Deserializza il messaggio da JSON
    pub fn from_json(json: &str) -> ProtocolResult<Self> {
        serde_json::from_str(json)
            .map_err(|e| ProtocolError::DeserializationError(e.to_string()))
    }

    /// Aggiunge un delimitatore di fine messaggio
    pub fn to_wire_format(&self) -> ProtocolResult<String> {
        let json = self.to_json()?;
        Ok(format!("{}\n", json))
    }

    /// Parsifica un messaggio dal formato wire (con delimitatore)
    pub fn from_wire_format(data: &str) -> ProtocolResult<Self> {
        let trimmed = data.trim();
        Self::from_json(trimmed)
    }
}
