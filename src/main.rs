mod engine;

fn main() {
    let mut state = engine::GameCompleteState::new(10, 2);
    state.transition(&engine::Action::ChooseOrientation(engine::FlipHand::DoFlip));
    state.transition(&engine::Action::ChooseOrientation(engine::FlipHand::DoNotFlip));
    state.display();
}
