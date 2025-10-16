use std::sync::{Arc, Mutex};
use rusqlite::{Connection, Result as SqlResult, params};
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;
use chrono::Utc;
use crate::common::*;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(db_path: &str) -> SqlResult<Self> {
        let conn = Connection::open(db_path)?;
        let db = Database { 
            conn: Arc::new(Mutex::new(conn)) 
        };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        
        // Tabella utenti
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        // Tabella gruppi
        conn.execute(
            "CREATE TABLE IF NOT EXISTS groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                creator_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(creator_id) REFERENCES users(id)
            )",
            [],
        )?;

        // Tabella appartenenze ai gruppi
        conn.execute(
            "CREATE TABLE IF NOT EXISTS group_memberships (
                id TEXT PRIMARY KEY,
                group_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                joined_at TEXT NOT NULL,
                FOREIGN KEY(group_id) REFERENCES groups(id),
                FOREIGN KEY(user_id) REFERENCES users(id),
                UNIQUE(group_id, user_id)
            )",
            [],
        )?;

        // Tabella messaggi
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                group_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                content TEXT NOT NULL,
                sent_at TEXT NOT NULL,
                FOREIGN KEY(group_id) REFERENCES groups(id),
                FOREIGN KEY(user_id) REFERENCES users(id)
            )",
            [],
        )?;

        // Tabella per tracciare chi ha abbandonato un gruppo
        conn.execute(
            "CREATE TABLE IF NOT EXISTS group_departures (
                id TEXT PRIMARY KEY,
                group_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                left_at TEXT NOT NULL,
                FOREIGN KEY(group_id) REFERENCES groups(id),
                FOREIGN KEY(user_id) REFERENCES users(id),
                UNIQUE(group_id, user_id)
            )",
            [],
        )?;

        Ok(())
    }

    pub fn register_user(&self, username: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Verifica se l'utente esiste già
        if self.user_exists(username)? {
            return Err("Username already exists".into());
        }

        let user_id = Uuid::new_v4().to_string();
        let password_hash = hash(password, DEFAULT_COST)?;
        let created_at = Utc::now().to_rfc3339();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![user_id, username, password_hash, created_at],
        )?;

        Ok(user_id)
    }

    pub fn login_user(&self, username: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, password_hash FROM users WHERE username = ?1")?;
        
        let user_result = stmt.query_row(params![username], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        });

        match user_result {
            Ok((user_id, password_hash)) => {
                if verify(password, &password_hash)? {
                    Ok(user_id)
                } else {
                    Err("Invalid password".into())
                }
            }
            Err(_) => Err("User not found".into()),
        }
    }

    fn user_exists(&self, username: &str) -> SqlResult<bool> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM users WHERE username = ?1")?;
        let count: i64 = stmt.query_row(params![username], |row| row.get(0))?;
        Ok(count > 0)
    }

    pub fn create_group(&self, name: &str, creator_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let group_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();

        let conn = self.conn.lock().unwrap();
        
        // Crea il gruppo
        conn.execute(
            "INSERT INTO groups (id, name, creator_id, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![group_id, name, creator_id, created_at],
        )?;

        // Aggiunge il creatore al gruppo
        let membership_id = Uuid::new_v4().to_string();
        let joined_at = Utc::now().to_rfc3339();
        
        conn.execute(
            "INSERT INTO group_memberships (id, group_id, user_id, joined_at) VALUES (?1, ?2, ?3, ?4)",
            params![membership_id, group_id, creator_id, joined_at],
        )?;

        Ok(())
    }

    pub fn get_user_groups(&self, user_id: &str) -> Result<Vec<Group>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
            
        let mut stmt = conn.prepare(
            "SELECT g.id, g.name, g.creator_id, g.created_at 
             FROM groups g 
             JOIN group_memberships gm ON g.id = gm.group_id 
             WHERE gm.user_id = ?1"
        )?;

        let group_iter = stmt.query_map(params![user_id], |row| {
            Ok(Group {
                id: row.get::<_, String>(0)?,
                name: row.get::<_, String>(1)?,
                creator_id: row.get::<_, String>(2)?,
                created_at: row.get::<_, String>(3)?,
                members: Vec::new(), // Popoleremo dopo se necessario
            })
        })?;

        let mut groups = Vec::new();
        for group in group_iter {
            groups.push(group?);
        }

        Ok(groups)
    }

    pub fn get_all_users(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT username FROM users ORDER BY username")?;
        
        let user_iter = stmt.query_map([], |row| {
            Ok(row.get::<_, String>(0)?)
        })?;

        let mut users = Vec::new();
        for user in user_iter {
            users.push(user?);
        }

        Ok(users)
    }

    pub fn get_user_count(&self) -> SqlResult<u32> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM users")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as u32)
    }

    pub fn get_group_count(&self) -> SqlResult<u32> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM groups")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as u32)
    }

    pub fn get_message_count(&self) -> SqlResult<u32> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM messages")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as u32)
    }

    pub fn join_group(&self, group_name: &str, user_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
                
        // Trova l'ID del gruppo
        let mut stmt = conn.prepare("SELECT id FROM groups WHERE name = ?1")?;
        let group_id: String = stmt.query_row(params![group_name], |row| row.get(0))
            .map_err(|_| "Group not found")?;

        // Verifica se l'utente è già nel gruppo
        let mut check_stmt = conn.prepare("SELECT COUNT(*) FROM group_memberships WHERE group_id = ?1 AND user_id = ?2")?;
        let count: i64 = check_stmt.query_row(params![group_id, user_id], |row| row.get(0))?;
                
        if count > 0 {
            // L'utente è già nel gruppo - questo è OK, permettiamo di "entrare" nel gruppo
            return Ok(());
        }

        // Verifica se l'utente ha abbandonato questo gruppo in precedenza
        let mut departure_stmt = conn.prepare("SELECT COUNT(*) FROM group_departures WHERE group_id = ?1 AND user_id = ?2")?;
        let departure_count: i64 = departure_stmt.query_row(params![group_id, user_id], |row| row.get(0))?;
                
        if departure_count > 0 {
            return Err("You cannot rejoin a group you have left. You need to be invited by another member.".into());
        }

        // Aggiunge l'utente al gruppo (solo se non era già membro e non ha mai abbandonato)
        let membership_id = Uuid::new_v4().to_string();
        let joined_at = Utc::now().to_rfc3339();
        
        conn.execute(
            "INSERT INTO group_memberships (id, group_id, user_id, joined_at) VALUES (?1, ?2, ?3, ?4)",
            params![membership_id, group_id, user_id, joined_at],
        )?;

        Ok(())
    }

    pub fn invite_user_to_group(&self, group_name: &str, username: &str, inviter_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        
        // Trova l'ID del gruppo
        let mut stmt = conn.prepare("SELECT id FROM groups WHERE name = ?1")?;
        let group_id: String = stmt.query_row(params![group_name], |row| row.get(0))
            .map_err(|_| "Group not found")?;

        // Verifica che l'invitante sia nel gruppo
        let mut check_inviter = conn.prepare("SELECT COUNT(*) FROM group_memberships WHERE group_id = ?1 AND user_id = ?2")?;
        let inviter_count: i64 = check_inviter.query_row(params![group_id, inviter_id], |row| row.get(0))?;
        
        if inviter_count == 0 {
            return Err("You are not a member of this group".into());
        }

        // Trova l'ID dell'utente da invitare
        let mut user_stmt = conn.prepare("SELECT id FROM users WHERE username = ?1")?;
        let user_id: String = user_stmt.query_row(params![username], |row| row.get(0))
            .map_err(|_| "User not found")?;

        // Verifica se l'utente è già nel gruppo
        let mut check_member = conn.prepare("SELECT COUNT(*) FROM group_memberships WHERE group_id = ?1 AND user_id = ?2")?;
        let member_count: i64 = check_member.query_row(params![group_id, user_id], |row| row.get(0))?;
        
        if member_count > 0 {
            return Err("User is already in the group".into());
        }

        // Se l'utente aveva abbandonato il gruppo in precedenza, rimuovi il record di partenza
        conn.execute(
            "DELETE FROM group_departures WHERE group_id = ?1 AND user_id = ?2",
            params![group_id, user_id],
        )?;

        // Aggiunge l'utente al gruppo
        let membership_id = Uuid::new_v4().to_string();
        let joined_at = Utc::now().to_rfc3339();
        
        conn.execute(
            "INSERT INTO group_memberships (id, group_id, user_id, joined_at) VALUES (?1, ?2, ?3, ?4)",
            params![membership_id, group_id, user_id, joined_at],
        )?;

        Ok(())
    }

    pub fn leave_group(&self, group_name: &str, user_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
                
        // Trova l'ID del gruppo
        let mut stmt = conn.prepare("SELECT id FROM groups WHERE name = ?1")?;
        let group_id: String = stmt.query_row(params![group_name], |row| row.get(0))
            .map_err(|_| "Group not found")?;

        // Rimuove l'utente dal gruppo
        let rows_affected = conn.execute(
            "DELETE FROM group_memberships WHERE group_id = ?1 AND user_id = ?2",
            params![group_id, user_id],
        )?;

        if rows_affected == 0 {
            return Err("You are not a member of this group".into());
        }

        // Registra la partenza nella tabella group_departures
        let departure_id = Uuid::new_v4().to_string();
        let left_at = Utc::now().to_rfc3339();
        
        // Usa INSERT OR REPLACE per evitare errori se l'utente ha già abbandonato questo gruppo in passato
        conn.execute(
            "INSERT OR REPLACE INTO group_departures (id, group_id, user_id, left_at) VALUES (?1, ?2, ?3, ?4)",
            params![departure_id, group_id, user_id, left_at],
        )?;

        Ok(())
    }

    pub fn get_group_members(&self, group_name: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        
        // Trova l'ID del gruppo
        let mut stmt = conn.prepare("SELECT id FROM groups WHERE name = ?1")?;
        let group_id: String = stmt.query_row(params![group_name], |row| row.get(0))
            .map_err(|_| "Group not found")?;

        // Ottiene i membri del gruppo
        let mut members_stmt = conn.prepare(
            "SELECT u.username 
             FROM users u 
             JOIN group_memberships gm ON u.id = gm.user_id 
             WHERE gm.group_id = ?1 
             ORDER BY u.username"
        )?;

        let member_iter = members_stmt.query_map(params![group_id], |row| {
            Ok(row.get::<_, String>(0)?)
        })?;

        let mut members = Vec::new();
        for member in member_iter {
            members.push(member?);
        }

        Ok(members)
    }

    pub fn send_message(&self, group_name: &str, user_id: &str, content: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        
        // Trova l'ID del gruppo
        let mut stmt = conn.prepare("SELECT id FROM groups WHERE name = ?1")?;
        let group_id: String = stmt.query_row(params![group_name], |row| row.get(0))
            .map_err(|_| "Group not found")?;

        // Verifica se l'utente è nel gruppo
        let mut check_stmt = conn.prepare("SELECT COUNT(*) FROM group_memberships WHERE group_id = ?1 AND user_id = ?2")?;
        let count: i64 = check_stmt.query_row(params![group_id, user_id], |row| row.get(0))?;
        
        if count == 0 {
            return Err("You are not a member of this group".into());
        }

        // Crea il messaggio
        let message_id = Uuid::new_v4().to_string();
        let sent_at = Utc::now().to_rfc3339();
        
        conn.execute(
            "INSERT INTO messages (id, group_id, user_id, content, sent_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![message_id, group_id, user_id, content, sent_at],
        )?;

        Ok(vec![message_id, group_id, user_id.to_string(), content.to_string(), sent_at])
    }

    pub fn get_recent_messages(&self, group_name: &str, limit: u32) -> Result<Vec<ChatMessage>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        
        // Trova l'ID del gruppo
        let mut stmt = conn.prepare("SELECT id FROM groups WHERE name = ?1")?;
        let group_id: String = stmt.query_row(params![group_name], |row| row.get(0))
            .map_err(|_| "Group not found")?;

        // Ottiene i messaggi recenti ordinati per timestamp (più recenti per primi)
        let mut messages_stmt = conn.prepare(
            "SELECT m.id, m.content, u.username, m.sent_at 
             FROM messages m 
             JOIN users u ON m.user_id = u.id 
             WHERE m.group_id = ?1 
             ORDER BY m.sent_at DESC 
             LIMIT ?2"
        )?;

        let message_iter = messages_stmt.query_map(params![group_id, limit], |row| {
            Ok(ChatMessage {
                id: row.get::<_, String>(0)?,
                content: row.get::<_, String>(1)?,
                username: row.get::<_, String>(2)?,
                timestamp: row.get::<_, String>(3)?,
            })
        })?;

        let mut messages = Vec::new();
        for message in message_iter {
            messages.push(message?);
        }

        // Inverti l'ordine per avere i messaggi più vecchi per primi
        messages.reverse();
        Ok(messages)
    }

    pub fn get_group_id(&self, group_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id FROM groups WHERE name = ?1")?;
        let group_id: String = stmt.query_row(params![group_name], |row| row.get(0))
            .map_err(|_| format!("Group '{}' not found", group_name))?;
        Ok(group_id)
    }
}
