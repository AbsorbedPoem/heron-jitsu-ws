use serde::Deserialize;
use tokio::sync::{mpsc::UnboundedSender};
use crate::gamestate::{Player};
use tokio_tungstenite::tungstenite::Message;
use serde::{Deserializer, de::Error};
use rand::Rng;

const CARD_MAX_NUMBER: u8 = 12;


#[derive(Debug, Clone, Deserialize, Copy)]
pub struct Card {
    pub number: u8,
    pub element: CardElement,
    pub color: CardColor
}
impl Card {
    pub fn new(number: u8, element: CardElement, color: CardColor) -> Self {
        if number < 1 || number > CARD_MAX_NUMBER {
            panic!("El número de la carta debe estar entre 1 y 12");
        }
        Card { number, element, color }
    }

    pub fn default() -> Self {
        let mut rng = rand::thread_rng();

        let random_number = rng.gen_range(1..=12);

        let element_pool = [CardElement::Fire, CardElement::Water, CardElement::Ice];
        let random_element = element_pool[rng.gen_range(0..element_pool.len())];

        let color_pool = [ CardColor::Red, CardColor::Blue, CardColor::Green, CardColor::Yellow, CardColor::Purple, CardColor::Orange ];
        let random_color = color_pool[rng.gen_range(0..color_pool.len())];

        Card::new(random_number, random_element, random_color)
    }

    pub fn default_maze() -> [Card; 7] {
        [Card::default(); 7]
    }

    pub fn beat_with_element(&self, other: &Card) -> CardBeatCases {
        match (self.element, other.element) {
            // Casos en los que ganas (Piedra, Papel o Tijera elemental)
            (CardElement::Fire, CardElement::Fire)   => CardBeatCases::Ties,
            (CardElement::Water, CardElement::Water) => CardBeatCases::Ties,
            (CardElement::Ice, CardElement::Ice)   => CardBeatCases::Ties,

            // Casos en los que ganas (Piedra, Papel o Tijera elemental)
            (CardElement::Fire, CardElement::Ice)   => CardBeatCases::Wins,
            (CardElement::Water, CardElement::Fire) => CardBeatCases::Wins,
            (CardElement::Ice, CardElement::Water)   => CardBeatCases::Wins,

            // Cualquier otra combinación significa que pierdes
            _ => CardBeatCases::Loses,
        }
    }

    pub fn beat_with_number(&self, other: &Card) -> CardBeatCases {
        if self.number > other.number {
            CardBeatCases::Wins
        } else if self.number < other.number {
            CardBeatCases::Loses
        } else {
            CardBeatCases::Ties
        }
    }
}


#[derive(Debug, Clone, Deserialize, Copy)]
pub enum CardElement {
    Fire,
    Water,
    Ice,
}

#[derive(Debug, Clone, Deserialize, Copy)]
pub enum CardColor {
    Red, Blue, Green, Yellow, Purple, Orange
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardIndex {pub index: u8}
impl<'de> Deserialize<'de> for CardIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let val = u8::deserialize(deserializer)?;
        match CardIndex::from(val) {
            Ok(i) => Ok(i),
            Err(str) => Err(D::Error::custom(str))
        }
    }
}
impl CardIndex {
    pub fn from(index: u8) -> Result<Self, &'static str> {
        if index < 0 || index > 6 {
            return Err("El índice de la carta debe estar entre 0 y 6")
        }
        Ok(CardIndex { index })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardBeatCases {
    Wins,
    Loses,
    Ties
}


pub enum SpecialCardEffect {
    None,
    IngnoreBadHierarchy,
    BlockElement(CardElement),
}



impl Player {
    pub fn from(name: String, tx: UnboundedSender<Message>) -> Self {
        Player {
            name,
            tx,
            played: false,
            maze: Card::default_maze(),
            card_stack: CardStack::default()
        }
    }
    
    pub fn advertise_for_turn_end(&mut self, card_played: CardIndex, card_replace: Card, oponent_played_card: Card) {
        {
            self.tx.send(Message::Text(format!("{{\"message\": \"Tu oponente ha jugado su carta: {oponent_played_card:?}\"}}").into())).unwrap();
            // we can use unwrap safely because card index never goes out of bounds, it´s validated when CardIndex was instantieted
            let used_card_slot = self.maze.get_mut(card_played.index as usize).unwrap();
            *used_card_slot = card_replace;
            self.played = false;
        }

    }

    pub fn mut_card_from_maze(&self, card_index: CardIndex) -> Card {
        self.maze.get(card_index.index as usize).unwrap().clone()
    }
    pub fn card_from_maze(&self, card_index: CardIndex) -> Card {
        self.maze.get(card_index.index as usize).unwrap().clone()
    }
}


#[derive(Clone, Debug)]
pub struct CardStack {
    substacks: [Vec<Card>; 3]
}
impl CardStack {
    fn map_element(card: Card) -> u8 {
        match card.element {
            CardElement::Fire  => 0,
            CardElement::Water => 1,
            CardElement::Ice   => 2,
        }
    }

    pub fn add_to_stack(&mut self, card: Card) {

        let index = Self::map_element(card);
        // unwrap is safe because index is always from 0 to 2
        let stack = self.substacks.get_mut(index as usize).unwrap();

        stack.push(card);
    }

    pub fn default() -> Self {
        Self {
            substacks: [vec![], vec![], vec![]]
        }
    }
}
