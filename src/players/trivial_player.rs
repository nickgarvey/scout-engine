use crate::engine::{self, OrientedCard};
use crate::players::player::Player;

struct MoveIter<'a> {
    hand: &'a [OrientedCard],
    board: &'a [OrientedCard],
    start_idx: usize,
    end_idx: usize,
}

impl<'a> MoveIter<'a> {
    fn new(hand: &'a [OrientedCard], board: &'a [OrientedCard]) -> MoveIter<'a> {
        MoveIter {
            hand,
            board,
            start_idx: 0,
            end_idx: 1,
        }
    }
}

struct TrivialPlayer {}
impl Iterator for MoveIter<'_> {
    type Item = engine::Action;

    fn next(&mut self) -> Option<engine::Action> {
        while self.start_idx < self.hand.len() {
            while self.end_idx < self.hand.len() {
                let proposed = &self.hand[self.start_idx..self.end_idx];
                if engine::legal_and_beats_board(self.board, proposed).is_none() {
                    return Some(engine::Action::PlayCards(
                        self.start_idx as u8,
                        self.end_idx as u8,
                    ));
                }
                self.end_idx += 1;
            }
            self.start_idx += 1;
            self.end_idx = self.start_idx + 1;
        }
        None
    }
}

impl Player for TrivialPlayer {
    fn choose_action(
        &self,
        public_state: &engine::PublicState,
        hidden_state: &engine::PlayerHiddenState,
    ) -> engine::Action {
        if !public_state.orientation_chosen {
            return engine::Action::ChooseOrientation(engine::FlipHand::DoNotFlip);
        }
        let mut move_iter = MoveIter::new(&hidden_state.hand, &public_state.board);
        let card_play = move_iter.next();
        if card_play.is_some() {
            card_play.unwrap()
        } else {
            engine::Action::PlayScoutToken((
                engine::PickedCard::FirstCard,
                0,
                engine::Orientation::Larger,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choose_action() {
        let mut state = engine::GameState::new(10, 3, 123);
        let trivial_player_1 = TrivialPlayer {};
        let trivial_player_2 = TrivialPlayer {};
        while !state.public_state.game_complete {
            let active_player: &TrivialPlayer;
            let hidden_state: &engine::PlayerHiddenState;
            if state.public_state.is_player_one_turn {
                active_player = &trivial_player_1;
                hidden_state = &state.player_one_hidden_state;
            } else {
                active_player = &trivial_player_2;
                hidden_state = &state.player_two_hidden_state;
            }
            let action = active_player.choose_action(&state.public_state, &hidden_state);
            let result = state.transition(&action);
            if !matches!(result, engine::TransitionResult::IllegalMove(_)) {
                state.display();
            }
            assert!(
                !matches!(result, engine::TransitionResult::IllegalMove(_)),
                "Illegal move: {:?}",
                result
            );
        }
    }
}
