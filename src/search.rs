use core::panic;
use itertools::Itertools;
use std::ops::Index;

use crate::engine::{
    build_deck, legal_and_beats_board, Action, Card, GameState, Orientation, OrientedCard,
    PlayerHiddenState, PublicState, TransitionResult,
};

pub struct MoveIter<'a> {
    public_state: &'a PublicState,
    hidden_state: &'a PlayerHiddenState,

    // Internal iteration state
    num_orientations_itered: u8,
    hand_start_idx: usize,
    hand_end_idx: usize,
    num_tokens_itered: u8,
    scout_position_idx: usize,
}

impl<'a> MoveIter<'a> {
    pub fn new(public_state: &'a PublicState, hidden_state: &'a PlayerHiddenState) -> Self {
        MoveIter {
            public_state,
            hidden_state,
            num_orientations_itered: 0,
            hand_start_idx: 0,
            hand_end_idx: 0,
            num_tokens_itered: 0,
            scout_position_idx: 0,
        }
    }
}

impl<'a> Iterator for MoveIter<'a> {
    type Item = Action;

    fn next(&mut self) -> Option<Action> {
        if self.public_state.game_complete {
            return None;
        }

        if !self.public_state.orientation_chosen {
            if self.num_orientations_itered < 2 {
                self.num_orientations_itered += 1;
                return Some(Action::ChooseOrientation(
                    if self.num_orientations_itered == 1 {
                        crate::engine::FlipHand::DoFlip
                    } else {
                        crate::engine::FlipHand::DoNotFlip
                    },
                ));
            } else {
                return None;
            }
        }

        debug_assert_eq!(self.public_state.orientation_chosen, true);

        let hand = &self.hidden_state.hand;
        while self.hand_start_idx < hand.len() {
            while self.hand_end_idx < hand.len() {
                self.hand_end_idx += 1;
                let proposed = &hand[self.hand_start_idx..self.hand_end_idx];
                if legal_and_beats_board(&self.public_state.board, proposed).is_none() {
                    return Some(Action::PlayCards(
                        self.hand_start_idx as u8,
                        self.hand_end_idx as u8,
                    ));
                }
            }
            self.hand_start_idx += 1;
            self.hand_end_idx = self.hand_start_idx;
        }

        let num_tokens = if self.public_state.is_player_one_turn {
            self.public_state.player_one_scout_token_count
        } else {
            self.public_state.player_two_scout_token_count
        };

        if num_tokens == 0 || self.public_state.board.len() == 0 {
            return None;
        }

        if self.num_tokens_itered == 0 {
            while self.scout_position_idx < hand.len() {
                let to_scout_idx = self.scout_position_idx as u8;
                self.scout_position_idx += 1;
                if to_scout_idx % 4 == 0 {
                    return Some(Action::PlayScoutToken((
                        crate::engine::PickedCard::FirstCard,
                        to_scout_idx,
                        crate::engine::Orientation::Larger,
                    )));
                } else if to_scout_idx % 4 == 1 {
                    return Some(Action::PlayScoutToken((
                        crate::engine::PickedCard::FirstCard,
                        to_scout_idx,
                        crate::engine::Orientation::Smaller,
                    )));
                } else if to_scout_idx % 4 == 2 && self.public_state.board.len() > 1 {
                    return Some(Action::PlayScoutToken((
                        crate::engine::PickedCard::LastCard,
                        to_scout_idx,
                        crate::engine::Orientation::Larger,
                    )));
                } else if to_scout_idx % 4 == 2 && self.public_state.board.len() > 1 {
                    return Some(Action::PlayScoutToken((
                        crate::engine::PickedCard::LastCard,
                        to_scout_idx,
                        crate::engine::Orientation::Smaller,
                    )));
                }
            }
        }

        None
    }
}

pub fn walk_games<F>(state: GameState, walker: &mut F)
where
    F: FnMut(GameState),
{
    if state.public_state.game_complete {
        walker(state);
        return;
    }

    let hidden_state = if state.public_state.is_player_one_turn {
        &state.player_one_hidden_state
    } else {
        &state.player_two_hidden_state
    };

    let mut move_iter = MoveIter::new(&state.public_state, hidden_state);

    while let Some(action) = move_iter.next() {
        let mut new_state = state.clone();
        match new_state.transition(&action) {
            TransitionResult::IllegalMove(reason) => {
                panic!("Illegal move ({:?}): {:?}", reason, action);
            }
            _ => {
                walk_games(new_state, walker);
            }
        }
    }
}

type Hand = Vec<OrientedCard>;

fn uu_cards_to_hands<'a>(uu_cards: &'a Vec<&Card>) -> Vec<Hand> {
    let mut hands: Vec<Hand> = vec![];
    for perm in uu_cards.iter().permutations(uu_cards.len()) {
        for mut orientation_bits in 0..2u32.pow(perm.len() as u32) {
            let mut hand: Hand = vec![];
            for card in &perm {
                let bit = orientation_bits & 0x1;
                let orientation = if bit == 0x1 {
                    Orientation::Larger
                } else {
                    Orientation::Smaller
                };
                hand.push(OrientedCard {
                    card: ***card,
                    orientation,
                });
                orientation_bits >>= 1;
            }
            hands.push(hand);
        }
    }
    hands
}

