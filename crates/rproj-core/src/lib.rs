//! Shared, interface-independent services for the rproj CLI and desktop.

#![forbid(unsafe_code)]

pub mod catalog;
pub mod creation;
pub mod desktop_state;
pub mod inspection;
pub mod operations;

pub use inspection::{
    ActionAvailability, Diagnostic, DiagnosticSeverity, FileSnapshot, FileState, PackageDependency,
    PackagePresence, PresenceState, ProjectActions, ProjectFiles, ProjectKind, ProjectSnapshot,
    Realm, ResolutionState, ResolvedPackage, ToolPin, inspect,
};
