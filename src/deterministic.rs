use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Deterministic timestamp generator for Machine-HTTP
/// Provides synchronized timestamping across requests and sessions
pub struct DeterministicTimestamp {
    /// Base timestamp in milliseconds since epoch
    base_ms: AtomicU64,
    /// Counter for deterministic increments within the same millisecond
    counter: AtomicU64,
    /// Whether to use real time or synthetic time
    use_synthetic_time: bool,
}

impl DeterministicTimestamp {
    /// Create a new deterministic timestamp generator
    pub fn new(use_synthetic_time: bool) -> Self {
        let base_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        DeterministicTimestamp {
            base_ms: AtomicU64::new(base_ms),
            counter: AtomicU64::new(0),
            use_synthetic_time,
        }
    }

    /// Create with a specific base timestamp (for testing or replay)
    pub fn with_base_time(base_ms: u64) -> Self {
        DeterministicTimestamp {
            base_ms: AtomicU64::new(base_ms),
            counter: AtomicU64::new(0),
            use_synthetic_time: true,
        }
    }

    /// Get the next deterministic timestamp
    pub fn next(&self) -> u64 {
        if self.use_synthetic_time {
            // Use synthetic time with counter
            self.base_ms.load(Ordering::Relaxed) + self.counter.fetch_add(1, Ordering::Relaxed)
        } else {
            // Use real time but ensure monotonicity
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            
            // Ensure monotonicity by updating base_ms if needed
            let mut current_base = self.base_ms.load(Ordering::Relaxed);
            while now_ms > current_base {
                if let Ok(_) = self.base_ms.compare_exchange(
                    current_base,
                    now_ms,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    current_base = now_ms;
                } else {
                    // If compare_exchange failed, reload current_base and try again
                    current_base = self.base_ms.load(Ordering::Relaxed);
                }
            }
            
            current_base + self.counter.fetch_add(1, Ordering::Relaxed)
        }
    }

    /// Synchronize timestamp with a remote source
    pub fn sync_with_remote(&mut self, remote_ms: u64) {
        self.base_ms.store(remote_ms, Ordering::Relaxed);
        self.counter.store(0, Ordering::Relaxed);
    }
}

/// Deterministic random number generator for Machine-HTTP
/// Provides reproducible randomness based on a seed
pub struct DeterministicRng {
    seed: u64,
    state: u64,
    counter: u64,
}

impl DeterministicRng {
    /// Create a new deterministic RNG with a specific seed
    pub fn new(seed: u64) -> Self {
        DeterministicRng {
            seed,
            state: seed ^ 0xDEADBEEFDEADBEEF,
            counter: 0,
        }
    }

    /// Reset RNG to its initial state
    pub fn reset(&mut self) {
        self.state = self.seed ^ 0xDEADBEEFDEADBEEF;
        self.counter = 0;
    }

    /// Get the next random u64
    pub fn next_u64(&mut self) -> u64 {
        // Simple xorshift64+ algorithm for deterministic randomness
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.counter += 1;
        self.state
    }

    /// Get the next random f64 in [0, 1)
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Get the current seed
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Get the current counter (number of random numbers generated)
    pub fn counter(&self) -> u64 {
        self.counter
    }
}

/// Interface for deterministic JavaScript execution environment
/// Provides controlled execution of JavaScript code with reproducible results
pub trait DeterministicJsEnv {
    /// Initialize the environment with specific settings
    fn init(
        &mut self,
        timestamp: u64,
        rng_seed: u64,
        allow_network: bool,
        allow_dom_access: bool,
    ) -> Result<(), JsEnvError>;

    /// Execute JavaScript code in a deterministic manner
    fn execute(&mut self, code: &str, context: Option<&str>) -> Result<JsExecutionResult, JsEnvError>;

    /// Reset the environment to its initial state
    fn reset(&mut self) -> Result<(), JsEnvError>;

    /// Get the current execution state for debugging/replay
    fn get_execution_state(&self) -> JsExecutionState;
}

/// JavaScript execution result
pub enum JsExecutionResult {
    /// String result
    String(String),
    /// Number result
    Number(f64),
    /// Boolean result
    Boolean(bool),
    /// Object result (serialized as JSON)
    Object(String),
    /// Array result (serialized as JSON)
    Array(String),
    /// Null result
    Null,
    /// Undefined result
    Undefined,
}

/// JavaScript execution state for reproducibility
#[derive(Debug, Clone)]
pub struct JsExecutionState {
    pub timestamp: u64,
    pub rng_seed: u64,
    pub rng_counter: u64,
    pub execution_count: u64,
    pub heap_size: usize,
}

/// JavaScript environment errors
#[derive(Debug)]
pub enum JsEnvError {
    SyntaxError(String),
    RuntimeError(String),
    SecurityViolation(String),
    Timeout,
    InternalError(String),
}

impl std::error::Error for JsEnvError {
    fn description(&self) -> &str {
        match self {
            JsEnvError::SyntaxError(_) => "Syntax error",
            JsEnvError::RuntimeError(_) => "Runtime error",
            JsEnvError::SecurityViolation(_) => "Security violation",
            JsEnvError::Timeout => "Timeout exceeded",
            JsEnvError::InternalError(_) => "Internal error",
        }
    }
}

impl std::fmt::Display for JsEnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsEnvError::SyntaxError(msg) => write!(f, "Syntax error: {}", msg),
            JsEnvError::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
            JsEnvError::SecurityViolation(msg) => write!(f, "Security violation: {}", msg),
            JsEnvError::Timeout => write!(f, "Timeout exceeded"),
            JsEnvError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Machine-HTTP deterministic control parameters
#[derive(Debug, Clone)]
pub struct DeterministicControlParams {
    pub timestamp: u64,
    pub rng_seed: u64,
    pub rng_counter: u64,
    pub js_execution_state: Option<JsExecutionState>,
    pub allow_network: bool,
    pub allow_dom_access: bool,
}

impl Default for DeterministicControlParams {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        DeterministicControlParams {
            timestamp: now,
            rng_seed: now, // Default seed based on timestamp
            rng_counter: 0,
            js_execution_state: None,
            allow_network: true,
            allow_dom_access: true,
        }
    }
}
