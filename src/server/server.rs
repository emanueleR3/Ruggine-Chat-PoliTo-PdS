use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use ruggine::common::*;
use ruggine::protocol::*;
use ruggine::database::Database;

pub struct ChatServer {
    database: Database,
    connected_users: Arc<Mutex<HashMap<UserId, String>>>, // user_id -> username
}

impl ChatServer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database = Database::new("ruggine.db")?;
        let connected_users = Arc::new(Mutex::new(HashMap::new()));
        
        Ok(Self {
            database,
            connected_users,
        })
    }

    pub fn start(&self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr)?;
        println!("🚀 Server listening on {}", addr);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let connected_users = Arc::clone(&self.connected_users);
                    let database = self.database.clone();
                    
                    thread::spawn(move || {
                        if let Err(e) = handle_client(stream, database, connected_users) {
                            eprintln!("❌ Client error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("❌ Connection failed: {}", e);
                }
            }
        }
        
        Ok(())
    }

    pub fn get_stats(&self) -> Result<ServerStats, Box<dyn std::error::Error>> {
        let connected_count = self.connected_users.lock().unwrap().len();
        let total_users = self.database.get_user_count()?;
        let total_groups = self.database.get_group_count()?;
        let total_messages = self.database.get_message_count()?;
        
        Ok(ServerStats {
            connected_users: connected_count,
            total_users,
            total_groups,
            total_messages,
        })
    }
}

pub struct ServerStats {
    pub connected_users: usize,
    pub total_users: u32,
    pub total_groups: u32,
    pub total_messages: u32,
}

pub fn handle_client(
    mut stream: TcpStream,
    database: Database,
    connected_users: Arc<Mutex<HashMap<UserId, String>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut current_user_id: Option<UserId> = None;
    
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // Connection closed
                if let Some(user_id) = current_user_id {
                    connected_users.lock().unwrap().remove(&user_id);
                }
                break;
            }
            Ok(_) => {
                match ProtocolMessage::from_wire_format(&line) {
                    Ok(message) => {
                        let response = process_message(
                            message,
                            &database,
                            &mut current_user_id,
                            &connected_users,
                        );
                        
                        let response_data = response.to_wire_format()?;
                        stream.write_all(response_data.as_bytes())?;
                        stream.flush()?;
                    }
                    Err(e) => {
                        eprintln!("❌ Error parsing message: {}", e);
                        let error_response = ProtocolMessage::Error {
                            message: "Invalid message format".to_string(),
                        };
                        let response_data = error_response.to_wire_format()?;
                        stream.write_all(response_data.as_bytes())?;
                        stream.flush()?;
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Error reading from client: {}", e);
                break;
            }
        }
    }
    
    // Cleanup on disconnect
    if let Some(user_id) = current_user_id {
        connected_users.lock().unwrap().remove(&user_id);
    }
    
    Ok(())
}

fn process_message(
    message: ProtocolMessage,
    database: &Database,
    current_user_id: &mut Option<UserId>,
    connected_users: &Arc<Mutex<HashMap<UserId, String>>>,
) -> ProtocolMessage {
    match message {
        ProtocolMessage::Register { username, password } => {
            match database.register_user(&username, &password) {
                Ok(user_id) => {
                    *current_user_id = Some(user_id.clone());
                    connected_users.lock().unwrap().insert(user_id.clone(), username.clone());
                    ProtocolMessage::AuthResult {
                        success: true,
                        user_id: Some(user_id),
                        message: format!("Registration successful! Welcome {}!", username),
                    }
                }
                Err(e) => ProtocolMessage::AuthResult {
                    success: false,
                    user_id: None,
                    message: format!("Registration failed: {}", e),
                },
            }
        }

        ProtocolMessage::Login { username, password } => {
            match database.login_user(&username, &password) {
                Ok(user_id) => {
                    *current_user_id = Some(user_id.clone());
                    connected_users.lock().unwrap().insert(user_id.clone(), username.clone());
                    ProtocolMessage::AuthResult {
                        success: true,
                        user_id: Some(user_id),
                        message: format!("Login successful! Welcome back {}!", username),
                    }
                }
                Err(e) => ProtocolMessage::AuthResult {
                    success: false,
                    user_id: None,
                    message: format!("Login failed: {}", e),
                },
            }
        }

        ProtocolMessage::ListGroups => {
            if let Some(user_id) = current_user_id {
                match database.get_user_groups(user_id) {
                    Ok(groups) => ProtocolMessage::GroupListResponse { groups },
                    Err(e) => ProtocolMessage::Error {
                        message: format!("Failed to get groups: {}", e),
                    },
                }
            } else {
                ProtocolMessage::Error {
                    message: "Not authenticated".to_string(),
                }
            }
        }

        ProtocolMessage::CreateGroup { name } => {
            if let Some(user_id) = current_user_id {
                match database.create_group(&name, user_id) {
                    Ok(_) => ProtocolMessage::Ok {
                        message: format!("Group '{}' created successfully!", name),
                    },
                    Err(e) => ProtocolMessage::Error {
                        message: format!("Failed to create group: {}", e),
                    },
                }
            } else {
                ProtocolMessage::Error {
                    message: "Not authenticated".to_string(),
                }
            }
        }

        ProtocolMessage::JoinGroup { group_name } => {
            if let Some(user_id) = current_user_id {
                match database.join_group(&group_name, user_id) {
                    Ok(_) => ProtocolMessage::Ok {
                        message: format!("Joined group '{}' successfully!", group_name),
                    },
                    Err(e) => ProtocolMessage::Error {
                        message: format!("Failed to join group: {}", e),
                    },
                }
            } else {
                ProtocolMessage::Error {
                    message: "Not authenticated".to_string(),
                }
            }
        }

        ProtocolMessage::InviteUser { username } => {
            if let Some(user_id) = current_user_id {
                // Per ora usiamo un gruppo di default - dovremmo passare il gruppo corrente
                match database.invite_user("default", &username, user_id) {
                    Ok(_) => ProtocolMessage::Ok {
                        message: format!("Invited {} to group!", username),
                    },
                    Err(e) => ProtocolMessage::Error {
                        message: format!("Failed to invite user: {}", e),
                    },
                }
            } else {
                ProtocolMessage::Error {
                    message: "Not authenticated".to_string(),
                }
            }
        }

        ProtocolMessage::SendMessage { content } => {
            if let Some(user_id) = current_user_id {
                match database.send_message("default", user_id, &content) {
                    Ok(_) => ProtocolMessage::Ok {
                        message: "Message sent!".to_string(),
                    },
                    Err(e) => ProtocolMessage::Error {
                        message: format!("Failed to send message: {}", e),
                    },
                }
            } else {
                ProtocolMessage::Error {
                    message: "Not authenticated".to_string(),
                }
            }
        }

        ProtocolMessage::ListUsers => {
            match database.get_all_users() {
                Ok(users) => ProtocolMessage::UserListResponse { users },
                Err(e) => ProtocolMessage::Error {
                    message: format!("Failed to get users: {}", e),
                },
            }
        }

        ProtocolMessage::Quit => {
            if let Some(user_id) = current_user_id {
                connected_users.lock().unwrap().remove(user_id);
                *current_user_id = None;
            }
            ProtocolMessage::Ok {
                message: "Goodbye!".to_string(),
            }
        }

        _ => ProtocolMessage::Error {
            message: "Unsupported message type".to_string(),
        },
    }
}
