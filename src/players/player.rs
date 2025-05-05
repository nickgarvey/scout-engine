#![allow(dead_code)]
use crate::engine::{self};

pub trait Player {
    fn choose_action(
        &self,
        public_state: &engine::PublicState,
        hidden_state: &engine::PlayerHiddenState,
    ) -> engine::Action;
}