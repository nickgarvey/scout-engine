use std::{fmt, vec};

use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_xoshiro::SplitMix64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card {
    first: u8,
    second: u8,
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
                Orientation::Larger
            } else {
                Orientation::Smaller
            },
        })
        .collect()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Orientation {
    Larger,
    Smaller,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OrientedCard {
    card: Card,
    orientation: Orientation,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerHiddenState {
    hand: Vec<OrientedCard>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicState {
    game_complete: bool,

    orientation_chosen: bool,
    is_hero_turn: bool,
    board: Vec<OrientedCard>,

    hero_card_count: u8,
    villian_card_count: u8,

    hero_scout_token_count: u8,
    villian_scout_token_count: u8,

    hero_won_cards: Vec<Card>,
    villian_won_cards: Vec<Card>,

    hero_scouted_cards: Vec<Card>,
    villian_scouted_cards: Vec<Card>,

    action_history: Vec<(Action, TransitionResult)>,
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
pub enum IllegalMoveReason {
    GameComplete,
    BadHandIndex,
    MustChooseOrientation,
    DoesNotBeatBoard,
    InvalidSet,
    NoScoutTokens,
    ScoutWhenBoardEmpty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    seed: u64,
    public_state: PublicState,
    hero_hidden_state: PlayerHiddenState,
    villian_hidden_state: PlayerHiddenState,
}

impl GameState {
    pub fn new(max_card_num: u8, scout_tokens: u8, seed: u64) -> Self {
        // If max_card_num is too high then u8 could overflow
        // 40 is an abritrary limit, the game itself plays up to 10
        debug_assert!(max_card_num < 40);

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
            game_complete: false,
            orientation_chosen: false,
            is_hero_turn: true,
            board: vec![],
            hero_card_count: cards_per_player as u8,
            villian_card_count: cards_per_player as u8,

            hero_won_cards: vec![],
            villian_won_cards: vec![],

            hero_scouted_cards: vec![],
            villian_scouted_cards: vec![],

            hero_scout_token_count: scout_tokens,
            villian_scout_token_count: scout_tokens,

            action_history: vec![],
        };

        GameState {
            seed,
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
                    self.villian_hidden_state.hand = self
                        .villian_hidden_state
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

    fn build_game_complete(&self, hero_scores: bool) -> TransitionResult {
        if hero_scores {
            TransitionResult::GameComplete(
                self.public_state.hero_won_cards.len() as i8
                    + self.public_state.hero_scout_token_count as i8,
                self.public_state.villian_won_cards.len() as i8
                    - self.public_state.villian_card_count as i8
                    + self.public_state.villian_scout_token_count as i8,
            )
        } else {
            TransitionResult::GameComplete(
                self.public_state.hero_won_cards.len() as i8
                    - self.public_state.hero_card_count as i8
                    + self.public_state.hero_scout_token_count as i8,
                self.public_state.villian_won_cards.len() as i8
                    + self.public_state.villian_scout_token_count as i8,
            )
        }
    }

    /// Illegal set is checked before checking if the proposed play beats the board.
    pub fn legal_and_beats_board(
        &self,
        proposed_play: &[OrientedCard],
    ) -> Option<IllegalMoveReason> {
        match (
            build_card_set(proposed_play),
            build_card_set(&self.public_state.board),
        ) {
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
        if self.public_state.is_hero_turn {
            hand = &self.hero_hidden_state.hand;
        } else {
            hand = &self.villian_hidden_state.hand;
        }
        if end_idx_u > hand.len() {
            return TransitionResult::IllegalMove(IllegalMoveReason::BadHandIndex);
        }

        let proposed_play = &hand[start_idx_u..end_idx_u];
        if let Some(illegal_move) = self.legal_and_beats_board(proposed_play) {
            return TransitionResult::IllegalMove(illegal_move);
        }

        let board_cards = self.public_state.board.iter().map(|c| c.card);

        if self.public_state.is_hero_turn {
            self.public_state.hero_card_count -= proposed_play.len() as u8;
            self.public_state.hero_won_cards.extend(board_cards);
            self.public_state.board = proposed_play.to_vec();
            self.hero_hidden_state.hand.drain(start_idx_u..end_idx_u);
            self.public_state.is_hero_turn = false;
            if self.public_state.hero_card_count == 0 {
                self.build_game_complete(true)
            } else if self.public_state.villian_scout_token_count == 0
                && !self.has_legal_play(false)
            {
                self.build_game_complete(true)
            } else {
                TransitionResult::MoveAccepted
            }
        } else {
            self.public_state.villian_card_count -= proposed_play.len() as u8;
            self.public_state.villian_won_cards.extend(board_cards);
            self.public_state.board = proposed_play.to_vec();
            self.villian_hidden_state.hand.drain(start_idx_u..end_idx_u);
            self.public_state.is_hero_turn = true;
            if self.public_state.villian_card_count == 0 {
                self.build_game_complete(false)
            } else if self.public_state.hero_scout_token_count == 0 && !self.has_legal_play(true) {
                self.build_game_complete(false)
            } else {
                TransitionResult::MoveAccepted
            }
        }
    }

    fn has_legal_play(self: &Self, check_hero: bool) -> bool {
        let hand = if check_hero {
            &self.hero_hidden_state.hand
        } else {
            &self.villian_hidden_state.hand
        };

        (1..=hand.len()).any(|window_size| {
            hand.windows(window_size)
                .any(|window| self.legal_and_beats_board(window).is_none())
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
        if self.public_state.is_hero_turn {
            if self.public_state.hero_scout_token_count == 0 {
                return TransitionResult::IllegalMove(IllegalMoveReason::NoScoutTokens);
            }
            hand = &mut self.hero_hidden_state.hand;
        } else {
            if self.public_state.villian_scout_token_count == 0 {
                return TransitionResult::IllegalMove(IllegalMoveReason::NoScoutTokens);
            }
            hand = &mut self.villian_hidden_state.hand;
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
        if self.public_state.is_hero_turn {
            self.public_state.hero_scout_token_count -= 1;
            self.public_state.hero_card_count += 1;
        } else {
            self.public_state.villian_scout_token_count -= 1;
            self.public_state.villian_card_count += 1;
        }

        TransitionResult::MoveAccepted
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
                self.public_state
                    .action_history
                    .push((action.clone(), result.clone()));
            }
            TransitionResult::MoveAccepted => {
                self.public_state
                    .action_history
                    .push((action.clone(), result.clone()));
            }
            _ => {}
        }

        result
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

        if self.public_state.game_complete {
            println!("--Game Complete--");
        } else {
            print!("--Turn: ");
            if self.public_state.is_hero_turn {
                println!("Hero--");
            } else {
                println!("Villian--");
            }
        }

        print!(
            "Hero:    [Tokens {:?}] [Hand: ",
            self.public_state.hero_scout_token_count
        );
        print_cards(&self.hero_hidden_state.hand);
        print!("] [Won: ");
        print_cards(&self.public_state.hero_won_cards);
        println!("]");

        print!(
            "Villian: [Tokens {:?}] [Hand: ",
            self.public_state.villian_scout_token_count
        );
        print_cards(&self.villian_hidden_state.hand);
        print!("] [Won: ");
        print_cards(&self.public_state.villian_won_cards);
        println!("]");

        print!("Board:  ");
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
        let mut state = GameState::new(10, 3, 2);
        assert_eq!(true, state.public_state.is_hero_turn);
        assert_eq!(false, state.public_state.orientation_chosen);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        assert_eq!(false, state.public_state.is_hero_turn);
        assert_eq!(false, state.public_state.orientation_chosen);
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        assert_eq!(true, state.public_state.is_hero_turn);
        assert_eq!(true, state.public_state.orientation_chosen);
    }
    #[test]
    fn test_play_illegal_cards() {
        let mut state = GameState::new(10, 3, 2);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.display();

        let cards_per_player = state.public_state.hero_card_count;

        assert_eq!(true, state.public_state.is_hero_turn);

        let result = state.transition(&Action::PlayCards(0, 3));
        assert!(matches!(result, TransitionResult::IllegalMove(_)));
        assert_eq!(true, state.public_state.is_hero_turn);

        let result = state.transition(&Action::PlayCards(100, 0));
        assert!(matches!(result, TransitionResult::IllegalMove(_)));
        assert_eq!(true, state.public_state.is_hero_turn);

        let result = state.transition(&Action::PlayCards(1, 1));
        assert!(matches!(result, TransitionResult::IllegalMove(_)));
        assert_eq!(true, state.public_state.is_hero_turn);

        let result = state.transition(&Action::PlayCards(1, 0));
        assert!(matches!(result, TransitionResult::IllegalMove(_)));
        assert_eq!(true, state.public_state.is_hero_turn);

        let result = state.transition(&Action::PlayCards(cards_per_player, cards_per_player + 1));
        assert!(matches!(result, TransitionResult::IllegalMove(_)));
        assert_eq!(true, state.public_state.is_hero_turn);
    }

    #[test]
    fn test_play_same_pair() {
        let mut state = GameState::new(10, 3, 2);
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.display();

        let played = state.hero_hidden_state.hand[0..2].to_vec();
        let result = state.transition(&Action::PlayCards(0, 2));
        state.display();

        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(false, state.public_state.is_hero_turn);
        assert_eq!(played, state.public_state.board);
    }

    #[test]
    fn test_play_single() {
        let mut state = GameState::new(10, 3, 2);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.display();

        let played = state.hero_hidden_state.hand[0..1].to_vec();
        let result = state.transition(&Action::PlayCards(0, 1));
        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(false, state.public_state.is_hero_turn);
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
        let mut state = GameState::new(10, 3, 2);
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.display();
        println!();
        assert_eq!(2, state.hero_hidden_state.hand[0].top());

        // Hero plays a 2
        let played = state.hero_hidden_state.hand[0..1].to_vec();
        let result = state.transition(&Action::PlayCards(0, 1));
        state.display();
        println!();
        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(false, state.public_state.is_hero_turn);
        assert_eq!(played, state.public_state.board);

        // Villian plays a 6
        let played = state.villian_hidden_state.hand[0..1].to_vec();
        let result = state.transition(&Action::PlayCards(0, 1));
        state.display();
        println!();
        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(true, state.public_state.is_hero_turn);
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
        let mut state = GameState::new(10, 3, 2);
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
        let mut state = GameState::new(10, 3, 2);
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
        assert_eq!(12, state.public_state.villian_card_count);
        assert_eq!(8, state.villian_hidden_state.hand[0].top());
    }
    #[test]
    fn test_bad_scout() {
        let mut state = GameState::new(10, 3, 2);
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
        // Villian's Turn
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
        assert_eq!(0, state.public_state.villian_scout_token_count);
        assert_eq!(13, state.public_state.villian_card_count);
    }
    #[test]
    fn test_illegal_move_reason() {
        let mut state = GameState::new(10, 3, 3);
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
        let mut state = GameState::new(10, 3, 3);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.display();

        state.transition(&Action::PlayCards(0, 1));
        state.display();

        state.transition(&Action::PlayCards(1, 2));
        state.display();
        assert_eq!(true, state.public_state.is_hero_turn);
        assert_eq!(1, state.public_state.villian_won_cards.len());
        assert_eq!(0, state.public_state.hero_won_cards.len());

        state.transition(&Action::PlayCards(3, 6));
        assert_eq!(false, state.public_state.is_hero_turn);
        state.display();

        assert_eq!(1, state.public_state.villian_won_cards.len());
        assert_eq!(1, state.public_state.hero_won_cards.len());
        let result = state.transition(&Action::PlayScoutToken((
            PickedCard::FirstCard,
            2,
            Orientation::Larger,
        )));
        state.display();
        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(false, state.public_state.is_hero_turn);
        assert_eq!(1, state.public_state.villian_won_cards.len());
        assert_eq!(1, state.public_state.hero_won_cards.len());

        let result = state.transition(&Action::PlayCards(2, 4));
        state.display();
        assert_eq!(TransitionResult::MoveAccepted, result);
        assert_eq!(true, state.public_state.is_hero_turn);
        assert_eq!(3, state.public_state.villian_won_cards.len());
        assert_eq!(1, state.public_state.hero_won_cards.len());
    }

    #[test]
    fn test_game_end() {
        let mut state = GameState::new(6, 3, 3);
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
        // Hero: 1 won card + 2 tokens
        // Villian: 1 won card - 1 card in hand + 3 tokens
        assert_eq!(TransitionResult::GameComplete(3, 3), result);
    }

    #[test]
    fn test_has_legal_play() {
        let mut state = GameState::new(6, 0, 3);
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
        let mut state = GameState::new(6, 0, 3);
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.display();
        state.transition(&Action::PlayCards(1, 3));
        state.display();

        let proposed_play = state.villian_hidden_state.hand[1..3].to_vec();
        let result = state.legal_and_beats_board(&proposed_play);
        assert_eq!(None, result);

        let proposed_play = state.villian_hidden_state.hand[1..2].to_vec();
        let result = state.legal_and_beats_board(&proposed_play);
        assert_eq!(Some(IllegalMoveReason::DoesNotBeatBoard), result);

        let proposed_play = state.villian_hidden_state.hand[0..2].to_vec();
        print!("Proposed play: ");
        print_cards(proposed_play.as_slice());
        println!();
        assert_eq!(None, build_card_set(&proposed_play));
        let result = state.legal_and_beats_board(&proposed_play);
        assert_eq!(Some(IllegalMoveReason::InvalidSet), result);
    }

    #[test]
    fn test_game_end2() {
        let mut state = GameState::new(10, 3, 1234);
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
        let old_won = state.public_state.hero_won_cards.len();
        state.play_and_display(&Action::PlayCards(6, 8), true);
        let new_won = state.public_state.hero_won_cards.len();
        assert_eq!(old_won + 2, new_won);

        state.play_and_display(&Action::PlayCards(4, 6), true);
        state.play_and_display(&Action::PlayCards(0, 3), true);
        let result = state.play_and_display(&Action::PlayCards(0, 4), true);
        // Hero: 1 won card + 2 tokens
        // Villian: 1 won card - 1 card in hand + 3 tokens
        assert_eq!(TransitionResult::GameComplete(4, 11), result);
    }

    #[test]
    fn test_cant_play_past_end() {
        let mut state = GameState::new(6, 0, 5);
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
            state.public_state.action_history.last().unwrap().1,
            TransitionResult::GameComplete(_, _)
        ));
        assert_eq!(5, state.public_state.action_history.len());
    }
}
