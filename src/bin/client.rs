use std::io::{self, Write, BufRead, BufReader};
use std::net::TcpStream;
use chrono;
use std::sync::mpsc;
use std::thread;

use ruggine::protocol::ProtocolMessage;

#[derive(PartialEq)]
enum ClientState {
    NotAuthenticated,
    Home,
    InGroup(String),
}

struct UserInterface {
    pub state: ClientState,
}

impl UserInterface {
    fn new() -> Self {
        Self {
            state: ClientState::NotAuthenticated,
        }
    }

    fn show_welcome(&self) {
        println!("🚀 Welcome to Ruggine Chat!");
        println!("Type /help for available commands.");
    }

    fn show_prompt(&self) -> String {
        match &self.state {
            ClientState::NotAuthenticated => "> ".to_string(),
            ClientState::Home => "home> ".to_string(),
            ClientState::InGroup(group) => format!("{}> ", group),
        }
    }

    fn show_available_commands(&self) {
        match &self.state {
            ClientState::NotAuthenticated => {
                println!("\n📚 Available commands:");
                println!("  /register <username> <password> - Register new account");
                println!("  /login <username> <password>    - Login to existing account");
                println!("  /quit                           - Exit application");
            }
            ClientState::Home => {
                println!("\n📚 Available commands:");
                println!("  /groups           - List your groups");
                println!("  /create <name>    - Create a new group");
                println!("  /join <name>      - Join a group");
                println!("  /quit             - Exit application");
            }
            ClientState::InGroup(group_name) => {
                println!("\n📚 Available commands in group '{}':", group_name);
                println!("  /home             - Return to home");
                println!("  /quit-group       - Leave current group");
                println!("  /invite <user>    - Invite user to group");
                println!("  /users            - List group users");
                println!("  <message>         - Send message to group");
            }
        }
        println!(); // Riga vuota per separare
    }

    fn parse_command(&mut self, input: &str) -> Option<ProtocolMessage> {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let command = parts[0];

        match &self.state {
            ClientState::NotAuthenticated => {
                match command {
                    "/register" => {
                        if parts.len() == 2 {
                            let credentials: Vec<&str> = parts[1].splitn(2, ' ').collect();
                            if credentials.len() == 2 {
                                Some(ProtocolMessage::Register {
                                    username: credentials[0].to_string(),
                                    password: credentials[1].to_string(),
                                })
                            } else {
                                println!("❌ Usage: /register <username> <password>");
                                None
                            }
                        } else {
                            println!("❌ Usage: /register <username> <password>");
                            None
                        }
                    }
                    "/login" => {
                        if parts.len() == 2 {
                            let credentials: Vec<&str> = parts[1].splitn(2, ' ').collect();
                            if credentials.len() == 2 {
                                Some(ProtocolMessage::Login {
                                    username: credentials[0].to_string(),
                                    password: credentials[1].to_string(),
                                })
                            } else {
                                println!("❌ Usage: /login <username> <password>");
                                None
                            }
                        } else {
                            println!("❌ Usage: /login <username> <password>");
                            None
                        }
                    }
                    "/help" => {
                        println!("📚 Available commands:");
                        println!("  /register <username> <password> - Register new account");
                        println!("  /login <username> <password>    - Login to existing account");
                        println!("  /quit                           - Exit application");
                        None
                    }
                    "/quit" => Some(ProtocolMessage::Quit),
                    _ => {
                        println!("❌ Unknown command. Type /help for available commands.");
                        None
                    }
                }
            }
            ClientState::Home => {
                match command {
                    "/help" => {
                        println!("📚 Available commands:");
                        println!("  /groups           - List your groups");
                        println!("  /create <name>    - Create a new group");
                        println!("  /join <name>      - Join a group");
                        println!("  /quit             - Exit application");
                        None
                    }
                    "/groups" => Some(ProtocolMessage::ListGroups),
                    "/create" => {
                        if parts.len() == 2 {
                            Some(ProtocolMessage::CreateGroup {
                                name: parts[1].to_string(),
                            })
                        } else {
                            println!("❌ Usage: /create <group_name>");
                            None
                        }
                    }
                    "/join" => {
                        if parts.len() == 2 {
                            let group_name = parts[1].to_string();
                            // Non cambiamo stato qui - lo faremo solo se il server conferma il successo
                            Some(ProtocolMessage::JoinGroup { group_name })
                        } else {
                            println!("❌ Usage: /join <group_name>");
                            None
                        }
                    }
                    "/quit" => Some(ProtocolMessage::Quit),
                    _ => {
                        println!("❌ Unknown command. Type /help for available commands.");
                        None
                    }
                }
            }
            ClientState::InGroup(group_name) => {
                match command {
                    "/help" => {
                        println!("📚 Available commands in group:");
                        println!("  /home             - Return to home");
                        println!("  /quit-group       - Leave current group");
                        println!("  /invite <user>    - Invite user to group");
                        println!("  /users            - List group users");
                        println!("  <message>         - Send message to group");
                        None
                    }
                    "/home" => {
                        self.state = ClientState::Home;
                        println!("🏠 Returned to home");
                        self.show_available_commands();
                        Some(ProtocolMessage::GoHome)
                    }
                    "/quit-group" => {
                        let group_name_clone = group_name.clone();
                        self.state = ClientState::Home;
                        Some(ProtocolMessage::LeaveGroup { group_name: group_name_clone })
                    }
                    "/invite" => {
                        if parts.len() == 2 {
                            Some(ProtocolMessage::InviteUser {
                                username: parts[1].to_string(),
                                group_name: group_name.clone(),
                            })
                        } else {
                            println!("❌ Usage: /invite <username>");
                            None
                        }
                    }
                    "/users" => Some(ProtocolMessage::ListGroupUsers { group_name: group_name.clone() }),
                    "/quit" => Some(ProtocolMessage::Quit),
                    _ => {
                        // Messaggio normale
                        Some(ProtocolMessage::SendMessage {
                            content: input.to_string(),
                            group_name: group_name.clone(),
                        })
                    }
                }
            }
        }
    }

