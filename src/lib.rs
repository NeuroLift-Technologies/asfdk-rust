//! ASFDK — Agent Solidarity Framework Dev Kit (Rust port).
//!
//! Governance-aware AI safety primitives — prompt defense, TOI/OTOI
//! validation, emotional-state detection (Sleepwalker), and crisis assessment
//! (RRT Advocate) — orchestrated by [`foundation::NeuroLiftFoundation`].
//!
//! Ported from the canonical reference implementation
//! ([NeuroLift-Technologies/asfdk], Python/TypeScript) with behavior parity
//! against the Go port ([asfdk-go]) and the C# port ([asfdk-csharp]).
//!
//! [NeuroLift-Technologies/asfdk]: https://github.com/NeuroLift-Technologies/asfdk
//! [asfdk-go]: https://github.com/NeuroLift-Technologies/asfdk-go
//! [asfdk-csharp]: https://github.com/NeuroLift-Technologies/asfdk-csharp

pub mod dto;
pub mod foundation;
pub mod promptdefense;
pub mod rrt;
pub mod sleepwalker;
#[cfg(test)]
mod tests;
pub mod types;
