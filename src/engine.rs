use std::fmt;

use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_xoshiro::SplitMix64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card {
    first: u8,
    second: u8,
}

impl OrientedCard {
    pub fn top(self: &Self) -> u8 {
        match self.orientation {
            Orientation::Lower => self.card.first,
            Orientation::Greater => self.card.second,
        }
    }

    pub fn bottom(self: &Self) -> u8 {
        match self.orientation {
            Orientation::Lower => self.card.second,
            Orientation::Greater => self.card.first,
        }
    }
}

fn build_deck(max_num: u8) -> Vec<Card> {
    let total_cards = max_num * (max_num - 1) / 2 - (max_num * (max_num - 1) / 2 % 4);

    let mut deck = Vec::with_capacity(total_cards as usize);
    let mut count = 0;
    'outer: for i in 1..max_num {
        for j in i + 1..=max_num {
            deck.push(Card {
                first: i,
                second: j,
            });
            count += 1;
            if count == total_cards {
                break 'outer;
            }
        }
    }

    debug_assert_eq!(deck.len(), total_cards as usize);
    debug_assert_eq!(deck.len() % 4, 0usize);

    deck
}

fn shuffle_deck(deck: &mut Vec<Card>, seed: u64) -> Vec<OrientedCard> {
    // We want this to be reproducable, so use SplitMix64 specifically
    let mut rng = SplitMix64::seed_from_u64(seed);
    deck.shuffle(&mut rng);

    deck.iter()
        .map(|&card| OrientedCard {
            card,
            orientation: if rng.gen_bool(0.5) {
                Orientation::Greater
            } else {
                Orientation::Lower
            },
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Orientation {
    Greater,
    Lower,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrientedCard {
    card: Card,
    orientation: Orientation,
}

impl OrientedCard {
    pub fn flip(&self) -> OrientedCard {
        OrientedCard {
            card: self.card,
            orientation: if self.orientation == Orientation::Greater {
                Orientation::Lower
            } else {
                Orientation::Greater
            },
        }
    }
}

impl fmt::Display for OrientedCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let top: u8;
        let bottom: u8;
        if self.orientation == Orientation::Greater {
            top = self.card.first;
            bottom = self.card.second;
        } else {
            top = self.card.second;
            bottom = self.card.first;
        }

        let to_char = |n: u8| {
            if n == 10 {
                'T'
            } else {
                (n + ('0' as u8)) as char
            }
        };
        write!(f, "{}({})", to_char(top), to_char(bottom))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerHiddenState {
    hand: Vec<OrientedCard>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicState {
    orientation_chosen: bool,
    is_hero_turn: bool,
    board: Vec<OrientedCard>,

    hero_card_count: u8,
    villian_card_count: u8,

    hero_won_cards: Vec<Card>,
    villian_won_cards: Vec<Card>,

    hero_scouted_cards: Vec<Card>,
    villian_scouted_cards: Vec<Card>,

    hero_scout_token_count: u8,
    villian_scout_token_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlipHand {
    DoFlip,
    DoNotFlip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickedCard {
    // The first card as ordered on the board
    FirstCard,
    // The last card as ordered on the board
    LastCard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // false to keep current, true to flip
    ChooseOrientation(FlipHand),
    // start index (inclusive), end index (inclusive)
    PlayCards(u8, u8),

    // First or last card -> (index, orientation)
    PlayScoutToken((PickedCard, u8, Orientation)),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionResult {
    MoveAccepted,
    IllegalMove,
    GameComplete,
}

fn build_proposed_board(
    hand: &Vec<OrientedCard>,
    start_idx_u: usize,
    end_idx_u: usize,
) -> Option<Vec<OrientedCard>> {
    let vals: Vec<u8> = hand[start_idx_u..end_idx_u]
        .iter()
        .map(|c| c.top())
        .collect();
    debug_assert!(vals.len() >= 1);
    let base = vals[0];
    // Check for set of like numbers
    if vals.iter().all(|v| *v == base) {
        let mut proposed_board = hand[start_idx_u..end_idx_u].to_vec();
        // The bottom part of the 0 index card should be less than the bottom part of the last card
        if proposed_board[0].bottom() > proposed_board.last().unwrap().bottom() {
            proposed_board.reverse();
        }
        return Some(proposed_board);
    }

    debug_assert!(vals.len() >= 2);

    // Check for increasing or decreasing set
    let ascending = vals[1] > vals[0];
    let is_valid_sequence = vals[..vals.len()]
        .iter()
        .zip(vals[1..].iter())
        .all(|(a, b)| if ascending { a > b } else { b < a });

    if is_valid_sequence {
        let result = if ascending {
            hand[start_idx_u..end_idx_u].to_vec()
        } else {
            hand[start_idx_u..end_idx_u]
                .to_vec()
                .into_iter()
                .rev()
                .collect()
        };
        Some(result)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameCompleteState {
    public_state: PublicState,
    hero_hidden_state: PlayerHiddenState,
    villian_hidden_state: PlayerHiddenState,
}

impl GameCompleteState {
    pub fn new(max_card_num: u8, seed: u64) -> Self {
        let mut deck = build_deck(max_card_num);
        let shuffled_deck = shuffle_deck(&mut deck, seed);

        let cards_per_player = deck.len() / 4;

        let hero_hidden_state = PlayerHiddenState {
            hand: shuffled_deck[0..cards_per_player].to_vec(),
        };
        let villian_hidden_state = PlayerHiddenState {
            hand: shuffled_deck[cards_per_player..cards_per_player * 2].to_vec(),
        };

        debug_assert_eq!(hero_hidden_state.hand.len(), cards_per_player);
        debug_assert_eq!(villian_hidden_state.hand.len(), cards_per_player);

        let public_state = PublicState {
            orientation_chosen: false,
            is_hero_turn: true,
            // empty Vec
            board: Vec::new(),
            hero_card_count: cards_per_player as u8,
            villian_card_count: cards_per_player as u8,

            hero_won_cards: Vec::new(),
            villian_won_cards: Vec::new(),

            hero_scouted_cards: Vec::new(),
            villian_scouted_cards: Vec::new(),

            hero_scout_token_count: 3,
            villian_scout_token_count: 3,
        };

        GameCompleteState {
            public_state,
            hero_hidden_state,
            villian_hidden_state,
        }
    }

    fn handle_orientation_action(&mut self, do_flip: &FlipHand) -> TransitionResult {
        if self.public_state.is_hero_turn {
            match *do_flip {
                FlipHand::DoFlip => {
                    self.hero_hidden_state.hand = self
                        .hero_hidden_state
                        .hand
                        .iter()
                        .map(|c| c.flip())
                        .collect();
                }
                FlipHand::DoNotFlip => {}
            }
            self.public_state.is_hero_turn = false;
        } else {
            match *do_flip {
                FlipHand::DoFlip => {
                    self.hero_hidden_state.hand = self
                        .hero_hidden_state
                        .hand
                        .iter()
                        .map(|c| c.flip())
                        .collect();
                }
                FlipHand::DoNotFlip => {}
            }
            self.public_state.is_hero_turn = true;
            self.public_state.orientation_chosen = true;
        }
        TransitionResult::MoveAccepted
    }

    fn handle_play_card_action(self: &mut Self, start_idx: &u8, end_idx: &u8) -> TransitionResult {
        if start_idx >= end_idx {
            return TransitionResult::IllegalMove;
        }

        let start_idx_u = *start_idx as usize;
        let end_idx_u = *end_idx as usize;

        if self.public_state.is_hero_turn {
            if end_idx_u > self.hero_hidden_state.hand.len() {
                return TransitionResult::IllegalMove;
            }
            let hand = &self.hero_hidden_state.hand;
            let proposed_board = build_proposed_board(hand, start_idx_u, end_idx_u);
            match proposed_board {
                None => {
                    return TransitionResult::IllegalMove;
                }
                Some(proposed_board) => {
                    self.public_state.board = proposed_board;
                }
            }
        } else {
            if end_idx_u > self.villian_hidden_state.hand.len() {
                return TransitionResult::IllegalMove;
            } 
        }
        // Handle the action
        self.public_state.is_hero_turn = !self.public_state.is_hero_turn;
        TransitionResult::MoveAccepted
    }

    fn handle_play_scout_token(
        self: &mut Self,
        picked_card_info: &(PickedCard, u8, Orientation),
    ) -> TransitionResult {
        let picked_card = &picked_card_info.0;
        let insertion_index = picked_card_info.1;
        let orientation = &picked_card_info.2;

        if self.public_state.is_hero_turn {
            if self.public_state.hero_scout_token_count == 0 {
                return TransitionResult::IllegalMove;
            }
            // TODO check board
            // TODO check index
        }
        // Handle the action
        TransitionResult::MoveAccepted
    }

    pub fn transition(self: &mut Self, action: &Action) -> TransitionResult {
        // Three choices for the enum
        return match action {
            Action::ChooseOrientation(do_flip) => self.handle_orientation_action(do_flip),
            Action::PlayCards(start_idx, end_idx) => {
                self.handle_play_card_action(start_idx, end_idx)
            }
            Action::PlayScoutToken(picked_card_info) => {
                self.handle_play_scout_token(picked_card_info)
            }
        };
    }

    pub fn display(&self) -> () {
        if !self.public_state.orientation_chosen {
            if self.public_state.is_hero_turn {
                println!("Hero choosing hand orientation");
            } else {
                println!("Villian choosing hand orientation");
            }
            return;
        }

        if self.public_state.is_hero_turn {
            println!("Hero's turn");
        } else {
            println!("Villian's turn");
        }

        print!("Hero's Hand: ");
        for card in &self.hero_hidden_state.hand {
            print!(" {} ", card);
        }
        println!();
        print!("Villian's Hand: ");
        for card in &self.villian_hidden_state.hand {
            print!(" {} ", card);
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_values() {
        let card = Card {
            first: 7,
            second: 11,
        };
        assert_eq!(card.first, 7);
        assert_eq!(card.second, 11);
        assert_eq!(card, card);
    }

    #[test]
    fn test_build_deck() {
        let deck = build_deck(4);
        assert_eq!(deck.len(), 4);

        let deck = build_deck(10);
        assert_eq!(deck.len(), 44);
    }

    #[test]
    fn test_shuffle_deck() {
        let orig = build_deck(4);
        let mut deck1 = orig.clone();
        let mut deck2 = orig.clone();
        let mut deck3 = orig.clone();

        shuffle_deck(&mut deck1, 0u64);
        shuffle_deck(&mut deck2, 0u64);
        assert_eq!(deck1, deck2);
        assert_ne!(orig, deck1);

        shuffle_deck(&mut deck3, 1u64);
        assert_ne!(deck1, deck3);
        assert_ne!(orig, deck3);
    }

    #[test]
    fn test_choose_orientation() {
        let mut state = GameCompleteState::new(10, 2);
        assert_eq!(state.public_state.is_hero_turn, true);
        assert_eq!(state.public_state.orientation_chosen, false);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        assert_eq!(state.public_state.is_hero_turn, false);
        assert_eq!(state.public_state.orientation_chosen, false);
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        assert_eq!(state.public_state.is_hero_turn, true);
        assert_eq!(state.public_state.orientation_chosen, true);
    }
    #[test]
    fn test_play_illegal_cards() {
        let mut state = GameCompleteState::new(10, 2);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.display();

        let cards_per_player = state.public_state.hero_card_count;

        assert_eq!(state.public_state.is_hero_turn, true);

        let result = state.transition(&Action::PlayCards(0, 3));
        assert_eq!(result, TransitionResult::IllegalMove);
        assert_eq!(state.public_state.is_hero_turn, true);

        let result = state.transition(&Action::PlayCards(100, 0));
        assert_eq!(result, TransitionResult::IllegalMove);
        assert_eq!(state.public_state.is_hero_turn, true);

        let result = state.transition(&Action::PlayCards(1, 1));
        assert_eq!(result, TransitionResult::IllegalMove);
        assert_eq!(state.public_state.is_hero_turn, true);

        let result = state.transition(&Action::PlayCards(1, 0));
        assert_eq!(result, TransitionResult::IllegalMove);
        assert_eq!(state.public_state.is_hero_turn, true);

        let result = state.transition(&Action::PlayCards(cards_per_player, cards_per_player + 1));
        assert_eq!(result, TransitionResult::IllegalMove);
        assert_eq!(state.public_state.is_hero_turn, true);
    }

    #[test]
    fn test_play_same_pair() {
        let mut start_state = GameCompleteState::new(10, 2);
        start_state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        start_state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        start_state.display();

        let mut state = start_state.clone();

        let mut played = state.hero_hidden_state.hand[0..2].to_vec();
        let result = state.transition(&Action::PlayCards(0, 2));
        // Reverse it because the board should be lowest to highest
        played.reverse();

        assert_eq!(result, TransitionResult::MoveAccepted);
        assert_eq!(state.public_state.is_hero_turn, false);
        assert_eq!(state.public_state.board, played);
    }
}
