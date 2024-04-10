use std::sync::mpsc;

#[derive(Default, Debug)]
pub struct InputState {
    pub cursor_position: (f64, f64),
    pub cursor_delta: (f64, f64),
    pub mouse_buttons: [bool; 3],
    pub keys: Vec<bool>,
    pub sender: Option<mpsc::Sender<InputStateEvent>>,
    pub receiver: Option<mpsc::Receiver<InputStateEvent>>,
}

impl InputState {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            cursor_position: (0.0, 0.0),
            cursor_delta: (0.0, 0.0),
            mouse_buttons: [false; 3],
            keys: vec![false; 1024],
            sender: Some(sender),
            receiver: Some(receiver),
        }
    }

    pub fn consume(&mut self, event: InputStateEvent) {
        match event {
            InputStateEvent::CursorMoved(position) => {
                self.cursor_delta = (
                    position.0 - self.cursor_position.0,
                    position.1 - self.cursor_position.1,
                );
                self.cursor_position = position;
            }
            InputStateEvent::MouseButtonPressed(button) => {
                self.mouse_buttons[button as usize] = true;
            }
            InputStateEvent::MouseButtonReleased(button) => {
                self.mouse_buttons[button as usize] = false;
            }
            InputStateEvent::KeyPressed(key) => {
                self.keys[key as usize] = true; // careful here, if this struct was created without keys being initialized large enough this will panic
            }
            InputStateEvent::KeyReleased(key) => {
                self.keys[key as usize] = false;
            }
            InputStateEvent::KeepAlive => {}
        }
    }

    pub fn consume_channel_events(&mut self) {
        let gathered_events = self.receiver.as_ref().unwrap().try_iter().collect::<Vec<_>>();
        for event in gathered_events.into_iter() {
            self.consume(event);
        }
    }

    pub fn sender_clone(&self) -> mpsc::Sender<InputStateEvent> {
        self.sender.as_ref().unwrap().clone()
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum InputStateEvent {
    CursorMoved((f64, f64)),
    MouseButtonPressed(u8),
    MouseButtonReleased(u8),
    KeyPressed(u8),
    KeyReleased(u8),
    #[default] KeepAlive,
}
