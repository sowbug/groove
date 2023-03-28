// Copyright (c) 2023 Mike Tsao. All rights reserved.

// PRO TIP: use `cargo expand` to see what's being generated. It requires the
// nightly toolchain.

#[macro_export]
macro_rules! boxed_entity_enum_and_common_crackers {
    ($($variant:ident: $type:ty,)*) => {
        #[derive(Debug)]
        pub enum Entity {
            $( $variant(Box<$type>) ),*
        }

        impl Entity {
            pub fn as_has_uid(&self) -> &dyn HasUid {
                match self {
                $( Entity::$variant(e) => e.as_ref(), )*
                }
            }
            pub fn as_has_uid_mut(&mut self) -> &mut dyn HasUid {
                match self {
                $( Entity::$variant(e) => e.as_mut(), )*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! controllable_crackers {
    ($($type:ident,)*) => {
        impl Entity {
            pub fn as_controllable(&self) -> Option<&dyn Controllable> {
                match self {
                    $( Entity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_controllable_mut(&mut self) -> Option<&mut dyn Controllable> {
                match self {
                    $( Entity::$type(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! controller_crackers {
    ($($type:ident,)*) => {
        impl Entity {
            pub fn as_is_controller(&self) -> Option<&dyn IsController<Message=EntityMessage>> {
                match self {
                    $( Entity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_controller_mut(&mut self) -> Option<&mut dyn IsController<Message=EntityMessage>> {
                match self {
                    $( Entity::$type(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! effect_crackers {
    ($($type:ident,)*) => {
        impl Entity {
            pub fn as_is_effect(&self) -> Option<&dyn IsEffect> {
                match self {
                $( Entity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_effect_mut(&mut self) -> Option<&mut dyn IsEffect> {
                match self {
                $( Entity::$type(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! instrument_crackers {
    ($($type:ident,)*) => {
        impl Entity {
            pub fn as_is_instrument(&self) -> Option<&dyn IsInstrument> {
                match self {
                $( Entity::$type(e) => Some(e.as_ref()), )*
                    _ => None,
                }
            }
            pub fn as_is_instrument_mut(&mut self) -> Option<&mut dyn IsInstrument> {
                match self {
                $( Entity::$type(e) => Some(e.as_mut()), )*
                    _ => None,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! handles_midi_crackers {
    ($($type:ident,)*) => {
        impl Entity {
            pub fn as_handles_midi(&self) -> Option<&dyn HandlesMidi> {
                match self {
                    $( Entity::$type(e) => Some(e.as_ref()), )*
                    _ => None
                }
            }
            pub fn as_handles_midi_mut(&mut self) -> Option<&mut dyn HandlesMidi> {
                match self {
                    $( Entity::$type(e) => Some(e.as_mut()), )*
                    _ => None
                }
            }
        }
    };
}

#[macro_export]
macro_rules! register_impl {
    ($trait_:ident for $ty:ty, true) => {
        impl<'a> MaybeImplements<'a, dyn $trait_> for $ty {
            fn as_trait_ref(&self) -> Option<&(dyn $trait_ + 'static)> {
                Some(self)
            }
            fn as_trait_mut(&mut self) -> Option<&mut (dyn $trait_ + 'static)> {
                Some(self)
            }
        }
    };
    ($trait_:ident for $ty:ty, false) => {
        impl<'a> MaybeImplements<'a, dyn $trait_> for $ty {
            fn as_trait_ref(&self) -> Option<&(dyn $trait_ + 'static)> {
                None
            }
            fn as_trait_mut(&mut self) -> Option<&mut (dyn $trait_ + 'static)> {
                None
            }
        }
    };
}

#[macro_export]
macro_rules! all_entities {
($($entity:ident; $params:tt; $message:ident; $is_controller:tt; $is_controllable:tt ,)*) => {
    #[derive(Clone, Debug)]
    pub enum OtherEntityMessage {
        $( $params($message) ),*
    }
    #[derive(Debug)]
    pub enum EntityParams {
        $( $entity(Box<$params>) ),*
    }
    impl EntityParams {
        pub fn is_controller(&self) -> bool {
            match self {
                $( EntityParams::$entity(_) => $is_controller, )*
            }
        }
        pub fn is_controllable(&self) -> bool {
            match self {
                $( EntityParams::$entity(_) => $is_controllable, )*
            }
        }
        pub fn as_controllable_ref(&self) -> Option<&(dyn Controllable + 'static)> {
            match self {
                $( EntityParams::$entity(e) => e.as_trait_ref(), )*
            }
        }
        pub fn as_controllable_mut(&mut self) -> Option<&mut (dyn Controllable + 'static)> {
            match self {
                $( EntityParams::$entity(e) => e.as_trait_mut(), )*
            }
        }

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
    trait MaybeImplements<'a, Trait: ?Sized> {
        fn as_trait_ref(&'a self) -> Option<&'a Trait>;
        fn as_trait_mut(&mut self) -> Option<&mut Trait>;
    }
    $( register_impl!(Controllable for $params, $is_controllable); )*
};
}
