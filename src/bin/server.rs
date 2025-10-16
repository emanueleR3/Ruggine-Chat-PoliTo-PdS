use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::io::{BufRead, BufReader, Write};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::HashMap;

use ruggine::common::Message;
use ruggine::database::Database;
use ruggine::protocol::ProtocolMessage;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦀 Ruggine Chat Server");
    println!("======================");
    
    let database = Database::new("ruggine.db")?;
    let database = Arc::new(database);
    
    let connected_users: Arc<Mutex<HashMap<String, (TcpStream, Option<String>)>>> = Arc::new(Mutex::new(HashMap::new()));
    
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("✅ Server listening on 127.0.0.1:8080");
    
    // Thread per il logging delle performance (ogni 2 minuti, con tempo CPU)
    let db_for_stats = Arc::clone(&database);
    thread::spawn(move || {
        // Funzione helper locale per leggere usage CPU (user + system) in millisecondi
        fn read_cpu_time_ms() -> u128 {
            unsafe {
                let mut usage: libc::rusage = std::mem::zeroed();
                if libc::getrusage(libc::RUSAGE_SELF, &mut usage) == 0 {
                    let user_sec = usage.ru_utime.tv_sec as u128;
                    let user_usec = usage.ru_utime.tv_usec as u128;
                    let sys_sec = usage.ru_stime.tv_sec as u128;
                    let sys_usec = usage.ru_stime.tv_usec as u128;
                    (user_sec * 1000 + user_usec / 1000) + (sys_sec * 1000 + sys_usec / 1000)
                } else {
                    0
                }
            }
        }

        let mut last_wall = Instant::now();
        let mut last_cpu_ms = read_cpu_time_ms();
        let mut last_log = Instant::now();
        loop {
            thread::sleep(Duration::from_secs(120)); // intervallo fisso
            if last_log.elapsed() >= Duration::from_secs(120) {
                let now_cpu_ms = read_cpu_time_ms();
                let delta_cpu_ms = now_cpu_ms.saturating_sub(last_cpu_ms);
                let wall_elapsed_ms = last_wall.elapsed().as_millis();
                log_performance_stats(&db_for_stats, now_cpu_ms, delta_cpu_ms, wall_elapsed_ms as u128);
                last_cpu_ms = now_cpu_ms;
                last_wall = Instant::now();
                last_log = Instant::now();
            }
        }
    });
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let database_clone = Arc::clone(&database);
                let connected_users_clone = Arc::clone(&connected_users);
                
                thread::spawn(move || {
                    if let Err(e) = handle_client(stream, database_clone, connected_users_clone) {
                        eprintln!("❌ Error handling client: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("❌ Error accepting connection: {}", e);
            }
        }
    }
    
    Ok(())
}

fn log_performance_stats(database: &Database, cumulative_cpu_ms: u128, delta_cpu_ms: u128, wall_elapsed_ms: u128) {
    match (database.get_user_count(), database.get_group_count(), database.get_message_count()) {
        (Ok(users), Ok(groups), Ok(messages)) => {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
            let stats_line = format!(
                "{} | users={} groups={} messages={} cpu_total_ms={} cpu_delta_ms={} wall_interval_ms={}",
                timestamp, users, groups, messages, cumulative_cpu_ms, delta_cpu_ms, wall_elapsed_ms
            );
            println!("📊 {}", stats_line);

            use std::fs::OpenOptions;
            use std::io::Write;
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open("server_performance.log") {
                if let Err(e) = writeln!(f, "{}", stats_line) {
                    eprintln!("❌ Failed to append performance log: {}", e);
                }
            } else {
                eprintln!("❌ Failed to open performance log file");
            }
        }
        _ => eprintln!("❌ Failed to get performance stats"),
    }
}

