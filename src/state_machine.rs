use windows::Win32::Foundation::HWND;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Idle,
    TransparencyModal {
        target_hwnd: HWND,
        current_percentage: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentAction {
    Decrease,
    Increase,
    Commit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    None,
    Changed { target_hwnd: HWND, new_percentage: u8 },
    Committed { target_hwnd: HWND, final_percentage: u8 },
    Aborted,
}

pub struct StateMachine {
    mode: Mode,
}

impl StateMachine {
    pub fn new() -> Self {
        Self { mode: Mode::Idle }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn enter_modal(&mut self, hwnd: HWND, initial_percentage: u8) -> Transition {
        let initial_clamped = initial_percentage.clamp(60, 100);
        self.mode = Mode::TransparencyModal {
            target_hwnd: hwnd,
            current_percentage: initial_clamped,
        };
        Transition::Changed {
            target_hwnd: hwnd,
            new_percentage: initial_clamped,
        }
    }

    pub fn handle_action(&mut self, action: AdjustmentAction) -> Transition {
        match self.mode {
            Mode::Idle => Transition::None,
            Mode::TransparencyModal { target_hwnd, current_percentage } => {
                match action {
                    AdjustmentAction::Decrease => {
                        let new_percentage = if current_percentage >= 60 + 2 {
                            current_percentage - 2
                        } else {
                            60
                        };
                        self.mode = Mode::TransparencyModal {
                            target_hwnd,
                            current_percentage: new_percentage,
                        };
                        Transition::Changed {
                            target_hwnd,
                            new_percentage,
                        }
                    }
                    AdjustmentAction::Increase => {
                        let new_percentage = if current_percentage + 2 <= 100 {
                            current_percentage + 2
                        } else {
                            100
                        };
                        self.mode = Mode::TransparencyModal {
                            target_hwnd,
                            current_percentage: new_percentage,
                        };
                        Transition::Changed {
                            target_hwnd,
                            new_percentage,
                        }
                    }
                    AdjustmentAction::Commit => {
                        self.mode = Mode::Idle;
                        Transition::Committed {
                            target_hwnd,
                            final_percentage: current_percentage,
                        }
                    }
                }
            }
        }
    }

    pub fn handle_window_change(&mut self, current_active_hwnd: HWND) -> Transition {
        match self.mode {
            Mode::Idle => Transition::None,
            Mode::TransparencyModal { target_hwnd, current_percentage } => {
                if current_active_hwnd != target_hwnd {
                    self.mode = Mode::Idle;
                    Transition::Committed {
                        target_hwnd,
                        final_percentage: current_percentage,
                    }
                } else {
                    Transition::None
                }
            }
        }
    }

    pub fn handle_window_closed(&mut self) -> Transition {
        match self.mode {
            Mode::Idle => Transition::None,
            Mode::TransparencyModal { .. } => {
                self.mode = Mode::Idle;
                Transition::Aborted
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let sm = StateMachine::new();
        assert_eq!(sm.mode(), Mode::Idle);
    }

    #[test]
    fn test_enter_modal() {
        let mut sm = StateMachine::new();
        let hwnd = HWND(12345);
        let trans = sm.enter_modal(hwnd, 80);

        assert_eq!(
            trans,
            Transition::Changed {
                target_hwnd: hwnd,
                new_percentage: 80,
            }
        );
        assert_eq!(
            sm.mode(),
            Mode::TransparencyModal {
                target_hwnd: hwnd,
                current_percentage: 80,
            }
        );
    }

    #[test]
    fn test_adjust_transparency() {
        let mut sm = StateMachine::new();
        let hwnd = HWND(12345);
        sm.enter_modal(hwnd, 80);

        // Decrease
        let trans = sm.handle_action(AdjustmentAction::Decrease);
        assert_eq!(
            trans,
            Transition::Changed {
                target_hwnd: hwnd,
                new_percentage: 78,
            }
        );

        // Increase
        let trans2 = sm.handle_action(AdjustmentAction::Increase);
        assert_eq!(
            trans2,
            Transition::Changed {
                target_hwnd: hwnd,
                new_percentage: 80,
            }
        );
    }

    #[test]
    fn test_adjust_clamping() {
        let mut sm = StateMachine::new();
        let hwnd = HWND(12345);
        
        // Clamp min
        sm.enter_modal(hwnd, 3);
        let trans = sm.handle_action(AdjustmentAction::Decrease);
        assert_eq!(
            trans,
            Transition::Changed {
                target_hwnd: hwnd,
                new_percentage: 60,
            }
        );
        let trans_further = sm.handle_action(AdjustmentAction::Decrease);
        assert_eq!(
            trans_further,
            Transition::Changed {
                target_hwnd: hwnd,
                new_percentage: 60,
            }
        );

        // Clamp max
        let mut sm2 = StateMachine::new();
        sm2.enter_modal(hwnd, 99);
        let trans2 = sm2.handle_action(AdjustmentAction::Increase);
        assert_eq!(
            trans2,
            Transition::Changed {
                target_hwnd: hwnd,
                new_percentage: 100,
            }
        );
        let trans_further2 = sm2.handle_action(AdjustmentAction::Increase);
        assert_eq!(
            trans_further2,
            Transition::Changed {
                target_hwnd: hwnd,
                new_percentage: 100,
            }
        );
    }

    #[test]
    fn test_commit_action() {
        let mut sm = StateMachine::new();
        let hwnd = HWND(12345);
        sm.enter_modal(hwnd, 80);

        let trans = sm.handle_action(AdjustmentAction::Commit);
        assert_eq!(
            trans,
            Transition::Committed {
                target_hwnd: hwnd,
                final_percentage: 80,
            }
        );
        assert_eq!(sm.mode(), Mode::Idle);
    }

    #[test]
    fn test_window_change() {
        let mut sm = StateMachine::new();
        let hwnd1 = HWND(12345);
        let hwnd2 = HWND(67890);
        sm.enter_modal(hwnd1, 80);

        // Window remains the same
        let trans = sm.handle_window_change(hwnd1);
        assert_eq!(trans, Transition::None);

        // Window changes
        let trans2 = sm.handle_window_change(hwnd2);
        assert_eq!(
            trans2,
            Transition::Committed {
                target_hwnd: hwnd1,
                final_percentage: 80,
            }
        );
        assert_eq!(sm.mode(), Mode::Idle);
    }

    #[test]
    fn test_window_closed() {
        let mut sm = StateMachine::new();
        let hwnd = HWND(12345);
        sm.enter_modal(hwnd, 80);

        let trans = sm.handle_window_closed();
        assert_eq!(trans, Transition::Aborted);
        assert_eq!(sm.mode(), Mode::Idle);
    }
}
