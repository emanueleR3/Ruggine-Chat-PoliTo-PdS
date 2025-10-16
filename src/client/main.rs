mod client;
mod ui;
mod message_handler;

use std::io::{self, Write};
use std::net::TcpStream;

use client::ChatClient;

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
