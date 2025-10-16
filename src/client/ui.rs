use ruggine::common::*;
use ruggine::protocol::*;

pub struct UserInterface {
    current_state: ClientState,
}

#[derive(Debug, Clone)]
pub enum ClientState {
    NotAuthenticated,
    Home,
    InGroup(String), // Nome del gruppo
}

impl UserInterface {
    pub fn new() -> Self {
        Self {
            current_state: ClientState::NotAuthenticated,
        }
    }

    pub fn show_auth_prompt(&self) -> String {
        match self.current_state {
            ClientState::NotAuthenticated => {
                "Ruggine Chat> ".to_string()
            }
            _ => unreachable!()
        }
    }

    pub fn show_prompt(&self) -> String {
        match &self.current_state {
            ClientState::NotAuthenticated => "Ruggine Chat> ".to_string(),
            ClientState::Home => "Ruggine [HOME]> ".to_string(),
            ClientState::InGroup(group_name) => format!("Ruggine [{}]> ", group_name),
        }
    }

    pub fn set_state(&mut self, state: ClientState) {
        self.current_state = state;
    }

    pub fn get_state(&self) -> &ClientState {
        &self.current_state
    }

    pub fn show_welcome(&self) {
        println!("🦀 Welcome to Ruggine Chat!");
        println!("============================");
        println!("Type 'register <username> <password>' to create an account");
        println!("Type 'login <username> <password>' to sign in");
        println!("Type '/help' for help, '/quit' to exit");
        println!();
    }

    pub fn show_help(&self) {
        match &self.current_state {
            ClientState::NotAuthenticated => {
                println!("📋 Authentication Commands:");
                println!("  register <username> <password> - Create a new account");
                println!("  login <username> <password>    - Sign in to existing account");
                println!("  /quit, /q                      - Exit application");
            }
            ClientState::Home => {
                println!("📋 Available Commands:");
                println!("  /help, /h             - Show this help");
                println!("  /quit, /q             - Exit application");
                println!("  /groups, /g           - List your groups");
                println!("  /create <name>        - Create a new group");
                println!("  /join <group_name>    - Enter a group by name");
            }
            ClientState::InGroup(_) => {
                println!("📋 Group Commands:");
                println!("  /help, /h             - Show this help");
                println!("  /quit, /q             - Exit application");
                println!("  /groups, /g           - List your groups");
                println!("  /create <name>        - Create a new group");
                println!("  /join <group_name>    - Enter a different group");
                println!("  /invite <username>    - Invite a user to current group");
                println!("  /home                 - Return to home (leave group temporarily)");
                println!("  /quit-group           - Leave group permanently");
                println!("  /users, /u            - List users in current group");
                println!("  <message>             - Send a message to the group");
            }
        }
    }

    pub fn handle_response(&mut self, response: ProtocolMessage) {
        match response {
            ProtocolMessage::AuthResult { success, message, .. } => {
                if success {
                    println!("✅ {}", message);
                    self.current_state = ClientState::Home;
                } else {
                    println!("❌ {}", message);
                }
            }

            ProtocolMessage::GroupCreated { group } => {
                println!("✅ Group '{}' created successfully!", group.name);
            }

            ProtocolMessage::GroupJoined { group } => {
                println!("✅ Joined group '{}'", group.name);
                self.current_state = ClientState::InGroup(group.name);
            }

            ProtocolMessage::GroupLeft => {
                println!("✅ Left group, back to home");
                self.current_state = ClientState::Home;
            }

            ProtocolMessage::GroupQuit => {
                println!("✅ Permanently left group");
                self.current_state = ClientState::Home;
            }

            ProtocolMessage::UserInvited { username } => {
                println!("✅ User '{}' has been invited to the group", username);
            }

            ProtocolMessage::MessageReceived { message } => {
                if matches!(self.current_state, ClientState::InGroup(_)) {
                    println!("📨 Message sent successfully");
                } else {
                    // Messaggio ricevuto da altri
                    self.display_message(&message);
                }
            }

            ProtocolMessage::GroupListResponse { groups } => {
                if groups.is_empty() {
                    println!("📋 You are not a member of any groups.");
                } else {
                    println!("📋 Your Groups:");
                    for group in groups {
                        println!("   • {} ({} members)", group.name, group.members.len());
                    }
                }
            }

            ProtocolMessage::UserListResponse { users } => {
                println!("👥 Users in current group:");
                for user in users {
                    println!("   • {}", user);
                }
            }

            ProtocolMessage::Ok { message } => {
                println!("✅ {}", message);
            }

            ProtocolMessage::Error { message } => {
                println!("❌ {}", message);
            }

            _ => {
                println!("📦 Server response: {:?}", response);
            }
        }
    }