fn handle_client(
    mut stream: TcpStream,
    database: Arc<Database>,
    connected_users: Arc<Mutex<HashMap<String, (TcpStream, Option<String>)>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut current_user_id: Option<String> = None;
    
    loop {
        let mut line = String::new();
        let mut reader = BufReader::new(&stream);
        
        match reader.read_line(&mut line) {
            Ok(0) => break, // Client disconnesso
            Ok(_) => {
                if let Ok(message) = ProtocolMessage::from_wire_format(&line) {
                    let response = process_message(message, &database, &connected_users, &mut current_user_id, &stream);
                    
                    if let Ok(response_data) = response.to_wire_format() {
                        if let Err(e) = stream.write_all(response_data.as_bytes()) {
                            eprintln!("❌ Error writing to client: {}", e);
                            break;
                        }
                        if let Err(e) = stream.flush() {
                            eprintln!("❌ Error flushing to client: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Error reading from client: {}", e);
                break;
            }
        }
    }
    
    // Cleanup quando il client si disconnette
    if let Some(user_id) = current_user_id {
        connected_users.lock().unwrap().remove(&user_id);
        println!("🔌 User {} disconnected", user_id);
        //debug_print_connected_users(&connected_users);
    }
    
    Ok(())
}

fn debug_print_connected_users(connected_users: &Arc<Mutex<HashMap<String, (TcpStream, Option<String>)>>>) {
    let users_map = connected_users.lock().unwrap();
    println!("🔍 DEBUG: Connected users state:");
    
    if users_map.is_empty() {
        println!("  📭 No users connected");
    } else {
        for (user_id, (_, group_id_opt)) in users_map.iter() {
            match group_id_opt {
                Some(group_id) => println!("  👤 User {} -> Group ID: {}", user_id, group_id),
                None => println!("  👤 User {} -> Home (no group)", user_id),
            }
        }
    }
    println!("  Total connected users: {}", users_map.len());
    println!();
}

fn process_message(
    message: ProtocolMessage,
    database: &Database,
    connected_users: &Arc<Mutex<HashMap<String, (TcpStream, Option<String>)>>>,
    current_user_id: &mut Option<String>,
    stream: &TcpStream,
) -> ProtocolMessage {
    match message {
        ProtocolMessage::Register { username, password } => {
            match database.register_user(&username, &password) {
                Ok(user_id) => {
                    *current_user_id = Some(user_id.clone());
                    connected_users.lock().unwrap().insert(user_id.clone(), (stream.try_clone().unwrap(), None));
                    println!("✅ User {} registered and connected", user_id);
                    //debug_print_connected_users(connected_users);
                    ProtocolMessage::AuthResult {
                        success: true,
                        user_id: Some(user_id),
                        message: "Registration successful!".to_string(),
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
                    connected_users.lock().unwrap().insert(user_id.clone(), (stream.try_clone().unwrap(), None));
                    println!("✅ User {} logged in and connected", user_id);
                    //debug_print_connected_users(connected_users);
                    ProtocolMessage::AuthResult {
                        success: true,
                        user_id: Some(user_id),
                        message: "Login successful!".to_string(),
                    }
                }
                Err(e) => ProtocolMessage::AuthResult {
                    success: false,
                    user_id: None,
                    message: format!("Login failed: {}", e),
                },
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
                    Ok(_) => {
                        // Ottieni il group_id dal group_name
                        match database.get_group_id(&group_name) {
                            Ok(group_id) => {
                                // Aggiorna il gruppo corrente dell'utente con il group_id
                                if let Some((_stream_ref, current_group)) = connected_users.lock().unwrap().get_mut(user_id) {
                                    *current_group = Some(group_id.clone());
                                }
                                println!("🏠 User {} joined group '{}' (ID: {})", user_id, group_name, group_id);
                                //debug_print_connected_users(connected_users);
                            }
                            Err(_) => {
                                // Se non riusciamo a ottenere il group_id, logghiamo l'errore ma continuiamo
                                eprintln!("❌ Warning: Could not get group_id for group '{}'", group_name);
                            }
                        }
                        
                        // Recupera i messaggi recenti del gruppo (massimo 20)
                        let recent_messages = database.get_recent_messages(&group_name, 20)
                            .unwrap_or_else(|_| Vec::new());
                        
                        // Crea un oggetto Group temporaneo per la risposta
                        let group = ruggine::common::Group {
                            id: "temp_id".to_string(), // Non abbiamo l'ID qui, ma non è critico per il client
                            name: group_name.clone(),
                            members: Vec::new(),
                            creator_id: "".to_string(),
                            created_at: "".to_string(),
                        };
                        
                        ProtocolMessage::GroupJoined { 
                            group,
                            recent_messages,
                        }
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
        
        ProtocolMessage::ListUsers => {
            match database.get_all_users() {
                Ok(users) => ProtocolMessage::UserListResponse { users },
                Err(e) => ProtocolMessage::Error {
                    message: format!("Failed to get users: {}", e),
                },
            }
        }

        ProtocolMessage::ListGroupUsers { group_name } => {
            if let Some(_user_id) = current_user_id {
                match database.get_group_members(&group_name) {
                    Ok(users) => ProtocolMessage::UserListResponse { users },
                    Err(e) => ProtocolMessage::Error {
                        message: format!("Failed to get group members: {}", e),
                    },
                }
            } else {
                ProtocolMessage::Error {
                    message: "Not authenticated".to_string(),
                }
            }
        }

        ProtocolMessage::InviteUser { username, group_name } => {
            if let Some(user_id) = current_user_id {
                match database.invite_user_to_group(&group_name, &username, user_id) {
                    Ok(_) => ProtocolMessage::Ok {
                        message: format!("User '{}' invited to group '{}'!", username, group_name),
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

        ProtocolMessage::LeaveGroup { group_name } => {
            if let Some(user_id) = current_user_id {
                match database.leave_group(&group_name, user_id) {
                    Ok(_) => {
                        // Aggiorna il gruppo corrente dell'utente (torna nella home)
                        if let Some((_stream_ref, current_group)) = connected_users.lock().unwrap().get_mut(user_id) {
                            *current_group = None;
                        }
                        println!("🚪 User {} left group '{}' and returned to home", user_id, group_name);
                        //debug_print_connected_users(connected_users);
                        
                        ProtocolMessage::Ok {
                            message: format!("Left group '{}'!", group_name),
                        }
                    },
                    Err(e) => ProtocolMessage::Error {
                        message: format!("Failed to leave group: {}", e),
                    },
                }
            } else {
                ProtocolMessage::Error {
                    message: "Not authenticated".to_string(),
                }
            }
        }
        
        ProtocolMessage::QuitGroup => {
            if let Some(user_id) = current_user_id {
                // Aggiorna il gruppo corrente dell'utente (torna nella home)
                if let Some((_stream_ref, current_group)) = connected_users.lock().unwrap().get_mut(user_id) {
                    *current_group = None;
                }
                println!("🚪 User {} quit group and returned to home", user_id);
                //debug_print_connected_users(connected_users);
                
                ProtocolMessage::Ok {
                    message: "Left group and returned to home!".to_string(),
                }
            } else {
                ProtocolMessage::Error {
                    message: "Not authenticated".to_string(),
                }
            }
        }
        
        ProtocolMessage::GoHome => {
            if let Some(user_id) = current_user_id {
                // Aggiorna il gruppo corrente dell'utente (torna nella home)
                if let Some((_stream_ref, current_group)) = connected_users.lock().unwrap().get_mut(user_id) {
                    *current_group = None;
                }
                println!("🏠 User {} returned to home", user_id);
                //debug_print_connected_users(connected_users);
                
                ProtocolMessage::Ok {
                    message: "Returned to home".to_string(),
                }
            } else {
                ProtocolMessage::Error {
                    message: "Not authenticated".to_string(),
                }
            }
        }
        
        ProtocolMessage::Quit => {
            if let Some(user_id) = current_user_id {
                let user_id_for_log = user_id.clone();
                connected_users.lock().unwrap().remove(user_id);
                *current_user_id = None;
                println!("👋 User {} quit the application", user_id_for_log);
                //debug_print_connected_users(connected_users);
            }
            ProtocolMessage::Ok {
                message: "Goodbye!".to_string(),
            }
        }

        ProtocolMessage::SendMessage { content, group_name } => {
            if let Some(user_id) = current_user_id {
                // Ricava il group_id dal group_name
                let this_group_id = match database.get_group_id(&group_name) {
                    Ok(id) => id,
                    Err(e) => return ProtocolMessage::Error {
                        message: format!("Failed to get group ID: {}", e),
                    },
                };

                match database.send_message(&group_name, user_id, &content) {
                    Ok(message) => {
                        // Recupera i messaggi recenti del gruppo (massimo 20)
                        let recent_messages = database.get_recent_messages(&group_name, 20)
                            .unwrap_or_else(|_| Vec::new());
                        
                        // Invia in broadcast a tutti i membri del gruppo. Da connected_users vedo chi è connesso a quel group_id e invia un ProtocolMessage::ReloadMessages
                        for (connected_user_id, (user_stream, current_group)) in connected_users.lock().unwrap().iter_mut() {
                            if let Some(group_id) = current_group {
                                if group_id == &this_group_id && connected_user_id != user_id {
                                    let response = ProtocolMessage::ReloadMessages {
                                        recent_messages: recent_messages.clone(),
                                    };
                                    if let Ok(response_data) = response.to_wire_format() {
                                        if let Err(e) = user_stream.write_all(response_data.as_bytes()) {
                                            eprintln!("❌ Error sending message to {}: {}", connected_user_id, e);
                                        }
                                        if let Err(e) = user_stream.flush() {
                                            eprintln!("❌ Error flushing to {}: {}", connected_user_id, e);
                                        }
                                    }
                                }
                            }
                        }

                        ProtocolMessage::MessageReceived {
                            message: Message::new(
                                message[0].clone(), 
                                user_id.clone(),  
                                message[1].clone(), 
                                content.clone()   
                            ),
                            recent_messages,
                        }
                    }
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
        
        _ => ProtocolMessage::Error {
            message: "Command not implemented yet".to_string(),
        },
    }
}