    fn handle_response(&mut self, response: ProtocolMessage) -> Option<Vec<ruggine::common::ChatMessage>> {
        match response {
            ProtocolMessage::AuthResult { success, message, .. } => {
                if success {
                    println!("✅ {}", message);
                    self.state = ClientState::Home;
                    // Mostra automaticamente i comandi disponibili dopo il login/registrazione
                    self.show_available_commands();
                } else {
                    println!("❌ {}", message);
                }
                None
            }
            ProtocolMessage::GroupListResponse { groups } => {
                if groups.is_empty() {
                    println!("📭 You are not in any groups");
                } else {
                    println!("📋 Your groups:");
                    for group in groups {
                        println!("  • {}", group.name);
                    }
                }
                None
            }
            ProtocolMessage::GroupJoined { group, recent_messages } => {
                println!("✅ Entered group '{}'!", group.name);
                // Restituisce i messaggi per mostrarli dopo i comandi
                Some(recent_messages)
            }
            ProtocolMessage::UserListResponse { users } => {
                println!("👥 Users:");
                for user in users {
                    println!("  • {}", user);
                }
                None
            }
            ProtocolMessage::Ok { message } => {
                println!("✅ {}", message);
                None
            }
            ProtocolMessage::Error { message } => {
                println!("❌ {}", message);
                None
            }
            ProtocolMessage::MessageReceived { message: _, recent_messages } => {
                // Conferma che il messaggio è stato inviato (opzionale)
                println!("✅ Message sent");
                // Restituisce i messaggi recenti per mostrarli
                Some(recent_messages)
            }
            ProtocolMessage::ReloadMessages { recent_messages } => {
                // Mostra i nuovi messaggi ricevuti
                println!("📬 New messages received!");
                Some(recent_messages)
            }
            _ => {
                println!("{:?}", response);
                None
            }
        }
    }

    fn show_recent_messages(&self, messages: &[ruggine::common::ChatMessage]) {
        if !messages.is_empty() {
            println!("\n💬 Recent messages:");
            println!("═══════════════════");
            for message in messages {
                // Formatta il timestamp per renderlo più leggibile
                let timestamp = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&message.timestamp) {
                    dt.format("%H:%M:%S").to_string()
                } else {
                    message.timestamp.clone()
                };
                println!("[{}] {}: {}", timestamp, message.username, message.content);
            }
            println!("═══════════════════\n");
        } else {
            println!("📭 No recent messages in this group.\n");
        }
    }
}

struct ChatClient {
    stream: TcpStream,
    ui: UserInterface,
}

