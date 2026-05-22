use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, MutexGuard};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{tungstenite};
use crate::turns::{PlayerActionRequest, PlayerAction, TurnVariant};
use crate::cards::{Card, CardStack};

#[derive(Debug, Default)]
pub struct GameState {
    pub players: HashMap<u64, Player>,
    pub turns: Vec<TurnVariant>,
    // pub last_turn: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub name: String,
    pub tx: mpsc::UnboundedSender<Message>,
    pub played: bool,
    pub maze: [Card; 7],
    pub card_stack: CardStack
}

// Creamos un alias de tipo para no escribir toda la firma gigante cada vez
pub type SharedState = Arc<Mutex<GameState>>;



pub async fn handle_turn(message: tungstenite::protocol::Message, state: &SharedState, player_id: u64) {
    let tungstenite::protocol::Message::Text(text) = message else {
        eprintln!("Mensaje no es texto");
        return
    };

    let action_request = match PlayerActionRequest::from(&text) {
        Ok(request) => request,
        Err(err) => {
            eprintln!("No se pudo parsear la acción. Error: {err}");
            return
        }
    };

    // println!("Acción recibida del jugador {player_id}: {:?}", action_request);
    let mut game = state.lock().await;


    let Some((myself, opponent)) = find_myself(&game, player_id) else {
        eprintln!("FALLIDO: Jugador u oponente no encontrado");
        return
    };

    do_my_turn(&mut game, &action_request, myself, opponent);

    
    drop(game);
    // println!("Acción procesada: {:?}", action_request);
}


pub fn find_myself<'a>(game: &'a MutexGuard<'a, GameState>, player_id: u64) -> Option<(u64, u64)> {

    let mut myself: Option<u64> = None;
    let mut opponent: Option<u64> = None;

    for (id, _) in &game.players {
        if *id == player_id {
            myself = Some(id.clone());
        } else {
            opponent = Some(id.clone());
        }
    }

    if let (Some(myself), Some(opponent)) = (myself, opponent) {
        return Some((myself, opponent));
    }

    None
}

fn do_my_turn(game: &mut GameState, action_request: &PlayerActionRequest, myself: u64, opponent: u64) {

    match action_request.action {
        PlayerAction::PlayCard(card) => {
            game.play_card(myself, opponent, card);
        },
        PlayerAction::HighlightCard(card) => {
            game.highlight_card(opponent, card.index);
        },
        _ => {
            eprintln!("Acción no reconocida");
            return
        }
    }
}