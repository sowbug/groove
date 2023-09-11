# Notes

## TODOs

- Try dragging a pattern onto the sequencer and see it stamp itself into the
  sequence. Then hover over patterns vs notes and see that you can delete a
  pattern of notes as a group

## egui research

A widget is a lightweight reusable GUI component that is intended to be
instantiated on every render. Typically mutates a single provided variable,
though it can theoretically do more. egui's `Widget` trait consumes self, so
that's how you know it wasn't supposed to be kept around.
  
```rust
  pub trait Widget {
    fn ui(self, ui: &mut Ui) -> Response;
  }
```

A component (my term), on the other hand, is a long-lived struct that has the
ability to show a view of itself using a combination of widgets and custom
drawing commands. It can mutate any of its fields during the show operation, or
really do anything else.

There are many variations of the ui() signature in the demos.

```rust
// part of View trait
- fn ui(&mut self, ui: &mut Ui);
// not part of a trait
- fn ui(&mut self, ui: &mut Ui) -> Response;
// internal, part of Tree struct
- fn ui(&mut self, ui: &mut Ui) -> Action;
// appears to be top-level (demo_app_windows.rs)
- fn ui(&mut self, ctx: &Context);
// These are one-offs
- fn ui_content(&mut self, ui: &mut Ui) -> Response;
- fn ui_control(&mut self, ui: &mut Ui);
- fn ui_control(&mut self, ui: &mut Ui) -> Response;
```

show() is more consistent (part of the Demo trait): `fn show(&mut self, ctx: &Context, open: &mut bool);`

The sample custom widget (toggle) has a wrapper that implements `Widget`, but
I'm not totally sure how it works.

11 Sep 2023: an advantage of a widget is that it lives for only one frame. This
means that if you hand it a reference, it's a lot easier to reason about its
scope. This might mean that even components should have a "shadow widget" that
takes (1) a mut reference to the component, and (2) whatever other references it
needs just for drawing. The downside is that we lose the standardized Displays
trait in things that we were calling "components," and make the app rendering
code a little more complicated. But that complication should be exactly offset
by the simpler setup of the component in the first place (since we no longer
have to give the component the long-lived reference).

## painting one thing over another thing

```rust
// How big the paint surface should be
let desired_size = vec2(ui.available_width(), 64.0);
// Ask Ui to turn that Vec2 into a laid-out area
let (id, rect) = ui.allocate_space(desired_size);
// Get the portion of the Ui painter corresponding to the area we want to paint
let painter = ui.painter_at(rect);

// Example of painting within the region
// For easier painting, use the to_screen approach to transform local coords to the screen rect as
// demonstrated in https://github.com/emilk/egui/blob/master/crates/egui_demo_lib/src/demo/paint_bezier.rs#L72
painter.rect_filled(rect, Rounding::default(), Color32::GRAY);

// Now ask Ui to allocate a rect that's the same as the one we just painted on,
// and set the cursor to the start of that region.
ui.allocate_ui_at_rect(rect, |ui| {
    ui.label("I'm a widget being drawn on top of a painted surface!");
});
```

You can do this again and again for as many layers as you want.

## Possible egui traits

```rust
    /// Something that can be called during egui rendering to display a view of
    /// itself.
    //
    // Taken from egui_demo_lib/src/demo/mod.rs
    pub trait View {
        fn ui(&mut self, ui: &mut egui::Ui);
    }
 
    // pub trait DisplaysComponent {
    //     /// A self-contained entity that has all it needs to display itself.
    //     fn show_component(&mut self, ui: &mut egui::Ui) -> egui::Response;
    // }

    pub trait DisplaysArrangement {
        /// An entity that can display the portion of itself corresponding to a
        /// slice of [MusicalTime].
        fn show_arrangement(
            &mut self,
            ui: &mut egui::Ui,
            time_range: std::ops::Range<crate::time::MusicalTime>,
        ) -> egui::Response;
    }
```
