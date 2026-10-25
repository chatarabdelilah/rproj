//! Shared, interface-independent services for the rproj CLI and desktop.

#![forbid(unsafe_code)]

pub mod catalog;
pub mod creation;
pub mod desktop_state;
pub mod inspection;
pub mod operations;
#[doc(hidden)]
pub mod tool_configuration;
#[doc(hidden)]
pub mod upgrade;

pub use inspection::{
    ActionAvailability, Diagnostic, DiagnosticSeverity, FileSnapshot, FileState, PackageDependency,
    PackagePresence, PresenceState, ProjectActions, ProjectFiles, ProjectKind, ProjectSnapshot,
    Realm, ResolutionState, ResolvedPackage, ToolPin, inspect,
};

#[doc(hidden)]
pub mod machine_setup;
