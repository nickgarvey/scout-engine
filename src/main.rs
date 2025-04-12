mod engine;
mod search;
mod players;
mod tree_builder;

fn main() {
    let state = engine::GameState::new(7, 2, 123);
    let tree = search::tree_from_game_state(state, 100);
    let num_terminal_nodes = tree.num_terminal_nodes();
    println!("Number of terminal nodes: {}", num_terminal_nodes);
}
