use gilrs::{Gilrs, Event, EventType, Button};

/// Polls gilrs and forwards nav events to a handler.
/// Call from a background thread; handler must be Send.
pub fn run<F: Fn(NavEvent) + Send + 'static>(handler: F) {
    std::thread::spawn(move || {
        let mut gilrs = match Gilrs::new() {
            Ok(g) => g,
            Err(e) => { tracing::warn!("gamepad init failed: {e}"); return; }
        };
        loop {
            while let Some(Event { event, .. }) = gilrs.next_event() {
                let nav = match event {
                    EventType::ButtonPressed(Button::DPadUp, _) => Some(NavEvent::Up),
                    EventType::ButtonPressed(Button::DPadDown, _) => Some(NavEvent::Down),
                    EventType::ButtonPressed(Button::DPadLeft, _) => Some(NavEvent::Left),
                    EventType::ButtonPressed(Button::DPadRight, _) => Some(NavEvent::Right),
                    EventType::ButtonPressed(Button::South, _) => Some(NavEvent::Confirm),   // A/Cross
                    EventType::ButtonPressed(Button::East, _) => Some(NavEvent::Back),      // B/Circle
                    EventType::ButtonPressed(Button::Start, _) => Some(NavEvent::Menu),
                    _ => None,
                };
                if let Some(n) = nav { handler(n); }
            }
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
    });
}

#[derive(Clone, Copy, Debug)]
pub enum NavEvent { Up, Down, Left, Right, Confirm, Back, Menu }
