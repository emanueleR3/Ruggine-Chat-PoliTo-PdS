use std::io::{BufRead, BufReader, Write, self};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;

use ruggine::common::*;
use ruggine::protocol::*;
use crate::ui::UserInterface;
use crate::message_handler::MessageHandler;

pub struct ChatClient {
    stream: TcpStream,
    user_id: Option<UserId>,
    ui: UserInterface,
    message_handler: Arc<Mutex<MessageHandler>>,
}

impl ChatClient {
    pub fn new(stream: TcpStream) -> Result<Self, Box<dyn std::error::Error>> {
        let ui = UserInterface::new();
        let message_handler = Arc::new(Mutex::new(MessageHandler::new()));
        
        Ok(Self {
            stream,
            user_id: None,
            ui,
            message_handler,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Avvia il thread per ricevere messaggi
        let stream_clone = self.stream.try_clone()?;
        let handler_clone = Arc::clone(&self.message_handler);
        let _receive_handle = thread::spawn(move || {
            Self::receive_messages(stream_clone, handler_clone);
        });

        // Mostra benvenuto
        self.ui.show_welcome();

        // Loop principale per l'interfaccia utente
        loop {
            // Mostra il prompt appropriato
            print!("{}", self.ui.show_prompt());
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            // Gestisce comandi speciali
            if input.is_empty() {
                continue;
            }

            // Parsifica il comando
            if let Some(message) = self.ui.parse_command(input) {
                // Gestisce quit prima di inviare
                if matches!(message, ProtocolMessage::Quit) {
                    self.send_message(&message)?;
                    break;
                }

                // Invia il messaggio al server
                self.send_message(&message)?;
                
                // Riceve la risposta
                match self.receive_message() {
                    Ok(response) => {
                        self.ui.handle_response(response);
                    }
                    Err(e) => {
                        eprintln!("❌ Error receiving response: {}", e);
                    }
                }
            }
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

    fn receive_message(&mut self) -> Result<ProtocolMessage, Box<dyn std::error::Error>> {
        let mut reader = BufReader::new(&self.stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        
        let message = ProtocolMessage::from_wire_format(&line)?;
        Ok(message)
    }

    fn receive_messages(
        stream: TcpStream,
        handler: Arc<Mutex<MessageHandler>>,
    ) {
        let mut reader = BufReader::new(stream);
        
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    println!("\n🔌 Connection to server lost");
                    break;
                }
                Ok(_) => {
                    match ProtocolMessage::from_wire_format(&line) {
                        Ok(message) => {
                            let mut handler_lock = handler.lock().unwrap();
                            handler_lock.handle_message(message);
                        }
                        Err(e) => {
                            eprintln!("❌ Error parsing message: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error reading from server: {}", e);
                    break;
                }
            }
        }
    }
}
