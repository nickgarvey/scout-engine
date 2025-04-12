use core::panic;

use crate::engine::{
    legal_and_beats_board, Action, GameState, PlayerHiddenState, PublicState, TransitionResult,
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
    let seed = state.seed;
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
                panic!(
                    "Illegal move ({:?}) (seed:{:?}): {:?}",
                    reason, seed, action
                );
            }
            _ => {
                walk_games(new_state, walker);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_iter_orientations() {
        let state = GameState::new(4, 0, 123);
        let move_iter = MoveIter::new(&state.public_state, &state.player_one_hidden_state);
        assert_eq!(move_iter.count(), 2);
    }

    #[test]
    fn test_walker_small() {
        let state = GameState::new(4, 0, 123);
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
        let state = GameState::new(6, 1, 123);
        let mut count = 0;
        let mut count_fn = |state: GameState| {
            count += 1;
        };
        walk_games(state, &mut count_fn);
        // total games depends on the orientation each player picks. there are no choices after that.
        // so 4 games total.
        assert_eq!(count, 4040);
    }
}
