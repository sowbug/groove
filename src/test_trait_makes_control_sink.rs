use crate::traits::MakesControlSink;

// pub trait MakesControlSink: Debug {
//     fn make_control_sink(&self, param_name: &str) -> Option<Box<dyn SinksControl>>;
// }

#[test]
fn test_trait_makes_control_sink() {
    let s = instance();
    let _cs = s.make_control_sink("foo");
}
