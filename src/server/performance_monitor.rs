use std::thread;
use std::time::{Duration, Instant};
use std::fs::OpenOptions;
use std::io::Write;

use crate::server::server::{ChatServer, ServerStats};

pub struct PerformanceMonitor {
    server: ChatServer,
    start_time: Instant,
}

impl PerformanceMonitor {
    pub fn new(server: ChatServer) -> Self {
        Self {
            server,
            start_time: Instant::now(),
        }
    }

    pub fn start_monitoring(&self) {
        let mut last_cpu_time = self.start_time;
        
        loop {
            // Attende 2 minuti
            thread::sleep(Duration::from_secs(120));
            
            let now = Instant::now();
            let cpu_time_ms = now.duration_since(last_cpu_time).as_millis() as u64;
            last_cpu_time = now;
            
            // Ottiene le statistiche dal server
            match self.server.get_stats() {
                Ok(stats) => {
                    self.log_stats(&stats, cpu_time_ms);
                }
                Err(e) => {
                    eprintln!("❌ Error getting server stats: {}", e);
                }
            }
        }
    }

    fn log_stats(&self, stats: &ServerStats, cpu_time_ms: u64) {
        let log_entry = format!(
            "{} - Connected: {}, Total Users: {}, Total Groups: {}, Total Messages: {}, CPU Time: {}ms\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
            stats.connected_users,
            stats.total_users,
            stats.total_groups,
            stats.total_messages,
            cpu_time_ms
        );

        // Scrive nel file di log
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open("server_performance.log")
        {
            Ok(mut file) => {
                if let Err(e) = file.write_all(log_entry.as_bytes()) {
                    eprintln!("❌ Error writing to log file: {}", e);
                } else {
                    println!("📊 Performance logged: {} users connected", stats.connected_users);
                }
            }
            Err(e) => {
                eprintln!("❌ Error opening log file: {}", e);
            }
        }
    }
}
