//! Explicit connection state machine.
//!
//! Every CableDesk component reasons about connection progress through this
//! single enum instead of scattered boolean flags. The GTK interface renders
//! this state; it does not own networking or streaming logic.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A single lifecycle state of a CableDesk connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConnectionState {
    Unconfigured,
    CheckingCompatibility,
    Unsupported,
    Ready,
    WaitingForCable,
    CableDetected,
    InspectingPower,
    ConfiguringLink,
    DiscoveringPeer,
    PairingRequired,
    Pairing,
    Authenticating,
    PreparingHost,
    StartingSunshine,
    StartingViewer,
    Streaming,
    Suspended,
    Disconnecting,
    Disconnected,
    Recovering,
    Error,
}

impl fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl ConnectionState {
    /// The allowed next states from this state.
    ///
    /// `Error` is reachable from every state (any transition can fail) and is
    /// therefore not listed explicitly in each arm.
    fn allowed_next(self) -> &'static [ConnectionState] {
        use ConnectionState::*;
        match self {
            Unconfigured => &[CheckingCompatibility],
            CheckingCompatibility => &[Unsupported, Ready],
            Unsupported => &[CheckingCompatibility],
            Ready => &[WaitingForCable],
            WaitingForCable => &[CableDetected],
            CableDetected => &[InspectingPower, WaitingForCable],
            InspectingPower => &[ConfiguringLink],
            ConfiguringLink => &[DiscoveringPeer],
            DiscoveringPeer => &[PairingRequired, Authenticating, WaitingForCable],
            PairingRequired => &[Pairing],
            Pairing => &[Authenticating, PairingRequired],
            Authenticating => &[PreparingHost, PairingRequired],
            PreparingHost => &[StartingSunshine],
            StartingSunshine => &[StartingViewer],
            StartingViewer => &[Streaming],
            Streaming => &[Suspended, Disconnecting, WaitingForCable],
            Suspended => &[Streaming, Disconnecting],
            Disconnecting => &[Disconnected],
            Disconnected => &[WaitingForCable, Ready],
            Recovering => &[WaitingForCable, DiscoveringPeer, Disconnected],
            Error => &[Recovering, WaitingForCable, Ready],
        }
    }

    /// Whether `self -> next` is a legal transition.
    ///
    /// Any state may transition to `Error`, and `Error` may only be left via
    /// `Recovering`, `WaitingForCable` or `Ready` (see [`allowed_next`]).
    pub fn can_transition_to(self, next: ConnectionState) -> bool {
        next == ConnectionState::Error || self.allowed_next().contains(&next)
    }
}

/// A validating wrapper around [`ConnectionState`] that rejects illegal
/// transitions instead of silently accepting them.
#[derive(Debug, Clone)]
pub struct StateMachine {
    current: ConnectionState,
    history: Vec<ConnectionState>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("cannot transition from {from} to {to}")]
pub struct TransitionError {
    pub from: ConnectionState,
    pub to: ConnectionState,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            current: ConnectionState::Unconfigured,
            history: Vec::new(),
        }
    }

    pub fn current(&self) -> ConnectionState {
        self.current
    }

    pub fn history(&self) -> &[ConnectionState] {
        &self.history
    }

    /// Attempt to move to `next`. On success, `current` is updated and the
    /// prior state is pushed onto `history`. On failure, state is unchanged.
    pub fn transition(
        &mut self,
        next: ConnectionState,
    ) -> Result<ConnectionState, TransitionError> {
        if !self.current.can_transition_to(next) {
            return Err(TransitionError {
                from: self.current,
                to: next,
            });
        }
        self.history.push(self.current);
        self.current = next;
        Ok(self.current)
    }

    /// Restore a machine at `current` without history (simulation / tests).
    pub fn restored(current: ConnectionState) -> Self {
        Self {
            current,
            history: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ConnectionState::*;

    #[test]
    fn starts_unconfigured() {
        let sm = StateMachine::new();
        assert_eq!(sm.current(), Unconfigured);
        assert!(sm.history().is_empty());
    }

    #[test]
    fn happy_path_reaches_streaming() {
        let mut sm = StateMachine::new();
        let path = [
            CheckingCompatibility,
            Ready,
            WaitingForCable,
            CableDetected,
            InspectingPower,
            ConfiguringLink,
            DiscoveringPeer,
            PairingRequired,
            Pairing,
            Authenticating,
            PreparingHost,
            StartingSunshine,
            StartingViewer,
            Streaming,
        ];
        for state in path {
            sm.transition(state).unwrap_or_else(|e| panic!("{e}"));
        }
        assert_eq!(sm.current(), Streaming);
        assert_eq!(sm.history().len(), path.len());
    }

    #[test]
    fn rejects_illegal_jump() {
        let mut sm = StateMachine::new();
        let err = sm.transition(Streaming).unwrap_err();
        assert_eq!(
            err,
            TransitionError {
                from: Unconfigured,
                to: Streaming
            }
        );
        // Rejected transitions must not mutate state.
        assert_eq!(sm.current(), Unconfigured);
    }

    #[test]
    fn any_state_can_error() {
        for state in [Unconfigured, Ready, Streaming, Pairing, Recovering] {
            assert!(state.can_transition_to(Error));
        }
    }

    #[test]
    fn error_recovers_back_toward_waiting() {
        let mut sm = StateMachine::new();
        sm.transition(CheckingCompatibility).unwrap();
        sm.transition(Ready).unwrap();
        sm.transition(WaitingForCable).unwrap();
        sm.transition(CableDetected).unwrap();
        sm.transition(Error).unwrap();
        assert_eq!(sm.current(), Error);
        sm.transition(Recovering).unwrap();
        sm.transition(WaitingForCable).unwrap();
        assert_eq!(sm.current(), WaitingForCable);
    }

    #[test]
    fn cable_removed_mid_stream_returns_to_waiting_for_cable() {
        let mut sm = StateMachine::new();
        for state in [
            CheckingCompatibility,
            Ready,
            WaitingForCable,
            CableDetected,
            InspectingPower,
            ConfiguringLink,
            DiscoveringPeer,
            PairingRequired,
            Pairing,
            Authenticating,
            PreparingHost,
            StartingSunshine,
            StartingViewer,
            Streaming,
        ] {
            sm.transition(state).unwrap();
        }
        // Cable unplugged mid-stream: must not silently fall back, must
        // return to WaitingForCable per the direct-interface-only rule.
        sm.transition(WaitingForCable).unwrap();
        assert_eq!(sm.current(), WaitingForCable);
    }
}
