mod engine;
mod players;
mod tree_builder;

fn main() {
    let mut state = engine::GameState::new(10, 3, 2);
    state.transition(&engine::Action::ChooseOrientation(engine::FlipHand::DoFlip));
    state.transition(&engine::Action::ChooseOrientation(engine::FlipHand::DoNotFlip));
    state.display();
}
