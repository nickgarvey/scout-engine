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

pub struct GameNode {
    state: GameState,
    children: Vec<Box<GameNode>>,
}

impl GameNode {
    pub fn new(state: GameState) -> Self {
        GameNode {
            state: state,
            children: Vec::new(),
        }
    }

    pub fn is_terminal(&self) -> bool {
        if self.state.public_state.game_complete != self.children.is_empty() {
            self.state.display();
            panic!(
                "Game state and children mismatch {:?} {:?}",
                self.state.public_state.game_complete,
                self.children.is_empty()
            );
        }
        self.state.public_state.game_complete
    }

    pub fn num_terminal_nodes(&self) -> usize {
        if self.is_terminal() {
            return 1;
        }
        let mut count = 0;
        for child in &self.children {
            count += child.num_terminal_nodes();
        }
        count
    }
}

pub fn tree_from_game_state(state: GameState, depth: usize) -> Box<GameNode> {
    let seed = state.seed;
    let mut node = Box::new(GameNode::new(state));
    if node.state.calculate_hash() == 18220208271962819626 {
        println!("Found a node with hash 18220208271962819626");
    }
    if depth == 0 || node.state.public_state.game_complete {
        return node;
    }

    let hidden_state = if node.state.public_state.is_player_one_turn {
        &node.state.player_one_hidden_state
    } else {
        &node.state.player_two_hidden_state
    };

    let mut move_iter = MoveIter::new(&node.state.public_state, hidden_state);

    while let Some(action) = move_iter.next() {
        let mut new_state = node.state.clone();
        match new_state.transition(&action) {
            TransitionResult::IllegalMove(reason) => {
                panic!(
                    "Illegal move ({:?}) (seed:{:?}): {:?}",
                    reason, seed, action
                );
            }
            TransitionResult::GameComplete(_, _) => {
                assert_eq!(
                    new_state.public_state.game_complete, true,
                    "Game should be complete after transition"
                );
                let child_node = tree_from_game_state(new_state, depth - 1);
                node.children.push(child_node);
            }
            _ => {
                let child_node = tree_from_game_state(new_state, depth - 1);
                node.children.push(child_node);
            }
        }
    }

    node
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
    fn test_tree_build_small() {
        let state = GameState::new(4, 0, 123);
        let tree = tree_from_game_state(state, 100);
        let num_terminal_nodes = tree.num_terminal_nodes();
        println!("Number of terminal nodes: {}", num_terminal_nodes);
        // total games depends on the orientation each player picks. there are no choices after that.
        // so 4 games total.
        assert_eq!(num_terminal_nodes, 4);
    }

    #[test]
    fn test_tree_build_medium() {
        let state = GameState::new(6, 1, 123);
        let tree = tree_from_game_state(state, 100);
        let num_terminal_nodes = tree.num_terminal_nodes();
        println!("Number of terminal nodes: {}", num_terminal_nodes);
        assert_eq!(num_terminal_nodes, 4);
    }
}
