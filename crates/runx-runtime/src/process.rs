#[cfg(any(
    feature = "cli-tool",
    feature = "external-adapter",
    feature = "thread-outbox-provider",
    feature = "mcp"
))]
mod signals;

#[cfg(any(
    feature = "cli-tool",
    feature = "external-adapter",
    feature = "thread-outbox-provider"
))]
mod capture;
#[cfg(any(
    feature = "cli-tool",
    feature = "external-adapter",
    feature = "thread-outbox-provider"
))]
mod resource_limits;
#[cfg(any(
    feature = "cli-tool",
    feature = "external-adapter",
    feature = "thread-outbox-provider"
))]
mod spec;
#[cfg(any(
    feature = "cli-tool",
    feature = "external-adapter",
    feature = "thread-outbox-provider"
))]
mod supervisor;
#[cfg(any(
    feature = "cli-tool",
    feature = "external-adapter",
    feature = "thread-outbox-provider"
))]
mod timeout;
#[cfg(feature = "mcp")]
mod tokio_supervisor;

#[cfg(any(feature = "cli-tool", feature = "external-adapter"))]
pub(crate) use self::capture::CapturedOutput;
#[cfg(any(
    feature = "cli-tool",
    feature = "external-adapter",
    feature = "thread-outbox-provider",
    feature = "mcp"
))]
pub(crate) use self::signals::{ProcessSignal, configure_process_group, signal_process_group_id};
#[cfg(any(
    feature = "cli-tool",
    feature = "external-adapter",
    feature = "thread-outbox-provider"
))]
pub(crate) use self::spec::{ProcessOutcome, ProcessSpec, ProcessStdin, ProcessSupervisorError};
#[cfg(any(
    feature = "cli-tool",
    feature = "external-adapter",
    feature = "thread-outbox-provider"
))]
pub(crate) use self::supervisor::run_process;
#[cfg(any(
    feature = "cli-tool",
    feature = "external-adapter",
    feature = "thread-outbox-provider"
))]
use self::supervisor::{kill_timed_out_context, poll_timed_out_context};
#[cfg(feature = "mcp")]
pub(crate) use self::tokio_supervisor::{TokioProcessSpec, spawn_tokio_process};
