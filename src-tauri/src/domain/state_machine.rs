use crate::domain::models::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingState {
    Idle,
    Recording,
    Paused,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportState {
    Queued,
    Running,
    Fallback,
    Success,
    Failed,
}

#[derive(Debug, Clone)]
pub struct RecordingMachine {
    state: RecordingState,
}

impl RecordingMachine {
    pub fn new() -> Self {
        Self {
            state: RecordingState::Idle,
        }
    }

    pub fn state(&self) -> RecordingState {
        self.state
    }

    pub fn start(&mut self) -> Result<(), AppError> {
        if self.state != RecordingState::Idle {
            return Err(AppError::new(
                "INVALID_RECORDING_STATE",
                "only idle state can start recording",
                Some("wait for current session to stop first".to_string()),
            ));
        }
        self.state = RecordingState::Recording;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), AppError> {
        if self.state != RecordingState::Recording {
            return Err(AppError::new(
                "INVALID_RECORDING_STATE",
                "only recording state can be paused",
                Some("check whether recording has started".to_string()),
            ));
        }
        self.state = RecordingState::Paused;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), AppError> {
        if self.state != RecordingState::Paused {
            return Err(AppError::new(
                "INVALID_RECORDING_STATE",
                "only paused state can resume",
                Some("pause recording before resume".to_string()),
            ));
        }
        self.state = RecordingState::Recording;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), AppError> {
        if self.state != RecordingState::Recording && self.state != RecordingState::Paused {
            return Err(AppError::new(
                "INVALID_RECORDING_STATE",
                "only recording or paused state can stop",
                Some("start recording before stop".to_string()),
            ));
        }
        self.state = RecordingState::Stopped;
        Ok(())
    }
}

impl Default for RecordingMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ExportMachine {
    state: ExportState,
}

impl ExportMachine {
    pub fn new() -> Self {
        Self {
            state: ExportState::Queued,
        }
    }

    pub fn state(&self) -> ExportState {
        self.state
    }

    pub fn start(&mut self) -> Result<(), AppError> {
        if self.state != ExportState::Queued {
            return Err(AppError::new(
                "INVALID_EXPORT_STATE",
                "only queued task can start",
                None,
            ));
        }
        self.state = ExportState::Running;
        Ok(())
    }

    pub fn fallback(&mut self) -> Result<(), AppError> {
        if self.state != ExportState::Running {
            return Err(AppError::new(
                "INVALID_EXPORT_STATE",
                "fallback only allowed while running",
                None,
            ));
        }
        self.state = ExportState::Fallback;
        Ok(())
    }

    pub fn success(&mut self) -> Result<(), AppError> {
        if self.state != ExportState::Running && self.state != ExportState::Fallback {
            return Err(AppError::new(
                "INVALID_EXPORT_STATE",
                "success only allowed from running or fallback",
                None,
            ));
        }
        self.state = ExportState::Success;
        Ok(())
    }

    pub fn fail(&mut self) -> Result<(), AppError> {
        if self.state == ExportState::Success {
            return Err(AppError::new(
                "INVALID_EXPORT_STATE",
                "cannot fail a successful task",
                None,
            ));
        }
        self.state = ExportState::Failed;
        Ok(())
    }
}

impl Default for ExportMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportMachine, ExportState, RecordingMachine, RecordingState};

    #[test]
    fn recording_state_machine_rejects_invalid_pause() {
        let mut machine = RecordingMachine::new();
        let result = machine.pause();
        assert!(result.is_err());
        assert_eq!(machine.state(), RecordingState::Idle);
    }

    #[test]
    fn recording_state_machine_full_flow() {
        let mut machine = RecordingMachine::new();
        machine.start().unwrap();
        machine.pause().unwrap();
        machine.resume().unwrap();
        machine.stop().unwrap();
        assert_eq!(machine.state(), RecordingState::Stopped);
    }

    #[test]
    fn export_state_machine_fallback_then_success() {
        let mut machine = ExportMachine::new();
        machine.start().unwrap();
        machine.fallback().unwrap();
        machine.success().unwrap();
        assert_eq!(machine.state(), ExportState::Success);
    }
}
