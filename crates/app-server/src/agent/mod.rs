//! Agent loop module.
//!
//! M3 owns the LLM agent loop. The orchestrator drives a multi-round
//! conversation: build context -> stream from the LLM -> dispatch tool-calls
//! through the M2 validator/executor -> inject results -> repeat.
//!
//! Submodules:
//! - [`orchestrator`]: the agent loop driver and `AgentEvent` channel API.
//! - [`tool_executor`]: dispatches validated tool-calls and runs the side-effects.
//! - [`context_builder`]: assembles the system prompt for each round.
//! - [`tools`]: tool catalog (`all_tools()`) exposed to the LLM.

pub mod context_builder;
pub mod orchestrator;
pub mod tool_executor;
pub mod tools;
