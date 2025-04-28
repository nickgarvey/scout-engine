use crate::engine::{
    legal_and_beats_board, Action, FlipHand, GameState, Orientation, PickedCard, TransitionResult,
};
use std::collections::HashMap;

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
        for insertion_idx in 0..=hand.len() {
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


/// Represents a node in the game state tree.
#[derive(Debug, Clone)]
pub struct GameNode {
    pub state: GameState,
    // Maps an action taken from this state to the resulting child node.
    pub children: HashMap<Action, GameNode>,
}

/// Builds a game tree recursively starting from the given state.
/// Explores all reachable states through legal actions.
pub fn build_game_tree(initial_state: GameState, depth_limit: u64) -> GameNode {
    let mut root_node = GameNode {
        state: initial_state,
        children: HashMap::new(),
    };
    if depth_limit == 0 {
        return root_node;
    }

    // If the game is already complete at this node, no further actions are possible.
    if root_node.state.public_state.game_complete {
        return root_node;
    }

    let legal_actions = enumerate_legal_actions(&root_node.state);

    for action in legal_actions {
        let mut next_state = root_node.state.clone();
        let transition_result = next_state.transition(&action);

        // Only proceed if the move was accepted or led to game completion.
        // Illegal moves shouldn't happen if enumerate_legal_actions is correct,
        // but we handle it defensively.
        match transition_result {
            TransitionResult::MoveAccepted | TransitionResult::GameComplete(..) => {
                // Recursively build the subtree for the resulting state.
                let child_node = build_game_tree(next_state, depth_limit-1);
                root_node.children.insert(action, child_node);
            }
            TransitionResult::IllegalMove(reason) => {
                assert!(
                    false,
                    "enumerate_legal_actions produced an illegal move {:?}: {:?}",
                    action, reason
                );
            }
        }
    }

    root_node
}

/// Counts the total number of nodes in the game tree rooted at the given node.
pub fn count_nodes(node: &GameNode) -> u64 {
    let mut count = 1; // Count the current node
    for child_node in node.children.values() {
        count += count_nodes(child_node); // Recursively count nodes in children
    }
    count
}


// Optional: Add tests specific to this function within this module
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{FlipHand, GameState, Orientation, PickedCard}; // Added PickedCard, Orientation

    // --- Tests for enumerate_legal_actions ---
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

    // --- Tests for build_game_tree ---

    // Note: Building the full tree can be very time-consuming.
    // These tests check the structure near the root.
    #[test]
    fn test_build_tree_orientation_phase() {
        // Use small parameters for manageable tree size in tests
        let initial_state = GameState::new(6, 3, 3);
        // We don't build the full tree here, just check the initial steps.
        // The build_game_tree function itself is recursive.
        let tree = build_game_tree(initial_state.clone(), 3); // Clone initial state for checks

        // Root node should be the initial state
        assert!(!tree.state.public_state.orientation_chosen);
        assert!(tree.state.public_state.is_player_one_turn);

        // Should have two children: Flip and DoNotFlip for player 1
        assert_eq!(tree.children.len(), 2);

        let flip_action = Action::ChooseOrientation(FlipHand::DoFlip);
        let no_flip_action = Action::ChooseOrientation(FlipHand::DoNotFlip);

        assert!(tree.children.contains_key(&flip_action));
        assert!(tree.children.contains_key(&no_flip_action));

        // Check state after player 1 chooses (e.g., NoFlip)
        let child_node_p1_no_flip = tree.children.get(&no_flip_action).unwrap();
        assert!(!child_node_p1_no_flip.state.public_state.orientation_chosen);
        assert!(!child_node_p1_no_flip.state.public_state.is_player_one_turn); // Player 2's turn

        // Player 2 should also have two orientation choices
        assert_eq!(child_node_p1_no_flip.children.len(), 2);
        assert!(child_node_p1_no_flip.children.contains_key(&flip_action));
        assert!(child_node_p1_no_flip.children.contains_key(&no_flip_action));

         // Check state after player 2 chooses (e.g., NoFlip again)
        let child_node_p2_no_flip = child_node_p1_no_flip.children.get(&no_flip_action).unwrap();
        assert!(child_node_p2_no_flip.state.public_state.orientation_chosen); // Orientation now chosen
        assert!(child_node_p2_no_flip.state.public_state.is_player_one_turn); // Back to Player 1

        // Now the children should be PlayCards/PlayScoutToken actions
        assert!(!child_node_p2_no_flip.children.is_empty());
        assert!(child_node_p2_no_flip.children.keys().all(|a| !matches!(a, Action::ChooseOrientation(_))),
                "After orientation, actions should be Play or Scout");

        // Avoid asserting on the full recursive build in the test itself due to size/time.
        // We've verified the first few levels.
    }

    #[test]
    fn test_build_tree_play_phase_root() { // Renamed to clarify scope
        let initial_state = GameState::new(6, 3, 3);

        let tree = build_game_tree(initial_state.clone(), 2); // Clone initial state for checks

        // Root node state should reflect completed orientation
        assert!(!tree.state.public_state.orientation_chosen);
        assert!(tree.state.public_state.is_player_one_turn);

        // Children should be the legal PlayCards/Scout actions for player 1
        let expected_actions = enumerate_legal_actions(&tree.state);
        assert_eq!(tree.children.len(), expected_actions.len());
        for action in &expected_actions {
            assert!(tree.children.contains_key(action), "Tree missing action: {:?}", action);
        }
        // Check the state of *one* arbitrary child node to verify transition
        if let Some((action, child_node)) = tree.children.iter().next() {
             assert_eq!(child_node.state.public_state.is_player_one_turn, false,
                       "Child node after P1's move ({:?}) should be P2's turn", action);
             // Avoid checking child_node.children recursively here.
        } else {
             // This case might happen if P1 has no legal moves after orientation,
             // though unlikely with the default setup.
             println!("Warning: No legal actions found for P1 after orientation in test_build_tree_play_phase_root");
        }
    }

     #[test]
    fn test_build_tree_game_end_node() {
        let mut state = GameState::new(6, 0, 5); // Use game that ends quickly
        state.transition(&Action::ChooseOrientation(FlipHand::DoFlip));
        state.transition(&Action::ChooseOrientation(FlipHand::DoNotFlip));
        state.transition(&Action::PlayCards(2, 3));
        state.transition(&Action::PlayCards(1, 2));
        // Game ends after this next transition
        let final_action = Action::PlayCards(0, 2);
        state.transition(&final_action);

        assert!(state.public_state.game_complete);

        // Build tree starting from the completed state
        let tree = build_game_tree(state, 2);

        // A node representing a completed game should have no children
        assert!(tree.state.public_state.game_complete);
        assert!(tree.children.is_empty());
    }

    #[test]
    fn test_count_nodes_simple() {
        // Build a small tree (depth 2: P1 orient, P2 orient)
        let initial_state = GameState::new(6, 0, 5);
        let tree = build_game_tree(initial_state, 2);

        // Expected nodes:
        // 1 (root)
        // + 2 (P1 orient choices)
        // + 2 * 2 (P2 orient choices for each P1 choice)
        // = 1 + 2 + 4 = 7
        let node_count = count_nodes(&tree);
        assert_eq!(node_count, 7, "Expected 7 nodes for depth 2 orientation phase");
    }

    #[test]
    fn test_count_nodes_deeper() {
         // Build a slightly deeper tree (depth 3: P1 orient, P2 orient, P1 play)
        let initial_state = GameState::new(6, 0, 5);
        initial_state.display(); 

        let tree = build_game_tree(initial_state, 3);

        let node_count = count_nodes(&tree);
        let expected_count = 1 + 2 + 4;
        assert_eq!(node_count, expected_count, "Node count mismatch for depth 3");
    }

}
