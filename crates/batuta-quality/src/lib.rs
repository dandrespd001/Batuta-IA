//! Evidencia y calidad investigada de rutas exactas.

#![forbid(unsafe_code)]

mod hash;
mod model;
mod projection;
mod research;
mod store;

pub use model::{
    ActionProfile, BenchmarkObservation, BenchmarkObservationV1, BenchmarkWeight, OverrideEvent,
    OverrideOperation, QualityError, SourceKind, initial_action_profiles,
};
pub use projection::{
    EvidenceContribution, ExclusionCode, ObservationExclusion, QualityProjection, ScoreRange,
    project,
};
pub use research::{ActiveEvidence, ProposalError, ResearchProposal};
pub use store::{ResearchStatus, ResearchStore};
