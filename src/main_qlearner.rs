mod engine;
mod players;
mod tree_builder;
mod search;

fn main() {
    players::qlearning_player::QLearningPlayer::new(0.1, 1.0, 0.05);
}
