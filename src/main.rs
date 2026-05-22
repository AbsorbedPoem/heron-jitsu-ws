use tokio::net::TcpListener;
mod turns;
mod core;
mod gamestate;
mod cards;
use std::sync::atomic::{AtomicU64};
use std::sync::Arc;
use tokio::sync::Mutex;

static NEXT_PLAYER_ID: AtomicU64 = AtomicU64::new(1);

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("No se pudo bindear");

    println!("WebSocket escuchando en ws://127.0.0.1:3000");
    let state: gamestate::SharedState = Arc::new(Mutex::new(gamestate::GameState::default()));

    while let Ok((stream, _)) = listener.accept().await {

        let (player_id, state_clone, ws_stream) = match core::asign_to_game(&state, &NEXT_PLAYER_ID, stream).await {
            Ok(result) => result,
            Err(err) => {
                eprintln!("Error asignando jugador {err}"); continue; // Rechaza la conexión y espera la siguiente
            }
        };

        tokio::spawn(core::handle_ws_connection(ws_stream, state_clone, player_id));
    }
}