use wasm_bindgen::JsCast;
use web_sys::{DragEvent, Event};

pub trait IntoDragEvent {
    fn into_drag_event(self) -> DragEvent;
}

impl IntoDragEvent for Event {
    fn into_drag_event(self) -> DragEvent {
        self.dyn_into::<DragEvent>()
            .expect("cannot cast given event into DragEvent")
    }
}
