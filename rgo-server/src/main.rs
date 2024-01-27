use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{extract::Path, response::Redirect, routing::get, Extension, Router, Json};
use nix::{sys::stat, unistd};
use serde::Deserialize;
use tokio::fs::read;

#[derive(Debug, Deserialize)]
enum Command {
    Add { key: String, value: String },
    Remove { key: String },
    Persist,
}

#[tokio::main]
async fn main() {
    setup_pipe();
    let state = Arc::new(Mutex::new(HashMap::<String, String>::new()));
    read_from_pipe_in_background(state.clone()).await;
    state
        .lock()
        .unwrap()
        .insert("google".to_string(), "https://google.com".to_string());

    let app = Router::new()
        .route("/priv/list", get(list))
        .route("/:key", get(redirect))
        .layer(Extension(state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn redirect(
    Path(key): Path<String>,
    Extension(state): Extension<Arc<Mutex<HashMap<String, String>>>>,
) -> Redirect {
    match state.lock().unwrap().get(&key) {
        Some(x) => Redirect::to(x),
        None => Redirect::to("/404"),
    }
}

async fn list(
    Extension(state): Extension<Arc<Mutex<HashMap<String, String>>>>,
) -> Json<HashMap<String, String>> {
    Json(state.lock().unwrap().clone())
}


fn setup_pipe() {
    if !std::path::Path::new("/tmp/rgo.pipe").exists() {
        match unistd::mkfifo("/tmp/rgo.pipe", stat::Mode::S_IRWXU) {
            Ok(_) => println!("Created fifo"),
            Err(e) => println!("Error creating fifo: {}", e),
        }
    } else {
        println!("Pipe already exists");
    }
}

async fn execute_command(command: Command, state: Arc<Mutex<HashMap<String, String>>>) {
    match command {
        Command::Add { key, value } => state.lock().unwrap().insert(key, value),
        Command::Remove { key } => state.lock().unwrap().remove(&key),
        Command::Persist => {
            println!("Persisting");
            let serialized = rmp_serde::to_vec(&*state.lock().unwrap()).unwrap();
            match tokio::fs::write("/tmp/rgo-client", serialized).await {
                Ok(_) => {println!("Persisted"); Some("Written".to_string())},
                Err(e) => {println!("Error persisting: {}", e); None},
            }
        }
    };
}

async fn read_from_pipe_in_background(state: Arc<Mutex<HashMap<String, String>>>) {
    tokio::spawn(async move {
        loop {
            println!("{:?}", state);
            println!("Reading");
            let command = read("/tmp/rgo.pipe").await;
            match command {
                Ok(command) => {
                    let unpacked: Command = rmp_serde::from_slice(&command).unwrap();
                    println!("Command: {:?}", unpacked);
                    execute_command(unpacked, state.clone()).await;
                }
                Err(e) => println!("Error reading from pipe: {}", e),
            }
        }
    });
}
