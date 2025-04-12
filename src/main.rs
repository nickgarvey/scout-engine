mod engine;
mod players;
mod search;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let num_cards = args[1].parse::<u8>().unwrap();
    let num_scout = args[2].parse::<u8>().unwrap();
    let seed = args[3].parse::<u64>().unwrap();

    let state = engine::GameState::new(num_cards, num_scout, seed);
    let mut count = 0;
    let mut count_fn = |_: engine::GameState| {
        count += 1;
    };
    search::walk_games(state, &mut count_fn);
    //let tree = search::tree_from_game_state(state, 100);
    //let num_terminal_nodes = tree.num_terminal_nodes();
    println!("Number of games: {}", count);
}
