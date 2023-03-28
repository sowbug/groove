// Copyright (c) 2023 Mike Tsao. All rights reserved.

// PRO TIP: use `cargo expand` to see what's being generated. It requires the
// nightly toolchain.

#[macro_export]
macro_rules! all_entities {
($($entity:ident; $params:tt; $message:ident; $is_controller:tt; $is_controllable:tt ,)*) => {
    #[derive(Clone, Debug)]
    pub enum OtherEntityMessage {
        $( $params($message) ),*
    }
    impl EntityParams {
        pub fn update(&mut self, message: OtherEntityMessage) {
            match self {
            $(
                EntityParams::$entity(e) => {
                    if let OtherEntityMessage::$params(message) = message {
                        e.update(message);
                    }
                }
            )*
            }
        }

    }
    impl Entity {
        pub fn update(&mut self, message: OtherEntityMessage) {
            match self {
            $(
                Entity::$entity(e) => {
                    if let OtherEntityMessage::$params(message) = message {
                        e.update(message);
                    }
                }
            )*
            }
        }

        pub fn message_for(
            &self,
            param_index: usize,
            value: groove_core::control::F32ControlValue,
        ) -> Option<OtherEntityMessage> {
            match self {
            $(
                Entity::$entity(e) => {
                    if let Some(message) = e.params().message_for_index(param_index, value) {
                        return Some(OtherEntityMessage::$params(message));
                    }
                }
            )*
            }
            None
        }

    }
};
}
