use std::os::windows::fs::OpenOptionsExt;

use tokio_tungstenite::tungstenite::Message;
use serde::Deserialize;
use crate::cards::{Card, CardIndex, SpecialCardEffect, CardBeatCases};
use crate::gamestate::{GameState, Player};


// pub struct Turn {
    //     pub player: u64,
    //     pub card_played: CardIndex,
    //     pub card_appended: Card,
    // }
#[derive(Debug, Clone, Copy)]
pub enum TurnVariant {
    TurnInit(PreTurn),
    TurnCompletion(PostTurn, PostTurn),
}

#[derive(Debug, Clone, Copy)]
pub struct PreTurn {
    pub player_id: u64,
    pub card_played: CardIndex,
}
#[derive(Debug, Clone, Copy)]
pub struct PostTurn {
    pub player_id: u64,
    pub card_played: CardIndex,
    pub card_appended: Card,
}



#[derive(Debug, Deserialize)]
pub enum PlayerAction {
    PlayCard(CardIndex),
    HighlightCard(CardIndex),
    Forfeit,
}

#[derive(Debug, Deserialize)]
pub struct PlayerActionRequest {
    pub action: PlayerAction,
}

impl PlayerActionRequest {
    pub fn from(message: &str) -> Result<Self, serde_json::Error> {
        let action_request: PlayerActionRequest = serde_json::from_str(message)?;
        Ok(action_request)
    }
}


impl GameState {
    pub fn play_card(&mut self, player_id: u64, oponent_id: u64, card_index: CardIndex) {
        println!("Jugador {player_id} intenta jugar la carta en el índice {card_index:?}");

        let player = self.find_player(player_id);
        let oponent = self.find_player(oponent_id);

        if player.played {
            eprintln!("Ya has jugado tu turno");
            return
        };

        if oponent.played {

            match self.decide_turn(player_id, oponent_id, card_index) {
                Ok(turn) => {
                    self.append_turn(turn);
                },
                Err(err) => eprintln!("Error al decidir el ganador: {err}"),
            };
            
        } else {
            println!("Esperando a que el oponente juegue su turno...");
            
            let player = self.find_player_mut(player_id);
            player.played = true;

            self.append_turn(TurnVariant::TurnInit(PreTurn {
                player_id,
                card_played: card_index.clone(),
            }));
            
        }
    }


    pub fn highlight_card(&self, oponent_id: u64, card: u8) {
        let oponent = self.find_player(oponent_id);

        oponent.tx.send(Message::Text(format!("{{\"highlight\": \"{card}\"}}").into())).unwrap();
    }


    fn find_player(&self, player_id: u64) -> &Player {
        match self.players.get(&player_id) {
            Some(player) => player,
            None => panic!("Jugador no encontrado"),
        }
    }

    fn find_player_mut(&mut self, player_id: u64) -> &mut Player {
        match self.players.get_mut(&player_id) {
            Some(player) => player,
            None => panic!("Jugador no encontrado"),
        }
    }

    pub fn append_turn(&mut self, turn: TurnVariant) {
        self.turns.push(turn);
    }

    pub fn get_last_preturn(&self) -> Option<PreTurn> {
        self.turns.iter().rev().find_map(|turn| {
            match turn {
                TurnVariant::TurnInit(preturn) => Some(*preturn),
                _ => None,
            }
        })
    }
    pub fn get_last_used_card_effect(&self) -> SpecialCardEffect {
        SpecialCardEffect::None
    }

    fn decide_turn(&mut self, player_id: u64, oponent_id: u64, card_index: CardIndex) -> Result<TurnVariant, &str> {
        
        let player = self.find_player(player_id);
        let oponent = self.find_player(oponent_id);

        let card = player.card_from_maze(card_index);
        let Some(last_preturn)  = self.get_last_preturn() else {
            return Err("BAD TURN: no se centra el último preturn")
        };
        let oponent_played = oponent.card_from_maze(last_preturn.card_played);
        
        let new_cards = (Card::default(), Card::default());

        {
            let player = self.find_player_mut(player_id);
            player.advertise_for_turn_end(card_index, new_cards.0, oponent_played);
        }
        {
            let oponent = self.find_player_mut(oponent_id);
            oponent.advertise_for_turn_end(card_index, new_cards.0, card);
        }

        self.determine_and_store_winner(player_id, oponent_id, card, oponent_played);
        

        Ok(TurnVariant::TurnCompletion(
            PostTurn {
                player_id: last_preturn.player_id,
                card_played: last_preturn.card_played,
                card_appended: new_cards.0,
            },
            PostTurn {
                player_id,
                card_played: card_index.clone(),
                card_appended: new_cards.0,
            }
        ))
    }

    fn determine_and_store_winner(&mut self, player1_id: u64, player2_id: u64, card1: Card, card2: Card) {

        let wins_with_element = card1.beat_with_element(&card2);
        let wins_with_number = card1.beat_with_number(&card2);


        let winner = match (wins_with_element, wins_with_number) {
            (CardBeatCases::Wins, _) | (_, CardBeatCases::Wins) => CardBeatCases::Wins,
            (CardBeatCases::Loses, _) | (_, CardBeatCases::Loses) => CardBeatCases::Loses,
            _ => CardBeatCases::Ties,
        };

        if winner == CardBeatCases::Wins {

            let player = self.find_player_mut(player1_id);
            player.card_stack.add_to_stack(card1);

            println!("Jugador {player1_id} gana la ronda!");

        } else if winner == CardBeatCases::Loses {

            let oponent = self.find_player_mut(player2_id);
            oponent.card_stack.add_to_stack(card2);

            println!("Jugador {player2_id} gana la ronda!");
        
        } else {
            println!("La ronda termina en empate!");
        };
        
    }


}