//! Routing puro y relevo eficiente.

#![forbid(unsafe_code)]

mod application;
mod assembly;
mod catalog;
mod catalog_store;
mod coordinator;
mod dsh_sidecar;
mod execution_profile_store;
mod grant;
mod handoff;
mod health;
mod health_store;
mod ledger;
mod operational;
mod policy;
mod policy_migration;
mod routing_receipt;
mod run_receipt_v2;
mod run_state;
mod run_store;
mod selector;
mod serial_executor;
mod snapshot_store;
mod state_store;

pub use application::{
    ApplicationService, RouteRequestEnvelopeV2, RouteRequestV2, RoutingSnapshot,
};
pub use assembly::{
    AssemblyDiscard, AssemblyError, AssemblyReport, CapabilityIndexEntryV2, CapabilityIndexV2,
    CatalogRouteStateV2, CatalogStateV2, EvidenceStateV2, HealthStateV2, PolicyRouteStateV2,
    PolicyStateV2, assemble_snapshot,
};
pub use catalog::{
    Catalog, CatalogClass, CatalogImportError, CatalogImportReport, CatalogRejection, CatalogRoute,
    CostComponents, DshCatalogBridge,
};
pub use catalog_store::{CatalogProposal, CatalogStatus, CatalogStore, CatalogStoreError};
pub use coordinator::{
    RunAttemptV2, RunClock, RunCoordinator, RunCoordinatorError, RunJournalEventV2,
    RunJournalKindV2, RunNextActionV2, RunPhaseV2, RunRequestV2, RunSleeper, RunStatusV2,
};
pub use dsh_sidecar::{DshSidecarClient, DshSidecarError};
pub use execution_profile_store::{
    EMPTY_EXECUTION_PROFILE_HASH, ExecutionProfileProposalV1, ExecutionProfileStatusV1,
    ExecutionProfileStore, ExecutionProfileStoreError,
};
pub use grant::{
    ExecutionGrantDraftV1, ExecutionGrantV1, GrantError, GrantLimits, GrantOperation, GrantStatus,
    GrantStore, Revocation,
};
pub use handoff::{HandoffCheckpoint, HandoffDraft, HandoffError, TestFact, TestStatus};
pub use health::{
    FailureCategory, HealthObservationV2, HealthOutcomeV2, HealthTransition, RecoveryAction,
    RouteHealth, record_failure,
};
pub use health_store::{HealthStore, HealthStoreError};
pub use ledger::{BudgetAmount, BudgetError, LedgerStatus, LedgerStore, Reservation};
pub use operational::{
    CanaryEffectsV2, CanaryScenarioV2, CapabilityCanaryReceiptV2, OperationalError,
    ResearchProposalV2, ResearchSourceV2, ToolEventV2,
};
pub use policy::{
    AliasCatalog, AliasError, ExecutionPolicyV2, LegacyModelPolicy, MigrationSettings, PolicyError,
    RoutingPolicy,
};
pub use policy_migration::{PolicyMigration, PolicyMigrationError, PolicyMigrationOutcome};
pub use routing_receipt::{
    RoutingReceipt, RoutingReceiptError, RoutingReceiptStore, RoutingTransition,
};
pub use run_receipt_v2::{
    RunCandidateReceiptV2, RunConsumptionReceiptV2, RunDecisionReceiptV2, RunReceiptDraftV2,
    RunReceiptError, RunReceiptReferenceV2, RunReceiptStoreV2, RunReceiptV2, RunReservationKindV2,
    RunReservationReceiptV2, RunResultReceiptV2,
};
pub use run_state::{RunEvent, RunState, RunTransitionError, advance_run};
pub use run_store::{RunStateStore, RunStoreError};
pub use selector::{
    AuthorizationDecision, DecisionSealV2, DiscardReason, DiscardedRoute, RouteCandidate,
    RouteClass, RouteDecision, RouteRequest, RouteRequestDraft, RoutingActionProfile, SelectError,
    SelectionAuthorizations, SelectionMargin, select, select_sealed,
};
pub use serial_executor::{ExecutionBusy, SerialExecutionGate};
pub use snapshot_store::{RoutingSnapshotStore, SnapshotStoreError};
pub use state_store::{
    StateComponentsV2, StateManifestV2, StateSnapshotV2, StateStore, StateStoreError,
};