fn build_oriented_hands(unoriented_unordered_cards: &[Card]) -> Vec<(Hand, Hand)> {
    assert_eq!(unoriented_unordered_cards.len() % 2, 0);
    let cards_per_player = unoriented_unordered_cards.len() / 2;
    let player_one_hands_iter = unoriented_unordered_cards
        .iter()
        .combinations(cards_per_player)
        .map(|uu_cards| uu_cards_to_hands(&uu_cards))
        .flatten();

    let mut hands: Vec<(Vec<OrientedCard>, Vec<OrientedCard>)> = vec![];
    for player_one_hand in player_one_hands_iter {
        let player_one_uu_cards = player_one_hand.iter().map(|c| c.card).collect_vec();
        let player_two_uu_cards = unoriented_unordered_cards
            .iter()
            .filter(|c| !player_one_uu_cards.contains(c))
            .collect_vec();
        let player_two_hands = uu_cards_to_hands(&player_two_uu_cards);
        for player_two_hand in player_two_hands {
            hands.push((player_one_hand.clone(), player_two_hand));
        }
    }
    hands
}

pub struct HandIter<'a> {
    card_iter: Box<dyn Iterator<Item = Vec<Card>> + 'a>,
    hands: Vec<(Hand, Hand)>,
    hand_idx: usize,
}

impl<'a> HandIter<'a> {
    pub fn new(deck: &'a Vec<Card>) -> HandIter<'a> {
        debug_assert_eq!(deck.len() % 4, 0);
        let cards_per_player = deck.len() / 4;
        let mut card_iter: Box<dyn Iterator<Item = Vec<Card>> + 'a> = Box::new(
            deck.iter()
                .combinations(cards_per_player * 2)
                .map(|unoriented_hand| unoriented_hand.iter().map(|c| **c).collect()),
        );
        let cards = card_iter.next().unwrap();
        debug_assert!(!cards.is_empty());

        let hands = build_oriented_hands(&cards);

        HandIter {
            card_iter,
            hands,
            hand_idx: 0,
        }
    }
}

impl<'a> Iterator for HandIter<'a> {
    type Item = (Vec<OrientedCard>, Vec<OrientedCard>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.hand_idx == self.hands.len() {
            self.hands = build_oriented_hands(&self.card_iter.next()?);
            self.hand_idx = 0;
        }
        self.hand_idx += 1;
        Some(self.hands[self.hand_idx - 1].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn factorial(k: usize) -> usize {
        if k == 1 { 1 } else { k * factorial(k-1)}
    }

    #[test]
    fn test_iter_orientations() {
        let state = GameState::new_from_seed(4, 0, 123);
        let move_iter = MoveIter::new(&state.public_state, &state.player_one_hidden_state);
        assert_eq!(move_iter.count(), 2);
    }

    #[test]
    fn test_walker_small() {
        let state = GameState::new_from_seed(4, 0, 123);
        let mut count = 0;
        let mut count_fn = |state: GameState| {
            if state.public_state.game_complete {
                count += 1;
            }
        };
        walk_games(state, &mut count_fn);
        // total games depends on the orientation each player picks. there are no choices after that.
        // so 4 games total.
        assert_eq!(count, 4);
    }

    #[test]
    fn test_walker_medium() {
        let state = GameState::new_from_seed(6, 1, 123);
        let mut count = 0;
        let mut count_fn = |_state: GameState| {
            count += 1;
        };
        walk_games(state, &mut count_fn);
        // total games depends on the orientation each player picks. there are no choices after that.
        // so 4 games total.
        assert_eq!(count, 4040);
    }

    #[test]
    fn test_oriented_uu_cards_to_hands_one() {
        let cards = vec![&Card {
            first: 1,
            second: 2,
        }];

        let hands = uu_cards_to_hands(&cards);
        assert_eq!(hands.len(), 2);
    }

    #[test]
    fn test_oriented_uu_cards_to_hands_iter() {
        let cards = vec![
            Card {
                first: 1,
                second: 2,
            },
            Card {
                first: 3,
                second: 4,
            },
        ];

        let hands = build_oriented_hands(&cards);
        // 2 cards * 2 p1 orientations * 2 p2 orientations = 8
        assert_eq!(hands.len(), 8);
    }

    #[test]
    fn test_oriented_hand_four_cards() {
        let cards = vec![
            Card {
                first: 1,
                second: 2,
            },
            Card {
                first: 1,
                second: 3,
            },
            Card {
                first: 2,
                second: 3,
            },
            Card {
                first: 3,
                second: 4,
            },
        ];

        let hands = build_oriented_hands(&cards);
        // 4! perms * 2^4 orientations
        assert_eq!(hands.len(), (factorial(4) * 2u32.pow(4) as usize));
    }

    #[test]
    fn test_generate_a_few_hands() {
        let deck = build_deck(4);
        assert_eq!(deck.len(), 4);
        let mut hand_iter = HandIter::new(&deck);
        let hand = hand_iter.next();
        println!("{:?}", hand);
        assert!(hand.is_some());
        let hand = hand_iter.next();
        assert!(hand.is_some());
    }

    #[test]
    fn test_generate_enough_hands() {
        let deck = build_deck(4);
        assert_eq!(deck.len(), 4);
        let hand_iter = HandIter::new(&deck);
        let hands = hand_iter.collect_vec();
        for hand in &hands {
            println!("{:?}", hand);
        }
        // 4 choices for card 1, 3 for card 2, and each card is either larger or smaller
        assert_eq!(hands.len(), (factorial(4) / factorial(2) * 2u32.pow(2) as usize));
    }

    #[test]
    fn test_generate_six_num_max_hands() {
        let deck = build_deck(6);
        // 3 cards each player
        assert_eq!(deck.len(), 12);
        let hand_iter = HandIter::new(&deck);
        assert_eq!(hand_iter.count(), (factorial(12) / factorial(6) * 2u32.pow(3 * 2) as usize));
    }


}
