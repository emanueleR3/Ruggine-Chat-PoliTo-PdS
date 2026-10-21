# Ruggine

A text chat for the terminal, written in Rust. It has a multi-threaded TCP server, a command-line client, group chats and persistent history in SQLite. It was built as the bonus project for the *Programmazione di Sistema* (PdS, System Programming) course at Politecnico di Torino in 2025.

*Ruggine* is Italian for "rust".

## Features

- **Accounts**: users register and log in with a username and password. Passwords are hashed with bcrypt.
- **Groups**: users can create a group, join an existing one by name, or invite other users. Only members can invite, and an invited user is added immediately.
- **Leaving is final**: a user who leaves a group cannot rejoin by themselves. Another member has to invite them back.
- **Live messages**: when someone writes in a group, every member currently inside that group gets the updated conversation pushed to their terminal.
- **History**: users, groups, memberships and messages are stored in SQLite. Entering a group shows its 20 most recent messages.
- **Performance log**: every 2 minutes the server appends its CPU time (from `getrusage`) and the number of users, groups and messages to `server_performance.log`.

## How it works

```
 client ──┐                      ┌──────────────────────────┐
 client ──┼── TCP 127.0.0.1:8080 ┤ server                   │
 client ──┘   newline-delimited  │  one thread per client   │──── ruggine.db (SQLite)
              JSON messages      │  shared state:           │
                                 │  Arc<Mutex<HashMap<..>>> │──── server_performance.log
                                 └──────────────────────────┘
```

- **Protocol** ([`src/protocol.rs`](src/protocol.rs)): a single `ProtocolMessage` enum, serialised with serde as one JSON object per line. Requests (`Login`, `CreateGroup`, `SendMessage`, …) and responses (`AuthResult`, `GroupJoined`, `ReloadMessages`, `Error`, …) share the same type.
- **Server** ([`src/bin/server.rs`](src/bin/server.rs)): blocking `std::net` I/O with one OS thread per connection. The connected users and the group each of them is currently in live in an `Arc<Mutex<HashMap>>`, which is used to push new messages to group members.
- **Persistence** ([`src/database.rs`](src/database.rs)): `rusqlite` with a bundled SQLite. The tables are `users`, `groups`, `group_memberships`, `group_departures` and `messages`.
- **Client** ([`src/bin/client.rs`](src/bin/client.rs)): a small state machine (not logged in → home → inside a group) with one thread reading user input and one listening to the server.

## Getting started

You need a recent Rust toolchain. The project was last built with Rust 1.88.

```bash
cargo build --release

# terminal 1
./target/release/server

# terminal 2, 3, ...
./target/release/client     # press Enter to use the default 127.0.0.1:8080
```

The server creates `ruggine.db` and `server_performance.log` in the directory you start it from. The repository already contains a small sample database (three test users, one group, a few messages) and a sample log.

With Rust 1.88 on macOS (Apple Silicon), the release binaries are about **0.9 MB** for the client and **2.6 MB** for the server. The server is bigger because it bundles SQLite.

## Commands

| Where | Command | What it does |
|---|---|---|
| Not logged in | `/register <user> <password>` | Create an account |
| | `/login <user> <password>` | Log in |
| Home | `/groups` | List the groups you belong to |
| | `/create <group>` | Create a group |
| | `/join <group>` | Enter a group (and join it if you are not a member yet) |
| Inside a group | *any text* | Send a message to the group |
| | `/users` | List the members of the group |
| | `/invite <user>` | Add a user to the group |
| | `/home` | Go back to home without leaving the group |
| | `/quit-group` | Leave the group |
| Anywhere | `/help` | Show the commands available in the current state |
| | `/quit` | Close the client |

## Project layout

```
src/
├── bin/
│   ├── server.rs      Server: accept loop, request handling, performance logger
│   └── client.rs      Client: input parsing, state machine, rendering
├── protocol.rs        ProtocolMessage enum and JSON wire format
├── database.rs        SQLite schema and queries
├── common.rs          Shared types (User, Group, Message, ChatMessage)
├── server/, client/   An earlier modular version, not used by the two binaries
└── lib.rs
```

## Limitations

This is a course project, not a production chat:

- The server address is hard-coded to `127.0.0.1:8080`, so clients on other machines cannot connect without changing the code.
- Traffic is plain TCP with no encryption, and there are no session tokens: the connection itself is the session.
- Any logged-in user can join any group whose name they know. Invitations are only needed to come back after leaving.
- There is one thread per client and a single mutex around the SQLite connection. That is fine for a handful of users, but it doesn't scale.

## Team

Group 11: **Emanuele Romano** and **Giacomo Di Palma**.
