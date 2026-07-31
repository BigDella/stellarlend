export enum ProtocolErrorCode {
  Unauthorized = 1000,
  AlreadyInitialized = 1001,
  NotInitialized = 1002,
  InvalidAddress = 1003,
  InvalidAmount = 1004,
  InvalidConfiguration = 1005,
  InvalidState = 1006,
  InsufficientBalance = 1007,
  InsufficientCollateral = 1008,
  LimitExceeded = 1009,
  ReentrancyDetected = 1010,
  CrossContractFailure = 1011,
  SerializationFailure = 1012,
  Overflow = 1013,
  Underflow = 1014,
  UnsupportedOperation = 1015,
  NotFound = 1016,
  DeadlineExpired = 1017,
  DuplicateRequest = 1018,
  Unknown = 1999,
}

export interface ProtocolErrorInfo {
  code: ProtocolErrorCode;
  name: string;
  message: string;
  handlingStrategy: string;
  retryable: boolean;
}

export const PROTOCOL_ERROR_INFO: Record<ProtocolErrorCode, ProtocolErrorInfo> = {
  [ProtocolErrorCode.Unauthorized]: {
    code: ProtocolErrorCode.Unauthorized,
    name: 'Unauthorized',
    message: 'Unauthorized action',
    handlingStrategy: 'Check signer, auth flow, or role assignment.',
    retryable: false,
  },
  [ProtocolErrorCode.AlreadyInitialized]: {
    code: ProtocolErrorCode.AlreadyInitialized,
    name: 'AlreadyInitialized',
    message: 'Contract has already been initialized',
    handlingStrategy: 'Treat the request as idempotent or skip initialization.',
    retryable: false,
  },
  [ProtocolErrorCode.NotInitialized]: {
    code: ProtocolErrorCode.NotInitialized,
    name: 'NotInitialized',
    message: 'Contract state is not initialized',
    handlingStrategy: 'Initialize the contract before retrying.',
    retryable: false,
  },
  [ProtocolErrorCode.InvalidAddress]: {
    code: ProtocolErrorCode.InvalidAddress,
    name: 'InvalidAddress',
    message: 'Address is invalid or missing',
    handlingStrategy: 'Validate address inputs before submitting.',
    retryable: false,
  },
  [ProtocolErrorCode.InvalidAmount]: {
    code: ProtocolErrorCode.InvalidAmount,
    name: 'InvalidAmount',
    message: 'Amount is invalid',
    handlingStrategy: 'Require a positive, non-zero amount.',
    retryable: false,
  },
  [ProtocolErrorCode.InvalidConfiguration]: {
    code: ProtocolErrorCode.InvalidConfiguration,
    name: 'InvalidConfiguration',
    message: 'Configuration value is invalid',
    handlingStrategy: 'Refresh config or correct submitted settings.',
    retryable: false,
  },
  [ProtocolErrorCode.InvalidState]: {
    code: ProtocolErrorCode.InvalidState,
    name: 'InvalidState',
    message: 'Operation is not valid for the current state',
    handlingStrategy: 'Reload state and retry after the correct lifecycle step.',
    retryable: false,
  },
  [ProtocolErrorCode.InsufficientBalance]: {
    code: ProtocolErrorCode.InsufficientBalance,
    name: 'InsufficientBalance',
    message: 'Insufficient balance',
    handlingStrategy: 'Show the balance shortfall to the user.',
    retryable: false,
  },
  [ProtocolErrorCode.InsufficientCollateral]: {
    code: ProtocolErrorCode.InsufficientCollateral,
    name: 'InsufficientCollateral',
    message: 'Insufficient collateral',
    handlingStrategy: 'Ask the user to add collateral or reduce exposure.',
    retryable: false,
  },
  [ProtocolErrorCode.LimitExceeded]: {
    code: ProtocolErrorCode.LimitExceeded,
    name: 'LimitExceeded',
    message: 'Requested value exceeds an allowed limit',
    handlingStrategy: 'Reduce the request or raise the configured limit.',
    retryable: false,
  },
  [ProtocolErrorCode.ReentrancyDetected]: {
    code: ProtocolErrorCode.ReentrancyDetected,
    name: 'ReentrancyDetected',
    message: 'Reentrancy was detected and blocked',
    handlingStrategy: 'Retry only after the original call completes.',
    retryable: true,
  },
  [ProtocolErrorCode.CrossContractFailure]: {
    code: ProtocolErrorCode.CrossContractFailure,
    name: 'CrossContractFailure',
    message: 'Cross-contract invocation failed',
    handlingStrategy: 'Inspect the downstream contract and preserve the wrapped code when possible.',
    retryable: true,
  },
  [ProtocolErrorCode.SerializationFailure]: {
    code: ProtocolErrorCode.SerializationFailure,
    name: 'SerializationFailure',
    message: 'Failed to serialize or deserialize contract data',
    handlingStrategy: 'Treat as a state integrity issue and fail closed.',
    retryable: false,
  },
  [ProtocolErrorCode.Overflow]: {
    code: ProtocolErrorCode.Overflow,
    name: 'Overflow',
    message: 'Arithmetic overflow occurred',
    handlingStrategy: 'Use smaller values or safer arithmetic.',
    retryable: false,
  },
  [ProtocolErrorCode.Underflow]: {
    code: ProtocolErrorCode.Underflow,
    name: 'Underflow',
    message: 'Arithmetic underflow occurred',
    handlingStrategy: 'Use smaller deductions or validate ranges first.',
    retryable: false,
  },
  [ProtocolErrorCode.UnsupportedOperation]: {
    code: ProtocolErrorCode.UnsupportedOperation,
    name: 'UnsupportedOperation',
    message: 'Operation is not supported',
    handlingStrategy: 'Route to a supported entrypoint.',
    retryable: false,
  },
  [ProtocolErrorCode.NotFound]: {
    code: ProtocolErrorCode.NotFound,
    name: 'NotFound',
    message: 'Requested item was not found',
    handlingStrategy: 'Refresh cached state and retry with the right identifier.',
    retryable: false,
  },
  [ProtocolErrorCode.DeadlineExpired]: {
    code: ProtocolErrorCode.DeadlineExpired,
    name: 'DeadlineExpired',
    message: 'Deadline has expired',
    handlingStrategy: 'Submit a new transaction with a fresh deadline.',
    retryable: true,
  },
  [ProtocolErrorCode.DuplicateRequest]: {
    code: ProtocolErrorCode.DuplicateRequest,
    name: 'DuplicateRequest',
    message: 'Duplicate request was rejected',
    handlingStrategy: 'Deduplicate the request before retrying.',
    retryable: false,
  },
  [ProtocolErrorCode.Unknown]: {
    code: ProtocolErrorCode.Unknown,
    name: 'Unknown',
    message: 'An unknown contract error occurred',
    handlingStrategy: 'Log the numeric code and fail closed without panicking.',
    retryable: false,
  },
};