    pub fn display_message(&self, message: &Message) {
        println!("💬 [{}] {}: {}",
               message.timestamp.format("%H:%M:%S"),
               message.sender,
               message.content);
    }

    pub fn parse_command(&self, input: &str) -> Option<ProtocolMessage> {
        let input = input.trim();
        
        match &self.current_state {
            ClientState::NotAuthenticated => {
                if input.starts_with("register ") {
                    let parts: Vec<&str> = input.splitn(3, ' ').collect();
                    if parts.len() == 3 {
                        Some(ProtocolMessage::Register {
                            username: parts[1].to_string(),
                            password: parts[2].to_string(),
                        })
                    } else {
                        println!("❌ Usage: register <username> <password>");
                        None
                    }
                } else if input.starts_with("login ") {
                    let parts: Vec<&str> = input.splitn(3, ' ').collect();
                    if parts.len() == 3 {
                        Some(ProtocolMessage::Login {
                            username: parts[1].to_string(),
                            password: parts[2].to_string(),
                        })
                    } else {
                        println!("❌ Usage: login <username> <password>");
                        None
                    }
                } else if input == "/quit" || input == "/q" {
                    Some(ProtocolMessage::Quit)
                } else {
                    println!("❌ Unknown command. Type 'register <username> <password>' or 'login <username> <password>'");
                    None
                }
            }

            ClientState::Home | ClientState::InGroup(_) => {
                match input {
                    "/help" | "/h" => {
                        self.show_help();
                        None
                    }
                    "/quit" | "/q" => Some(ProtocolMessage::Quit),
                    "/groups" | "/g" => Some(ProtocolMessage::ListGroups),
                    "/users" | "/u" => {
                        if matches!(self.current_state, ClientState::InGroup(_)) {
                            Some(ProtocolMessage::ListUsers)
                        } else {
                            println!("❌ You must be in a group to list users");
                            None
                        }
                    }
                    "/home" => {
                        if matches!(self.current_state, ClientState::InGroup(_)) {
                            Some(ProtocolMessage::GoHome)
                        } else {
                            println!("❌ You are already at home");
                            None
                        }
                    }
                    "/quit-group" => {
                        if matches!(self.current_state, ClientState::InGroup(_)) {
                            Some(ProtocolMessage::QuitGroup)
                        } else {
                            println!("❌ You are not in a group");
                            None
                        }
                    }
                    _ if input.starts_with("/create ") => {
                        let name = input.strip_prefix("/create ").unwrap().trim();
                        if name.is_empty() {
                            println!("❌ Usage: /create <group_name>");
                            None
                        } else {
                            Some(ProtocolMessage::CreateGroup {
                                name: name.to_string(),
                            })
                        }
                    }
                    _ if input.starts_with("/join ") => {
                        let group_name = input.strip_prefix("/join ").unwrap().trim();
                        if group_name.is_empty() {
                            println!("❌ Usage: /join <group_name>");
                            None
                        } else {
                            Some(ProtocolMessage::JoinGroup {
                                group_name: group_name.to_string(),
                            })
                        }
                    }
                    _ if input.starts_with("/invite ") => {
                        if matches!(self.current_state, ClientState::InGroup(_)) {
                            let username = input.strip_prefix("/invite ").unwrap().trim();
                            if username.is_empty() {
                                println!("❌ Usage: /invite <username>");
                                None
                            } else {
                                Some(ProtocolMessage::InviteUser {
                                    username: username.to_string(),
                                })
                            }
                        } else {
                            println!("❌ You must be in a group to invite users");
                            None
                        }
                    }
                    _ if input.starts_with("/") => {
                        println!("❌ Unknown command '{}'. Type /help for available commands", input);
                        None
                    }
                    _ => {
                        // Messaggio normale
                        if matches!(self.current_state, ClientState::InGroup(_)) {
                            if !input.is_empty() {
                                Some(ProtocolMessage::SendMessage {
                                    content: input.to_string(),
                                })
                            } else {
                                None
                            }
                        } else {
                            println!("❌ You must be in a group to send messages");
                            None
                        }
                    }
                }
            }
        }
    }
}