impl ChatClient {
    fn new(stream: TcpStream) -> Result<Self, Box<dyn std::error::Error>> {
        let ui = UserInterface::new();
        
        Ok(Self {
            stream,
            ui,
        })
    }

    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use std::sync::{Arc, Mutex, mpsc};
        use std::io::{BufReader, BufRead};

        self.ui.show_welcome();

        let (tx_raw, rx_raw) = mpsc::channel::<ProtocolMessage>();
        let stream_clone = self.stream.try_clone()?;
        let ui = Arc::new(Mutex::new(std::mem::replace(&mut self.ui, UserInterface::new())));

        // THREAD 1: Lettura socket -> tx_raw
        {
            let tx_raw = tx_raw.clone();
            thread::spawn(move || {
                let mut reader = BufReader::new(stream_clone);
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_ok() {
                        if let Ok(msg) = ProtocolMessage::from_wire_format(&line) {
                            if tx_raw.send(msg).is_err() {
                                break;
                            }
                        }
                    }
                }
            });
        }

        // THREAD 2: Stampa messaggi dal server in tempo reale
        let ui_for_rx = Arc::clone(&ui);
        thread::spawn(move || {
            for message in rx_raw {
                // Limit lock duration: lock, handle, unlock, then lock, show, unlock.
                let messages = {
                    let mut ui = ui_for_rx.lock().unwrap();
                    ui.handle_response(message)
                };
                if let Some(msgs) = messages {
                    let ui = ui_for_rx.lock().unwrap();
                    ui.show_recent_messages(&msgs);
                    print!("{}", ui.show_prompt());
                    io::stdout().flush().unwrap();
                }
            }
        });

        // THREAD PRINCIPALE: Gestione input utente
        loop {
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                // Show prompt again at end of loop
                let prompt = {
                    let ui = ui.lock().unwrap();
                    ui.show_prompt()
                };
                print!("{}", prompt);
                io::stdout().flush()?;
                continue;
            }

            let command = {
                let mut ui = ui.lock().unwrap();
                ui.parse_command(input)
            };

            if let Some(message) = command {
                if matches!(message, ProtocolMessage::Quit) {
                    self.send_message(&message)?;
                    break;
                }

                let is_join_command = matches!(message, ProtocolMessage::JoinGroup { .. });
                let is_leave_command = matches!(message, ProtocolMessage::LeaveGroup { .. });
                let group_name_for_join = if let ProtocolMessage::JoinGroup { group_name } = &message {
                    Some(group_name.clone())
                } else {
                    None
                };

                self.send_message(&message)?;

                if is_join_command {
                    if let Some(group_name) = group_name_for_join {
                        let mut ui = ui.lock().unwrap();
                        ui.state = ClientState::InGroup(group_name);
                    }
                }

                if is_join_command || is_leave_command {
                    let ui = ui.lock().unwrap();
                    ui.show_available_commands();
                }
            }

            // Mostra prompt alla fine del ciclo
            let prompt = {
                let ui = ui.lock().unwrap();
                ui.show_prompt()
            };
            print!("{}", prompt);
            io::stdout().flush()?;
        }

        println!("👋 Goodbye!");
        Ok(())
    }

    fn send_message(&mut self, message: &ProtocolMessage) -> Result<(), Box<dyn std::error::Error>> {
        let data = message.to_wire_format()?;
        self.stream.write_all(data.as_bytes())?;
        self.stream.flush()?;
        Ok(())
    }

    /*fn receive_message(&mut self) -> Result<ProtocolMessage, Box<dyn std::error::Error>> {
        let mut reader = BufReader::new(&self.stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        
        let message = ProtocolMessage::from_wire_format(&line)?;
        Ok(message)
    }*/
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦀 Ruggine Chat Client");
    println!("====================");
    
    // Richiede l'indirizzo del server
    print!("Enter server address (default: 127.0.0.1:8080): ");
    io::stdout().flush()?;
    
    let mut server_addr = String::new();
    io::stdin().read_line(&mut server_addr)?;
    let server_addr = server_addr.trim();
    let server_addr = if server_addr.is_empty() {
        "127.0.0.1:8080"
    } else {
        server_addr
    };
    
    // Connessione al server
    println!("🔌 Connecting to {}...", server_addr);
    let stream = TcpStream::connect(server_addr)?;
    println!("✅ Connected to server!");
    
    // Crea e avvia il client
    let mut client = ChatClient::new(stream)?;
    client.run()?;

    Ok(())
}
