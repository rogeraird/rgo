use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{extract::Path, response::Redirect, routing::get, Extension, Router};
use nix::{sys::stat, unistd};
use serde::Deserialize;
use tokio::fs::read;

#[derive(Debug, Deserialize)]
enum Command {
    Add{key: String, value: String},
    Remove{key: String},
    List,
}

#[tokio::main]
async fn main() {
    setup_pipe();
    read_from_pipe_in_background().await;
    let state = Arc::new(Mutex::new(HashMap::<String, String>::new()));
    state
        .lock()
        .unwrap()
        .insert("google".to_string(), "https://google.com".to_string());

    let app = Router::new()
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
    Redirect::to(state.lock().unwrap().get(&key).unwrap())
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

async fn read_from_pipe_in_background() {
    tokio::spawn(async move {
        loop {
            println!("Reading");
            let command = read("/tmp/rgo.pipe").await;
            match command {
                Ok(command) => {
                    let unpacked: Command = rmp_serde::from_slice(&command).unwrap();
                    println!("Command: {:?}", unpacked);
                }
                Err(e) => println!("Error reading from pipe: {}", e),
            }
        }
    });
}
