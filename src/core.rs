use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream};
use futures_util::{SinkExt, StreamExt};
use crate::gamestate::{self, Player, SharedState};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;
use crate::cards;


pub async fn asign_to_game(state: &SharedState, nex_player_id: &AtomicU64, stream: TcpStream)
    -> Result<(u64, SharedState, WebSocketStream<TcpStream>), String> {
    let game_checker = state.lock().await;
    
    if game_checker.players.len() >= 2 {
        println!("Conexión rechazada: La sala ya está llena (2/2 jugadores).");
        // Al dejar que `stream` salga de scope aquí, la conexión TCP se cierra automáticamente
        return Err("Conexión rechazada: La sala ya está llena".into());
    }
    // Liberamos el candado explícitamente para que handle_connection pueda usarlo inmediatamente
    drop(game_checker);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Handshake falló");
    
    let player_id = nex_player_id.fetch_add(1, Ordering::Relaxed);
    let state_clone = Arc::clone(&state);

    // println!("Cliente conectado para jugador {player_id}");

    Ok((player_id, state_clone, ws_stream))
}


pub async fn handle_ws_connection(ws_stream: WebSocketStream<TcpStream>, state: SharedState, player_id: u64) {

    let (mut write, mut read) = ws_stream.split();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    // la tarea encargada de enviar mensajes al cliente
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {

            if let Err(e) = write.send(msg).await {
                eprintln!("error enviando ws: {e}");
                break;
            }
        }
    });


    {
        let mut game = state.lock().await;
        let tx_c = tx.clone();

        let player = Player::from(
                format!("Jugador {player_id}"),
                tx
            );
        let maze = player.maze;

        println!("Jugador {player_id}/2 ha entrado en la sala\n");
        println!("Mazo inicial: {maze:?}\n");
        tx_c.send(Message::Text(format!("Mazo inicial: {maze:?}").into())).unwrap();

        game.players.insert(
            player_id,
            player,
        );

    }

    // la tarea encargada de recibir mensajes del cliente
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                gamestate::handle_turn(msg, &state, player_id).await;
            }
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
        }
    }

    {
        let mut game = state.lock().await;
        game.players.remove(&player_id);
        println!("Jugador {player_id} desconectado. Sala: {}/2", game.players.len());
    }
}