export class ProtocolError extends Error {
  public readonly name = 'ProtocolError';

  constructor(
    public readonly code: ProtocolErrorCode,
    message = getProtocolErrorMessage(code),
    public readonly details?: unknown
  ) {
    super(message);
    Object.setPrototypeOf(this, ProtocolError.prototype);
  }

  get retryable(): boolean {
    return getProtocolErrorInfo(this.code).retryable;
  }

  get handlingStrategy(): string {
    return getProtocolErrorInfo(this.code).handlingStrategy;
  }
}

export function isProtocolErrorCode(code: number): code is ProtocolErrorCode {
  return Object.prototype.hasOwnProperty.call(PROTOCOL_ERROR_INFO, code);
}

export function getProtocolErrorInfo(code: ProtocolErrorCode | number): ProtocolErrorInfo {
  if (isProtocolErrorCode(code)) {
    return PROTOCOL_ERROR_INFO[code];
  }

  return PROTOCOL_ERROR_INFO[ProtocolErrorCode.Unknown];
}

export function getProtocolErrorMessage(code: ProtocolErrorCode | number): string {
  return getProtocolErrorInfo(code).message;
}

function extractNumber(value: unknown): number | undefined {
  if (typeof value === 'number' && Number.isInteger(value)) {
    return value;
  }

  if (typeof value === 'string') {
    const trimmed = value.trim();
    if (/^\d+$/.test(trimmed)) {
      return Number(trimmed);
    }

    const sorobanMatch = trimmed.match(/Error\s*\(\s*Contract\s*,\s*#?(\d+)\s*\)/i);
    if (sorobanMatch) {
      return Number(sorobanMatch[1]);
    }

    const hashMatch = trimmed.match(/#(\d{3,5})/);
    if (hashMatch) {
      return Number(hashMatch[1]);
    }

    const match = trimmed.match(/(?:code|errorCode|error_code)\D*(\d{3,5})/i);
    if (match) {
      return Number(match[1]);
    }
  }

  if (typeof value === 'object' && value !== null) {
    const candidate = value as {
      code?: unknown;
      errorCode?: unknown;
      error_code?: unknown;
      statusCode?: unknown;
      message?: unknown;
    };

    return (
      extractNumber(candidate.code) ??
      extractNumber(candidate.errorCode) ??
      extractNumber(candidate.error_code) ??
      extractNumber(candidate.statusCode) ??
      extractNumber(candidate.message)
    );
  }

  return undefined;
}

export function normalizeProtocolError(error: unknown): ProtocolError {
  if (error instanceof ProtocolError) {
    return error;
  }

  const code = extractNumber(error);
  if (typeof code === 'number' && isProtocolErrorCode(code)) {
    return new ProtocolError(code, getProtocolErrorMessage(code), error);
  }

  if (typeof error === 'string') {
    const matchedCode = extractNumber(error);
    if (typeof matchedCode === 'number') {
      return new ProtocolError(
        isProtocolErrorCode(matchedCode) ? matchedCode : ProtocolErrorCode.Unknown,
        getProtocolErrorMessage(matchedCode),
        error
      );
    }
  }

  return new ProtocolError(ProtocolErrorCode.Unknown, getProtocolErrorMessage(ProtocolErrorCode.Unknown), error);
}

export function protocolErrorToJson(error: ProtocolError): {
  code: ProtocolErrorCode;
  name: string;
  message: string;
  handlingStrategy: string;
  retryable: boolean;
  details?: unknown;
} {
  const info = getProtocolErrorInfo(error.code);
  return {
    code: info.code,
    name: info.name,
    message: info.message,
    handlingStrategy: info.handlingStrategy,
    retryable: info.retryable,
    details: error.details,
  };
}
