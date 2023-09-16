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

### Terminology: Entity, Device, Component, Thing

I've cycled through these terms throughout development. None is especially
better than the rest, but they've been useful historical markers during
refactoring to tell what's old and what's new. I think it's time to consolidate,
and perhaps to deal with [https://github.com/sowbug/groove/issues/149]
(separating Things-That-Display from Things-That-Don't-Display) while we're
there.

The concept we're trying to capture is something in our system that implements
certain core traits, optionally implements other traits, and provides enough
introspection via the core traits to allow others to figure out what it can do.

Candidates:

- Entity: good. A little overloaded by ECS (Entity/Component/System) patterns.
    Slight disadvantage that its pluralization is irregular.
- Device: meh. The term implies independent function, which is OK for a
    musical instrument or effect, but misleading for something like ControlTrip.
- Component: good. Like Entity, it's so overloaded that it's almost understood
    not to have a global meaning. Regular pluralization.
- Thing: annoying. It's what I started using when I ran out of new names.

Finalists are Entity and Component. I prefer Entity, partly because it's what I
first used in the project. I guess I don't mind much about irregular "Entities"
rather than "Entitys." **Entity is the winner.**

The Displays issue: the problem is that everyone has to implement it, but some
entities, like ESSequencer or Transport, are supposed to be passed into
custom-built widgets instead of being directly rendered via Displays. So right
now their implementation of Displays::ui() is just to panic. Is this so bad?
Each of these is a special kind of Entity, where the owner never loses track of
its concrete type and thus knows about the Entity-widget association. The only
time this could bite us, therefore, is when these special Entities end up on an
assembly line with other generic Entities, like in the explorer example (which
actually doesn't do any type erasure). Perhaps it's not that bad.

Another approach could be to define something like a SystemEntity that is like
an Entity but doesn't implement Displays. This might be weird because Rust
doesn't have negative traits, so it might actually be "Entity = SystemEntity++"
rather than "SystemEntity = Entity--". But that's just terminology; it could
also be that most normal things are DisplayableEntity, and "system entities" are
just Entity.

Or Displays could be another runtime-discoverable trait.

Or the degenerate implementation of Displays could instead invoke the widget
(though the whole point was that Displays::ui() wasn't sufficient).

**Conclusion: maybe we can live with panic().**

### Rules for communication among app components

(Moved from minidaw.rs. Might be out of date.)

- If it's in the same thread, don't be fancy. Example: the app owns the
  control bar, and the control bar always runs in the UI thread. The app
  should talk directly to the control bar (update transport), and the control
  bar can pass back an enum saying what happened (play button was pressed).
- If it's updated rarely but displayed frequently, the struct should push it
  to the app, and the app should cache it. Example: BPM is displayed in the
  control bar, so we're certain to need it on every redraw, but it rarely
  changes (unless it's automated). Orchestrator should define a channel
  message, and the app should handle it when it's received. (This is
  currently a not-great example, because we're cloning [Transport] on each
  cycle.)
- If it's updated more often than the UI framerate, let the UI pull it
  directly from the struct. Example: an LFO signal or a real-time spectrum
  analysis. These should be APIs directly on the struct, and we'll leave it
  up to the app to lock the struct and get what it needs.
