use crate::engine::{
    legal_and_beats_board, Action, FlipHand, GameState, Orientation, PickedCard,
};

/// Enumerates all legal actions for the current player from the given game state.
pub fn enumerate_legal_actions(state: &GameState) -> Vec<Action> {
    let mut legal_actions = Vec::new();

    if state.public_state.game_complete {
        return legal_actions; // No actions if game is over
    }

    // --- 1. Handle Orientation Choice ---
    if !state.public_state.orientation_chosen {
        legal_actions.push(Action::ChooseOrientation(FlipHand::DoFlip));
        legal_actions.push(Action::ChooseOrientation(FlipHand::DoNotFlip));
        return legal_actions; // Only orientation choice is possible
    }

    // --- 2. Handle PlayCards Actions ---
    let hand = if state.public_state.is_player_one_turn {
        &state.player_one_hidden_state.hand
    } else {
        &state.player_two_hidden_state.hand
    };

    for start_idx in 0..hand.len() {
        for end_idx in (start_idx + 1)..=hand.len() {
            let proposed_play = &hand[start_idx..end_idx];
            // Check if the proposed play forms a valid set AND beats the board (or board is empty)
            if legal_and_beats_board(&state.public_state.board, proposed_play).is_none() {
                legal_actions.push(Action::PlayCards(start_idx as u8, end_idx as u8));
            }
        }
    }

    // --- 3. Handle PlayScoutToken Actions ---
    let has_tokens = if state.public_state.is_player_one_turn {
        state.public_state.player_one_scout_token_count > 0
    } else {
        state.public_state.player_two_scout_token_count > 0
    };

    if has_tokens && !state.public_state.board.is_empty() {
        let hand_len = hand.len();
        for insertion_idx in 0..=hand_len {
            // Try taking the first card
            legal_actions.push(Action::PlayScoutToken((
                PickedCard::FirstCard,
                insertion_idx as u8,
                Orientation::Larger,
            )));
            legal_actions.push(Action::PlayScoutToken((
                PickedCard::FirstCard,
                insertion_idx as u8,
                Orientation::Smaller,
            )));

            // Try taking the last card (only if different from the first)
            if state.public_state.board.len() > 1 {
                legal_actions.push(Action::PlayScoutToken((
                    PickedCard::LastCard,
                    insertion_idx as u8,
                    Orientation::Larger,
                )));
                legal_actions.push(Action::PlayScoutToken((
                    PickedCard::LastCard,
                    insertion_idx as u8,
                    Orientation::Smaller,
                )));
            }
        }
    }

    legal_actions
}

// Optional: Add tests specific to this function within this module
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{FlipHand, GameState};

    #[test]
    fn test_enumerate_orientation() {
        let state = GameState::new(10, 3, 1);
        let actions = enumerate_legal_actions(&state);
        assert_eq!(actions.len(), 2);
        assert!(actions.contains(&Action::ChooseOrientation(FlipHand::DoFlip)));
        assert!(actions.contains(&Action::ChooseOrientation(FlipHand::DoNotFlip)));
    }

    #[test]
    fn test_enumerate_initial_play() {
        let mut state = GameState::new(10, 3, 1);
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip)); // Now player 1's turn, orientation chosen

        let actions = enumerate_legal_actions(&state);

        // Should only contain PlayCards actions, as board is empty and scout is illegal
        assert!(actions.iter().all(|a| matches!(a, Action::PlayCards(_, _))));

        // Check a specific expected legal play (e.g., playing the first card)
        assert!(actions.contains(&Action::PlayCards(0, 1)), "Initial state should allow playing the first card");
        // We cannot easily assert the exact count without access to private functions like build_card_set,
        // but we've confirmed the type of actions and the presence of a basic one.
        // The function enumerate_legal_actions internally uses legal_and_beats_board,
        // which handles the set validation.
    }

     #[test]
    fn test_enumerate_with_board_and_scout() {
        let mut state = GameState::new(10, 3, 1);
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        // Player 1 plays a card
        state.transition(&Action::PlayCards(0, 1)); // Assume this is legal

        // Now player 2's turn
        let actions = enumerate_legal_actions(&state);

        let hand = &state.player_two_hidden_state.hand;
        let board = &state.public_state.board;
        let mut expected_actions = Vec::new();

        // Expected PlayCards
        for start in 0..hand.len() {
            for end in (start + 1)..=hand.len() {
                 if legal_and_beats_board(board, &hand[start..end]).is_none() {
                    expected_actions.push(Action::PlayCards(start as u8, end as u8));
                 }
            }
        }

        // Expected Scout Tokens
        let hand_len = hand.len();
        if state.public_state.player_two_scout_token_count > 0 && !board.is_empty() {
             for insertion_idx in 0..=hand_len {
                expected_actions.push(Action::PlayScoutToken((PickedCard::FirstCard, insertion_idx as u8, Orientation::Larger)));
                expected_actions.push(Action::PlayScoutToken((PickedCard::FirstCard, insertion_idx as u8, Orientation::Smaller)));
                if board.len() > 1 { // Only add LastCard if distinct
                    expected_actions.push(Action::PlayScoutToken((PickedCard::LastCard, insertion_idx as u8, Orientation::Larger)));
                    expected_actions.push(Action::PlayScoutToken((PickedCard::LastCard, insertion_idx as u8, Orientation::Smaller)));
                }
            }
        }

        // Compare lengths first for easier debugging
        assert_eq!(actions.len(), expected_actions.len(), "Action count mismatch. Actual: {:?}, Expected: {:?}", actions, expected_actions);

        // Check that all expected actions are present
        for expected_action in &expected_actions {
            assert!(actions.contains(expected_action), "Missing expected action: {:?}", expected_action);
        }
         // Check that no unexpected actions are present
        for action in &actions {
            assert!(expected_actions.contains(action), "Unexpected action found: {:?}", action);
        }
    }

     #[test]
    fn test_enumerate_no_scout_tokens() {
        let mut state = GameState::new(10, 0, 1); // 0 scout tokens
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::PlayCards(0, 1)); // Player 1 plays

        // Player 2's turn, has 0 tokens
        let actions = enumerate_legal_actions(&state);

        // Should only contain PlayCards actions
        assert!(actions.iter().all(|a| matches!(a, Action::PlayCards(_, _))));
    }

     #[test]
    fn test_enumerate_game_complete() {
        let mut state = GameState::new(6, 0, 5); // Use a game that ends quickly
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::PlayCards(2, 3));
        state.transition(&Action::PlayCards(1, 2));
        state.transition(&Action::PlayCards(0, 2)); // Game should end here

        assert!(state.public_state.game_complete);
        let actions = enumerate_legal_actions(&state);
        assert_eq!(actions.len(), 0); // No legal actions when game is complete
    }
}
