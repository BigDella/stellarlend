# StellarLend Core Error Codes

The `stellarlend-core` crate exposes a stable `ProtocolError` codebook for
shared contract behavior. Clients should branch on the numeric `code` first and
only use messages for display.

## Codes

| Code | Variant | Meaning | Handling strategy |
| --- | --- | --- | --- |
| 1000 | `Unauthorized` | Unauthorized action | Check signer, auth, or role assignment |
| 1001 | `AlreadyInitialized` | Contract has already been initialized | Treat as idempotent or skip initialization |
| 1002 | `NotInitialized` | Contract state is not initialized | Initialize before retrying |
| 1003 | `InvalidAddress` | Address is invalid or missing | Validate address inputs before submitting |
| 1004 | `InvalidAmount` | Amount is invalid | Require a positive, non-zero amount |
| 1005 | `InvalidConfiguration` | Configuration value is invalid | Refresh config or correct submitted settings |
| 1006 | `InvalidState` | Operation is not valid for the current state | Reload state and retry after the correct lifecycle step |
| 1007 | `InsufficientBalance` | Insufficient balance | Show the balance shortfall to the user |
| 1008 | `InsufficientCollateral` | Insufficient collateral | Ask the user to add collateral or reduce exposure |
| 1009 | `LimitExceeded` | Requested value exceeds an allowed limit | Reduce the request or raise the configured limit |
| 1010 | `ReentrancyDetected` | Reentrancy was detected and blocked | Retry only after the original call completes |
| 1011 | `CrossContractFailure` | Cross-contract invocation failed | Inspect the downstream contract and propagate the wrapped code |
| 1012 | `SerializationFailure` | Failed to serialize or deserialize contract data | Treat as a state integrity issue and fail closed |
| 1013 | `Overflow` | Arithmetic overflow occurred | Use smaller values or safer arithmetic |
| 1014 | `Underflow` | Arithmetic underflow occurred | Use smaller deductions or validate ranges first |
| 1015 | `UnsupportedOperation` | Operation is not supported | Route to a supported entrypoint |
| 1016 | `NotFound` | Requested item was not found | Refresh cached state and retry with the right identifier |
| 1017 | `DeadlineExpired` | Deadline has expired | Submit a new transaction with a fresh deadline |
| 1018 | `DuplicateRequest` | Duplicate request was rejected | Deduplicate before retrying |
| 1999 | `Unknown` | Unknown contract error | Log the numeric code and fail closed |

## Handling Strategy

1. Parse the numeric code first.
2. Use the variant name or message only for logs and UI copy.
3. Do not panic when you encounter an unknown code.
4. Map unknown or malformed values to `Unknown`.
5. For cross-contract calls, preserve the downstream code whenever possible and
   fall back to `CrossContractFailure` if the remote error cannot be decoded.

## Internal Mapping

Internal modules should convert their local failures into `ProtocolError`
through `From<T>` implementations. That keeps the public codebook stable while
allowing lower-level code to stay expressive.
