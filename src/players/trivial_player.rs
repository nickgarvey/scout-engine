use crate::engine::{self};
use crate::players::player::Player;
use crate::search::MoveIter;


struct TrivialPlayer {}
impl Player for TrivialPlayer {
    fn choose_action(
        &self,
        public_state: &engine::PublicState,
        hidden_state: &engine::PlayerHiddenState,
    ) -> engine::Action {
        if !public_state.orientation_chosen {
            return engine::Action::ChooseOrientation(engine::FlipHand::DoNotFlip);
        }
        let mut move_iter = MoveIter::new(public_state, hidden_state);
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
