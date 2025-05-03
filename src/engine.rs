#![allow(dead_code)]
use std::hash::{DefaultHasher, Hash, Hasher};
use std::{fmt, vec};

use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_xoshiro::SplitMix64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub first: u8,
    pub second: u8,
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let to_char = |n: u8| {
            if n == 10 {
                'T'
            } else {
                (n + ('0' as u8)) as char
            }
        };
        write!(f, "{}|{}", to_char(self.first), to_char(self.second))
    }
}

fn print_cards<T>(cards: &[T])
where
    T: fmt::Display + PartialEq,
{
    for card in cards {
        print!("{}", card);
        if card != cards.last().unwrap() {
            print!(" ");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardSet {
    /// (Start, End), Inclusive
    Consecutive(u8, u8),
    /// (Number, Count)
    Same(u8, u8),
}

impl PartialOrd for CardSet {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        fn num_cards(card_set: CardSet) -> u8 {
            match card_set {
                CardSet::Consecutive(start, end) => end - start + 1,
                CardSet::Same(_number, count) => count,
            }
        }

        if num_cards(*self) != num_cards(*other) {
            return num_cards(*self).partial_cmp(&num_cards(*other));
        }

        match (self, other) {
            (CardSet::Consecutive(s_start, _), CardSet::Consecutive(o_start, _)) => {
                s_start.partial_cmp(o_start)
            }
            (CardSet::Same(s_num, _), CardSet::Same(o_num, _)) => s_num.partial_cmp(o_num),
            (CardSet::Same(..), CardSet::Consecutive(..)) => Some(std::cmp::Ordering::Greater),
            (CardSet::Consecutive(..), CardSet::Same(..)) => Some(std::cmp::Ordering::Less),
        }
    }
}

pub fn build_deck(max_num: u8) -> Vec<Card> {
    // e.g. 10 * 9 / 2 = 45, but -1 so it is divisible by 4 (two games with 10 cards per player)
    // so with 3 it is: 3 * 2 / 2. but that is only 3 cards, so for two games that means each player
    // doesn't get a card. we need at least a max_num of 4 to give each player a single card.
    debug_assert!(max_num >= 4);
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
                Orientation::Larger
            } else {
                Orientation::Smaller
            },
        })
        .collect()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Orientation {
    Larger,
    Smaller,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct OrientedCard {
    pub card: Card,
    pub orientation: Orientation,
}

impl OrientedCard {
    pub fn top(self: &Self) -> u8 {
        match self.orientation {
            Orientation::Smaller => self.card.first,
            Orientation::Larger => self.card.second,
        }
    }

    pub fn bottom(self: &Self) -> u8 {
        match self.orientation {
            Orientation::Smaller => self.card.second,
            Orientation::Larger => self.card.first,
        }
    }

    pub fn flip(&self) -> OrientedCard {
        OrientedCard {
            card: self.card,
            orientation: if self.orientation == Orientation::Larger {
                Orientation::Smaller
            } else {
                Orientation::Larger
            },
        }
    }
}

impl fmt::Display for OrientedCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let to_char = |n: u8| {
            if n == 10 {
                'T'
            } else {
                (n + ('0' as u8)) as char
            }
        };
        write!(f, "{}({})", to_char(self.top()), to_char(self.bottom()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerHiddenState {
    pub hand: Vec<OrientedCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PublicState {
    pub game_complete: bool,
    pub orientation_chosen: bool,
    pub is_player_one_turn: bool,

    pub board: Vec<OrientedCard>,

    pub player_one_card_count: u8,
    pub player_two_card_count: u8,

    pub player_one_scout_token_count: u8,
    pub player_two_scout_token_count: u8,

    pub player_one_won_cards: u8,
    pub player_two_won_cards: u8,

    pub action_history: Vec<(bool, Action, TransitionResult)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FlipHand {
    DoFlip,
    DoNotFlip,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PickedCard {
    // The first card as ordered on the board
    FirstCard,
    // The last card as ordered on the board
    LastCard,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    // false to keep current, true to flip
    ChooseOrientation(FlipHand),
    // start index (inclusive), end index (inclusive)
    PlayCards(u8, u8),
    // First or last card -> (index, orientation)
    PlayScoutToken((PickedCard, u8, Orientation)),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IllegalMoveReason {
    GameComplete,
    BadHandIndex,
    MustChooseOrientation,
    DoesNotBeatBoard,
    InvalidSet,
    NoScoutTokens,
    ScoutWhenBoardEmpty,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransitionResult {
    // Transition did occur, game state was updated
    MoveAccepted,
    GameComplete(i8, i8),

    // Transition did not occur, game state unchanged
    IllegalMove(IllegalMoveReason),
}

fn build_card_set(to_play: &[OrientedCard]) -> Option<CardSet> {
    let vals: Vec<u8> = to_play.iter().map(|c| c.top()).collect();
    if vals.len() == 0 {
        return None;
    }

    // Check for set of same numbers
    if vals.iter().all(|v| *v == vals[0]) {
        return Some(CardSet::Same(vals[0], vals.len() as u8));
    }

    // Check for consecutive numbers
    let ascending = vals[1] > vals[0];
    if !vals[..vals.len() - 1]
        .iter()
        .zip(vals[1..].iter())
        .all(|(a, b)| {
            if ascending {
                *a + 1 == *b
            } else {
                *a == *b + 1
            }
        })
    {
        return None;
    }
    let first = vals[0];
    let last = *vals.last().unwrap();
    if ascending {
        return Some(CardSet::Consecutive(first, last));
    } else {
        return Some(CardSet::Consecutive(last, first));
    }
}
/// Illegal set is checked before checking if the proposed play beats the board.
pub fn legal_and_beats_board(
    board: &[OrientedCard],
    proposed_play: &[OrientedCard],
) -> Option<IllegalMoveReason> {
    match (build_card_set(proposed_play), build_card_set(board)) {
        (Some(card_set), Some(board_set)) => {
            if card_set > board_set {
                None
            } else {
                Some(IllegalMoveReason::DoesNotBeatBoard)
            }
        }
        (Some(_), None) => None,
        _ => Some(IllegalMoveReason::InvalidSet),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GameState {
    pub public_state: PublicState,
    pub player_one_hidden_state: PlayerHiddenState,
    pub player_two_hidden_state: PlayerHiddenState,
}

impl GameState {
    pub fn new_from_hands(player_one_hand: &[OrientedCard], player_two_hand: &[OrientedCard], scout_tokens: u8) -> Self {
        let player_one_hidden_state = PlayerHiddenState {
            hand: player_one_hand.to_vec(),
        };
        let player_two_hidden_state = PlayerHiddenState {
            hand: player_two_hand.to_vec(),
        };

        debug_assert_eq!(player_one_hidden_state.hand.len(), player_two_hidden_state.hand.len(), "Expected same card counts for both players");

        let public_state = PublicState {
            game_complete: false,
            orientation_chosen: false,
            is_player_one_turn: true,
            board: vec![],
            player_one_card_count: player_one_hidden_state.hand.len() as u8,
            player_two_card_count: player_two_hidden_state.hand.len() as u8,

            player_one_won_cards: 0,
            player_two_won_cards: 0,

            player_one_scout_token_count: scout_tokens,
            player_two_scout_token_count: scout_tokens,

            action_history: vec![],
        };

        GameState {
            public_state,
            player_one_hidden_state,
            player_two_hidden_state,
        }
    }

    pub fn new_from_seed(max_card_num: u8, scout_tokens: u8, seed: u64) -> Self {
        // If max_card_num is too high then u8 could overflow
        // 40 is an abritrary limit, the game itself plays up to 10
        debug_assert!(max_card_num < 40);

        let mut deck = build_deck(max_card_num);
        let shuffled_deck = shuffle_deck(&mut deck, seed);

        let cards_per_player = deck.len() / 4;

        let player_one_hidden_state = PlayerHiddenState {
            hand: shuffled_deck[0..cards_per_player].to_vec(),
        };
        let player_two_hidden_state = PlayerHiddenState {
            hand: shuffled_deck[cards_per_player..cards_per_player * 2].to_vec(),
        };

        debug_assert_eq!(player_one_hidden_state.hand.len(), cards_per_player);
        debug_assert_eq!(player_two_hidden_state.hand.len(), cards_per_player);

        let public_state = PublicState {
            game_complete: false,
            orientation_chosen: false,
            is_player_one_turn: true,
            board: vec![],
            player_one_card_count: cards_per_player as u8,
            player_two_card_count: cards_per_player as u8,

            player_one_won_cards: 0,
            player_two_won_cards: 0,

            player_one_scout_token_count: scout_tokens,
            player_two_scout_token_count: scout_tokens,

            action_history: vec![],
        };

        GameState {
            public_state,
            player_one_hidden_state,
            player_two_hidden_state,
        }
    }

    fn handle_orientation_action(&mut self, do_flip: &FlipHand) -> TransitionResult {
        if self.public_state.is_player_one_turn {
            match *do_flip {
                FlipHand::DoFlip => {
                    self.player_one_hidden_state.hand = self
                        .player_one_hidden_state
                        .hand
                        .iter()
                        .map(|c| c.flip())
                        .collect();
                }
                FlipHand::DoNotFlip => {}
            }
            self.public_state.is_player_one_turn = false;
        } else {
            match *do_flip {
                FlipHand::DoFlip => {
                    self.player_two_hidden_state.hand = self
                        .player_two_hidden_state
                        .hand
                        .iter()
                        .map(|c| c.flip())
                        .collect();
                }
                FlipHand::DoNotFlip => {}
            }
            self.public_state.is_player_one_turn = true;
            self.public_state.orientation_chosen = true;
        }
        TransitionResult::MoveAccepted
    }

    fn accept_or_complete(&self) -> TransitionResult {
        if self.public_state.player_one_card_count == 0 {
            self.build_game_complete(true)
        } else if self.public_state.player_two_card_count == 0 {
            self.build_game_complete(false)
        } else if self.public_state.is_player_one_turn
            && self.public_state.player_one_scout_token_count == 0
            && !self.has_legal_play(true)
        {
            self.build_game_complete(false)
        } else if !self.public_state.is_player_one_turn
            && self.public_state.player_two_scout_token_count == 0
            && !self.has_legal_play(false)
        {
            self.build_game_complete(true)
        } else {
            TransitionResult::MoveAccepted
        }
    }

    fn build_game_complete(&self, player_one_scores: bool) -> TransitionResult {
        if player_one_scores {
            TransitionResult::GameComplete(
                self.public_state.player_one_won_cards as i8
                    + self.public_state.player_one_scout_token_count as i8,
                self.public_state.player_two_won_cards as i8
                    - self.public_state.player_two_card_count as i8
                    + self.public_state.player_two_scout_token_count as i8,
            )
        } else {
            TransitionResult::GameComplete(
                self.public_state.player_one_won_cards as i8
                    - self.public_state.player_one_card_count as i8
                    + self.public_state.player_one_scout_token_count as i8,
                self.public_state.player_two_won_cards as i8
                    + self.public_state.player_two_scout_token_count as i8,
            )
        }
    }

    /// Handles a PlayCards action
    fn handle_play_card_action(self: &mut Self, start_idx: &u8, end_idx: &u8) -> TransitionResult {
        if !self.public_state.orientation_chosen {
            return TransitionResult::IllegalMove(IllegalMoveReason::MustChooseOrientation);
        }
        if start_idx >= end_idx {
            return TransitionResult::IllegalMove(IllegalMoveReason::BadHandIndex);
        }
        let start_idx_u = *start_idx as usize;
        let end_idx_u = *end_idx as usize;
        let hand;
        if self.public_state.is_player_one_turn {
            hand = &self.player_one_hidden_state.hand;
        } else {
            hand = &self.player_two_hidden_state.hand;
        }
        if end_idx_u > hand.len() {
            return TransitionResult::IllegalMove(IllegalMoveReason::BadHandIndex);
        }

        let proposed_play = &hand[start_idx_u..end_idx_u];
        if let Some(illegal_move) = legal_and_beats_board(&self.public_state.board, proposed_play) {
            return TransitionResult::IllegalMove(illegal_move);
        }

        let board_cards = self.public_state.board.iter().map(|c| c.card);

        if self.public_state.is_player_one_turn {
            self.public_state.player_one_card_count -= proposed_play.len() as u8;
            self.public_state.player_one_won_cards += board_cards.len() as u8;
            self.public_state.board = proposed_play.to_vec();
            self.player_one_hidden_state
                .hand
                .drain(start_idx_u..end_idx_u);
            self.public_state.is_player_one_turn = false;
        } else {
            self.public_state.player_two_card_count -= proposed_play.len() as u8;
            self.public_state.player_two_won_cards += board_cards.len() as u8;
            self.public_state.board = proposed_play.to_vec();
            self.player_two_hidden_state
                .hand
                .drain(start_idx_u..end_idx_u);
            self.public_state.is_player_one_turn = true;
        }

        self.accept_or_complete()
    }

    fn has_legal_play(self: &Self, check_player_one: bool) -> bool {
        let hand = if check_player_one {
            &self.player_one_hidden_state.hand
        } else {
            &self.player_two_hidden_state.hand
        };

        (1..=hand.len()).any(|window_size| {
            hand.windows(window_size)
                .any(|window| legal_and_beats_board(&self.public_state.board, window).is_none())
        })
    }

    fn handle_play_scout_token(
        self: &mut Self,
        picked_card_info: &(PickedCard, u8, Orientation),
    ) -> TransitionResult {
        if !self.public_state.orientation_chosen {
            return TransitionResult::IllegalMove(IllegalMoveReason::MustChooseOrientation);
        }
        let picked_card = &picked_card_info.0;
        let insertion_index = picked_card_info.1;
        let orientation = &picked_card_info.2;

        let hand;
        if self.public_state.is_player_one_turn {
            if self.public_state.player_one_scout_token_count == 0 {
                return TransitionResult::IllegalMove(IllegalMoveReason::NoScoutTokens);
            }
            hand = &mut self.player_one_hidden_state.hand;
        } else {
            if self.public_state.player_two_scout_token_count == 0 {
                return TransitionResult::IllegalMove(IllegalMoveReason::NoScoutTokens);
            }
            hand = &mut self.player_two_hidden_state.hand;
        }

        if insertion_index as usize > hand.len() {
            return TransitionResult::IllegalMove(IllegalMoveReason::BadHandIndex);
        } else if self.public_state.board.len() == 0 {
            return TransitionResult::IllegalMove(IllegalMoveReason::ScoutWhenBoardEmpty);
        }

        let oriented_card;
        match picked_card {
            PickedCard::FirstCard => {
                // remove first element of board
                oriented_card = self.public_state.board.remove(0);
            }
            PickedCard::LastCard => {
                oriented_card = self.public_state.board.pop().unwrap();
            }
        }

        hand.insert(
            insertion_index as usize,
            OrientedCard {
                card: oriented_card.card,
                orientation: *orientation,
            },
        );
        if self.public_state.is_player_one_turn {
            self.public_state.player_one_scout_token_count -= 1;
            self.public_state.player_one_card_count += 1;
        } else {
            self.public_state.player_two_scout_token_count -= 1;
            self.public_state.player_two_card_count += 1;
        }
        self.accept_or_complete()
    }

    pub fn transition(self: &mut Self, action: &Action) -> TransitionResult {
        if self.public_state.game_complete {
            return TransitionResult::IllegalMove(IllegalMoveReason::GameComplete);
        }

        // Three choices for the enum
        let result = match action {
            Action::ChooseOrientation(do_flip) => self.handle_orientation_action(do_flip),
            Action::PlayCards(start_idx, end_idx) => {
                self.handle_play_card_action(start_idx, end_idx)
            }
            Action::PlayScoutToken(picked_card_info) => {
                self.handle_play_scout_token(picked_card_info)
            }
        };

        match result {
            TransitionResult::GameComplete(..) => {
                self.public_state.game_complete = true;
                self.public_state.action_history.push((
                    self.public_state.is_player_one_turn,
                    action.clone(),
                    result.clone(),
                ));
            }
            TransitionResult::MoveAccepted => {
                self.public_state.action_history.push((
                    self.public_state.is_player_one_turn,
                    action.clone(),
                    result.clone(),
                ));
            }
            _ => {}
        }

        result
    }

    pub fn calculate_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    pub fn display(&self) -> () {
        let hash = self.calculate_hash();
        println!("## State Hash: {:?}", hash);
        if !self.public_state.orientation_chosen {
            if self.public_state.is_player_one_turn {
                println!("player_one choosing hand orientation");
            } else {
                println!("player_two choosing hand orientation");
            }
            return;
        }

        if self.public_state.game_complete {
            println!("--Game Complete--");
        } else {
            print!("--Turn: ");
            if self.public_state.is_player_one_turn {
                println!("player_one--");
            } else {
                println!("player_two--");
            }
        }

        print!(
            "Player One: [Tokens {:?}] [Won {:?}] [Hand: ",
            self.public_state.player_one_scout_token_count, self.public_state.player_one_won_cards
        );
        print_cards(&self.player_one_hidden_state.hand);
        println!("]");

        print!(
            "Player Two: [Tokens {:?}] [Won {:?}] [Hand: ",
            self.public_state.player_two_scout_token_count, self.public_state.player_two_won_cards
        );
        print_cards(&self.player_two_hidden_state.hand);
        println!("]");

        print!("Board: ");
        for card in &self.public_state.board {
            if card != self.public_state.board.last().unwrap() {
                print!(" {} ", card);
            } else {
                print!(" {}", card);
            }
        }
        println!("");
    }
}

impl GameState {
    fn play_and_display(&mut self, action: &Action, ensure_legal: bool) -> TransitionResult {
        let result = self.transition(action);
        self.display();
        if ensure_legal {
            assert!(
                !matches!(result, TransitionResult::IllegalMove(..)),
                "{:?}",
                result
            );
        }
        result
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
        assert_eq!(7, card.first);
        assert_eq!(11, card.second);
        assert_eq!(card, card);
    }

    #[test]
    fn test_build_deck() {
        let deck = build_deck(4);
        assert_eq!(4, deck.len());

        let deck = build_deck(10);
        assert_eq!(44, deck.len());
    }

    #[test]
    fn test_shuffle_deck() {
        let orig = build_deck(4);
        let mut deck1 = orig.clone();
        let mut deck2 = orig.clone();
        let mut deck3 = orig.clone();

        shuffle_deck(&mut deck1, 0u64);
        shuffle_deck(&mut deck2, 0u64);
        assert_eq!(deck2, deck1);
        assert_ne!(deck1, orig);

        shuffle_deck(&mut deck3, 1u64);
        assert_ne!(deck3, deck1);
        assert_ne!(deck3, orig);
    }

    #[test]
    fn test_choose_orientation() {
        let mut state = GameState::new_from_seed(10, 3, 2);
        assert_eq!(true, state.public_state.is_player_one_turn);
        assert_eq!(false, state.public_state.orientation_chosen);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        assert_eq!(false, state.public_state.is_player_one_turn);
        assert_eq!(false, state.public_state.orientation_chosen);
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        assert_eq!(true, state.public_state.is_player_one_turn);
        assert_eq!(true, state.public_state.orientation_chosen);
    }
    #[test]
    fn test_play_illegal_cards() {
        let mut state = GameState::new_from_seed(10, 3, 2);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.display();

        let cards_per_player = state.public_state.player_one_card_count;

        assert_eq!(true, state.public_state.is_player_one_turn);

        let result = state.transition(&Action::PlayCards(0, 3));
        assert!(matches!(result, TransitionResult::IllegalMove(_)));
        assert_eq!(true, state.public_state.is_player_one_turn);

        let result = state.transition(&Action::PlayCards(100, 0));
        assert!(matches!(result, TransitionResult::IllegalMove(_)));
        assert_eq!(true, state.public_state.is_player_one_turn);

        let result = state.transition(&Action::PlayCards(1, 1));
        assert!(matches!(result, TransitionResult::IllegalMove(_)));
        assert_eq!(true, state.public_state.is_player_one_turn);

        let result = state.transition(&Action::PlayCards(1, 0));
        assert!(matches!(result, TransitionResult::IllegalMove(_)));
        assert_eq!(true, state.public_state.is_player_one_turn);

        let result = state.transition(&Action::PlayCards(cards_per_player, cards_per_player + 1));
        assert!(matches!(result, TransitionResult::IllegalMove(_)));
        assert_eq!(true, state.public_state.is_player_one_turn);
    }

    #[test]
    fn test_play_same_pair() {
        let mut state = GameState::new_from_seed(10, 3, 2);
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.display();

        let played = state.player_one_hidden_state.hand[0..2].to_vec();
        let result = state.transition(&Action::PlayCards(0, 2));
        state.display();

        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(false, state.public_state.is_player_one_turn);
        assert_eq!(played, state.public_state.board);
    }

    #[test]
    fn test_play_single() {
        let mut state = GameState::new_from_seed(10, 3, 2);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.display();

        let played = state.player_one_hidden_state.hand[0..1].to_vec();
        let result = state.transition(&Action::PlayCards(0, 1));
        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(false, state.public_state.is_player_one_turn);
        assert_eq!(played, state.public_state.board);
    }

    #[test]
    fn test_compare_sets() {
        let set1 = CardSet::Consecutive(1, 3);
        let set2 = CardSet::Consecutive(1, 4);
        assert!(set1 < set2);

        let set1 = CardSet::Consecutive(1, 3);
        let set2 = CardSet::Consecutive(1, 3);
        assert!(set1 == set2);

        let set1 = CardSet::Consecutive(1, 3);
        let set2 = CardSet::Consecutive(1, 2);
        assert!(set1 > set2);

        let set1 = CardSet::Same(3, 1);
        let set2 = CardSet::Same(3, 2);
        assert!(set1 < set2);

        let set1 = CardSet::Same(3, 1);
        let set2 = CardSet::Same(3, 1);
        assert!(set1 == set2);

        let set1 = CardSet::Same(3, 2);
        let set2 = CardSet::Same(3, 1);
        assert!(set1 > set2);
    }

    #[test]
    fn test_both_players_act() {
        let mut state = GameState::new_from_seed(10, 3, 2);
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.display();
        println!();
        assert_eq!(2, state.player_one_hidden_state.hand[0].top());

        // player_one plays a 2
        let played = state.player_one_hidden_state.hand[0..1].to_vec();
        let result = state.transition(&Action::PlayCards(0, 1));
        state.display();
        println!();
        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(false, state.public_state.is_player_one_turn);
        assert_eq!(played, state.public_state.board);

        // player_two plays a 6
        let played = state.player_two_hidden_state.hand[0..1].to_vec();
        let result = state.transition(&Action::PlayCards(0, 1));
        state.display();
        println!();
        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(true, state.public_state.is_player_one_turn);
        assert_eq!(played, state.public_state.board);
    }

    #[test]
    fn test_oriented() {
        let oc = OrientedCard {
            card: Card {
                first: 1,
                second: 2,
            },
            orientation: Orientation::Larger,
        };

        assert_eq!(2, oc.top());
        assert_eq!(1, oc.bottom());

        let flipped = oc.flip();
        assert_eq!(1, flipped.top());
        assert_eq!(2, flipped.bottom());
    }

    #[test]
    fn test_build_card_set() {
        let oc1 = OrientedCard {
            card: Card {
                first: 1,
                second: 2,
            },
            orientation: Orientation::Larger,
        };
        let card_set1 = build_card_set(&vec![oc1]);
        assert_eq!(Some(CardSet::Same(2, 1)), card_set1);

        let oc2 = OrientedCard {
            card: Card {
                first: 3,
                second: 4,
            },
            orientation: Orientation::Smaller,
        };
        let card_set2 = build_card_set(&vec![oc2]);
        assert_eq!(Some(CardSet::Same(3, 1)), card_set2);

        let card_set3 = build_card_set(&vec![oc1, oc2]);
        assert_eq!(Some(CardSet::Consecutive(2, 3)), card_set3);

        assert!(card_set2 > card_set1);
        assert!(card_set3 > card_set2);
        assert!(card_set3 > card_set1);
    }

    #[test]
    fn test_no_orient() {
        let mut state = GameState::new_from_seed(10, 3, 2);
        let result = state.transition(&Action::PlayCards(0, 1));
        assert_eq!(
            TransitionResult::IllegalMove(IllegalMoveReason::MustChooseOrientation),
            result
        );
        let result = state.transition(&Action::PlayScoutToken((
            PickedCard::FirstCard,
            0,
            Orientation::Larger,
        )));
        assert_eq!(
            TransitionResult::IllegalMove(IllegalMoveReason::MustChooseOrientation),
            result
        );
        state.display();
        println!();
    }

    #[test]
    fn test_scout() {
        let mut state = GameState::new_from_seed(10, 3, 2);
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.display();
        println!();

        let result = state.transition(&Action::PlayCards(0, 1));
        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(2, state.public_state.board[0].top());

        let result = state.transition(&Action::PlayScoutToken((
            PickedCard::FirstCard,
            0u8,
            Orientation::Larger,
        )));
        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(0, state.public_state.board.len());
        assert_eq!(12, state.public_state.player_two_card_count);
        assert_eq!(8, state.player_two_hidden_state.hand[0].top());
    }
    #[test]
    fn test_bad_scout() {
        let mut state = GameState::new_from_seed(10, 3, 2);
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.display();
        println!();

        let result = state.transition(&Action::PlayScoutToken((
            PickedCard::FirstCard,
            0u8,
            Orientation::Larger,
        )));
        assert_eq!(
            TransitionResult::IllegalMove(IllegalMoveReason::ScoutWhenBoardEmpty),
            result
        );

        state.transition(&Action::PlayCards(0, 1));
        let result = state.transition(&Action::PlayScoutToken((
            PickedCard::FirstCard,
            0u8,
            Orientation::Larger,
        )));
        assert_eq!(TransitionResult::MoveAccepted, result);
        let result = state.transition(&Action::PlayScoutToken((
            PickedCard::FirstCard,
            0u8,
            Orientation::Larger,
        )));
        assert_eq!(
            TransitionResult::IllegalMove(IllegalMoveReason::ScoutWhenBoardEmpty),
            result
        );
        state.transition(&Action::PlayCards(0, 1));
        // player_two's Turn
        let result = state.transition(&Action::PlayCards(3, 6));
        state.display();
        assert_eq!(TransitionResult::MoveAccepted, result);

        let result = state.transition(&Action::PlayScoutToken((
            PickedCard::FirstCard,
            0u8,
            Orientation::Larger,
        )));
        assert_eq!(TransitionResult::MoveAccepted, result);
        let result = state.transition(&Action::PlayScoutToken((
            PickedCard::FirstCard,
            0u8,
            Orientation::Larger,
        )));
        assert_eq!(TransitionResult::MoveAccepted, result);
        let result = state.transition(&Action::PlayScoutToken((
            PickedCard::FirstCard,
            0u8,
            Orientation::Larger,
        )));
        assert_eq!(
            TransitionResult::IllegalMove(IllegalMoveReason::NoScoutTokens),
            result
        );
        assert_eq!(0, state.public_state.player_two_scout_token_count);
        assert_eq!(13, state.public_state.player_two_card_count);
    }
    #[test]
    fn test_illegal_move_reason() {
        let mut state = GameState::new_from_seed(10, 3, 3);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.display();

        // Play a 3 card set
        state.transition(&Action::PlayCards(4, 7));
        state.display();

        // Play an illegal 2 card set
        // This certainly can't beat the board, but we want to make sure
        // InvalidSet is returned not DoesNotBeatBoard
        let result = state.transition(&Action::PlayCards(0, 2));
        match result {
            TransitionResult::IllegalMove(reason) => {
                assert_eq!(IllegalMoveReason::InvalidSet, reason);
            }
            _ => {
                assert!(false);
            }
        }
    }

    #[test]
    fn test_won_cards() {
        let mut state = GameState::new_from_seed(10, 3, 3);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.display();

        state.transition(&Action::PlayCards(0, 1));
        state.display();

        state.transition(&Action::PlayCards(1, 2));
        state.display();
        assert_eq!(true, state.public_state.is_player_one_turn);
        assert_eq!(1, state.public_state.player_two_won_cards);
        assert_eq!(0, state.public_state.player_one_won_cards);

        state.transition(&Action::PlayCards(3, 6));
        assert_eq!(false, state.public_state.is_player_one_turn);
        state.display();

        assert_eq!(1, state.public_state.player_two_won_cards);
        assert_eq!(1, state.public_state.player_one_won_cards);
        let result = state.transition(&Action::PlayScoutToken((
            PickedCard::FirstCard,
            2,
            Orientation::Larger,
        )));
        state.display();
        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(false, state.public_state.is_player_one_turn);
        assert_eq!(1, state.public_state.player_two_won_cards);
        assert_eq!(1, state.public_state.player_one_won_cards);

        let result = state.transition(&Action::PlayCards(2, 4));
        state.display();
        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(true, state.public_state.is_player_one_turn);
        assert_eq!(3, state.public_state.player_two_won_cards);
        assert_eq!(1, state.public_state.player_one_won_cards);
    }

    #[test]
    fn test_game_end() {
        let mut state = GameState::new_from_seed(6, 3, 3);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.display();

        state.transition(&Action::PlayCards(0, 1));
        state.display();
        state.transition(&Action::PlayCards(1, 3));
        state.display();
        state.transition(&Action::PlayScoutToken((
            PickedCard::LastCard,
            0,
            Orientation::Smaller,
        )));
        state.display();
        let result = state.transition(&Action::PlayCards(0, 3));
        state.display();
        // player_one: 1 won card + 2 tokens
        // player_two: 1 won card - 1 card in hand + 3 tokens
        assert_eq!(TransitionResult::GameComplete(3, 3), result);
    }

    #[test]
    fn test_has_legal_play() {
        let mut state = GameState::new_from_seed(6, 0, 3);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.display();

        state.transition(&Action::PlayCards(1, 3));
        state.display();
        assert_eq!(true, state.has_legal_play(false));

        state.transition(&Action::PlayCards(1, 3));
        state.display();
        assert_eq!(false, state.has_legal_play(true));
    }

    #[test]
    fn test_legal_and_beats_board() {
        let mut state = GameState::new_from_seed(6, 0, 3);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.display();
        state.transition(&Action::PlayCards(1, 3));
        state.display();

        let proposed_play = state.player_two_hidden_state.hand[1..3].to_vec();
        let result = legal_and_beats_board(&state.public_state.board, &proposed_play);
        assert_eq!(None, result);

        let proposed_play = state.player_two_hidden_state.hand[1..2].to_vec();
        let result = legal_and_beats_board(&state.public_state.board, &proposed_play);
        assert_eq!(Some(IllegalMoveReason::DoesNotBeatBoard), result);

        let proposed_play = state.player_two_hidden_state.hand[0..2].to_vec();
        print!("Proposed play: ");
        print_cards(proposed_play.as_slice());
        println!();
        assert_eq!(None, build_card_set(&proposed_play));
        let result = legal_and_beats_board(&state.public_state.board, &proposed_play);
        assert_eq!(Some(IllegalMoveReason::InvalidSet), result);
    }

    #[test]
    fn test_game_end2() {
        let mut state = GameState::new_from_seed(10, 3, 1234);
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.display();

        state.play_and_display(&Action::PlayCards(5, 6), true);
        state.play_and_display(&Action::PlayCards(3, 4), true);
        state.play_and_display(
            &Action::PlayScoutToken((PickedCard::LastCard, 2, Orientation::Larger)),
            true,
        );
        state.play_and_display(&Action::PlayCards(8, 9), true);
        state.play_and_display(&Action::PlayCards(2, 3), true);
        state.play_and_display(&Action::PlayCards(8, 10), true);
        state.play_and_display(&Action::PlayCards(5, 7), true);
        state.play_and_display(
            &Action::PlayScoutToken((PickedCard::LastCard, 3, Orientation::Larger)),
            true,
        );
        state.play_and_display(&Action::PlayCards(8, 9), true);
        state.play_and_display(
            &Action::PlayScoutToken((PickedCard::LastCard, 2, Orientation::Smaller)),
            true,
        );
        state.play_and_display(&Action::PlayCards(6, 8), true);
        let old_won = state.public_state.player_one_won_cards;
        state.play_and_display(&Action::PlayCards(6, 8), true);
        let new_won = state.public_state.player_one_won_cards;
        assert_eq!(old_won + 2, new_won);

        state.play_and_display(&Action::PlayCards(4, 6), true);
        state.play_and_display(&Action::PlayCards(0, 3), true);
        let result = state.play_and_display(&Action::PlayCards(0, 4), true);
        // player_one: 1 won card + 2 tokens
        // player_two: 1 won card - 1 card in hand + 3 tokens
        assert_eq!(TransitionResult::GameComplete(4, 11), result);
        assert_eq!(true, state.public_state.game_complete);
    }

    #[test]
    fn test_cant_play_past_end() {
        let mut state = GameState::new_from_seed(6, 0, 5);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.display();

        state.play_and_display(&Action::PlayCards(2, 3), true);
        state.play_and_display(&Action::PlayCards(1, 2), true);
        assert!(!state.public_state.game_complete);

        // Assert result is game end
        let result = state.play_and_display(&Action::PlayCards(0, 2), true);
        assert!(matches!(result, TransitionResult::GameComplete(_, _)));
        assert!(state.public_state.game_complete);
        assert_eq!(5, state.public_state.action_history.len());

        let result = state.play_and_display(&Action::PlayCards(0, 2), false);
        assert!(matches!(result, TransitionResult::IllegalMove(..)));
        assert!(state.public_state.game_complete);
        assert!(matches!(
            state.public_state.action_history.last().unwrap().2,
            TransitionResult::GameComplete(_, _)
        ));
        assert_eq!(5, state.public_state.action_history.len());
    }
}
