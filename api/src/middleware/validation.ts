import { body, param, query, validationResult, check } from 'express-validator';
import { Request, Response, NextFunction } from 'express';
import { ValidationError } from '../utils/errors';
import { StrKey } from '@stellar/stellar-sdk';
import {
  LendingOperationDto,
  PrepareRequestDto,
  SubmitRequestDto,
  RelayDelegatedDto,
  CreateSubscriptionDto,
  PaginationQueryDto,
} from '../dto';

const VALID_OPERATIONS = ['deposit', 'borrow', 'repay', 'withdraw'];
const VALID_IMPORT_FORMATS = ['csv', 'json'];
const MAX_XDR_LENGTH = 20000;

export const validateRequest = (req: Request, res: Response, next: NextFunction) => {
  const errors = validationResult(req);
  if (!errors.isEmpty()) {
    const errorMessages = errors
      .array()
      .map((err) => err.msg)
      .join(', ');
    throw new ValidationError(errorMessages);
  }
  next();
};

// ─── DTO-based validation middleware ─────────────────────────────────────────

/** Validates a lending operation (deposit/borrow/repay/withdraw) body via DTO. */
export const validateLendingOperationDto = (req: Request, res: Response, next: NextFunction) => {
  const source = { ...req.query, ...req.body } as Record<string, unknown>;
  const result = LendingOperationDto.validate({
    userAddress: source.userAddress as string,
    amount: source.amount as string,
    assetAddress: source.assetAddress as string | undefined,
  });
  if (!result.isValid) {
    throw new ValidationError(result.toErrorString());
  }
  // Attach typed DTO to request so controllers can use it without re-parsing.
  (req as Request & { dto: LendingOperationDto }).dto = LendingOperationDto.fromBody(source);
  next();
};

/** Validates the prepare endpoint via DTO. */
export const validatePrepareDto = (req: Request, res: Response, next: NextFunction) => {
  const source = { ...req.query, ...req.body, operation: req.params.operation } as Record<
    string,
    unknown
  >;
  const result = PrepareRequestDto.validate(source);
  if (!result.isValid) {
    throw new ValidationError(result.toErrorString());
  }
  (req as Request & { dto: PrepareRequestDto }).dto = PrepareRequestDto.from(
    String(source.operation),
    source,
  );
  next();
};

/** Validates the submit endpoint via DTO. */
export const validateSubmitDto = (req: Request, res: Response, next: NextFunction) => {
  const result = SubmitRequestDto.validate(req.body as Record<string, unknown>);
  if (!result.isValid) {
    throw new ValidationError(result.toErrorString());
  }
  (req as Request & { dto: SubmitRequestDto }).dto = SubmitRequestDto.fromBody(
    req.body as Record<string, unknown>,
  );
  next();
};

/** Validates the relay-delegated endpoint via DTO. */
export const validateRelayDelegatedDto = (req: Request, res: Response, next: NextFunction) => {
  const result = RelayDelegatedDto.validate(req.body as Record<string, unknown>);
  if (!result.isValid) {
    throw new ValidationError(result.toErrorString());
  }
  (req as Request & { dto: RelayDelegatedDto }).dto = RelayDelegatedDto.fromBody(
    req.body as Record<string, unknown>,
  );
  next();
};

/** Validates the create-subscription endpoint via DTO. */
export const validateCreateSubscriptionDto = (
  req: Request,
  res: Response,
  next: NextFunction,
) => {
  const result = CreateSubscriptionDto.validate(req.body as Record<string, unknown>);
  if (!result.isValid) {
    throw new ValidationError(result.toErrorString());
  }
  (req as Request & { dto: CreateSubscriptionDto }).dto = CreateSubscriptionDto.fromBody(
    req.body as Record<string, unknown>,
  );
  next();
};

/** Validates pagination query params via DTO. */
export const validatePaginationDto = (req: Request, res: Response, next: NextFunction) => {
  const maxLimit = parseInt(process.env.PAGINATION_MAX_LIMIT || '100', 10);
  const result = PaginationQueryDto.validate(req.query as Record<string, unknown>, maxLimit);
  if (!result.isValid) {
    throw new ValidationError(result.toErrorString());
  }
  (req as Request & { paginationDto: PaginationQueryDto }).paginationDto =
    PaginationQueryDto.fromQuery(req.query as Record<string, unknown>, maxLimit);
  next();
};

export const amountValidation = [
  check('amount')
    .notEmpty()
    .withMessage('Amount is required')
    .custom((value) => {
      const errMsg = 'Amount must be a valid positive integer';

      try {
        const str = String(value).trim();

        if (!/^\+?\d+$/.test(str)) {
          throw new Error(errMsg);
        }

        const amount = BigInt(str);
        if (amount <= 0n) {
          throw new Error(errMsg);
        }

        const maxI128 = (1n << 127n) - 1n;
        if (amount > maxI128) {
          throw new Error(errMsg);
        }

        return true;
      } catch {
        throw new Error(errMsg);
      }
    }),
];

const createLendingValidation = () => [
  param('operation')
    .isIn(VALID_OPERATIONS)
    .withMessage(`Operation must be one of: ${VALID_OPERATIONS.join(', ')}`),
  check('userAddress')
    .notEmpty()
    .withMessage('User address is required')
    .custom((value) => {
      if (!StrKey.isValidEd25519PublicKey(value)) {
        throw new Error('Invalid Stellar address');
      }
      return true;
    }),
  ...amountValidation,
  check('assetAddress').optional().isString().notEmpty().withMessage('Asset address is required'),
  validateRequest,
];

export const relayDelegatedValidation = [
  body('delegatorAddress')
    .isString()
    .notEmpty()
    .withMessage('delegatorAddress is required')
    .custom((value) => {
      if (!StrKey.isValidEd25519PublicKey(value)) {
        throw new Error('Invalid Stellar address');
      }
      return true;
    }),
  body('nonce').isString().notEmpty().withMessage('nonce is required'),
  body('deadline').isString().notEmpty().withMessage('deadline is required'),
  body('callsXdr')
    .isString()
    .notEmpty()
    .isLength({ max: MAX_XDR_LENGTH })
    .withMessage('callsXdr is required and must be <= 20000 characters'),
  validateRequest,
];

export const prepareValidation = createLendingValidation();

export const submitValidation = [
  body('signedXdr').isString().notEmpty().withMessage('signedXdr is required'),
  body('signedXdr')
    .isString()
    .notEmpty()
    .isLength({ max: MAX_XDR_LENGTH })
    .withMessage('signedXdr is required and must be <= 20000 characters'),
  body('operation')
    .optional()
    .isIn(VALID_OPERATIONS)
    .withMessage(`Operation must be one of: ${VALID_OPERATIONS.join(', ')}`),
  body('userAddress')
    .optional()
    .custom((value) => {
      if (value && !StrKey.isValidEd25519PublicKey(value)) {
        throw new Error('Invalid Stellar address');
      }
      return true;
    }),
  body('amount')
    .optional()
    .custom((value) => {
      if (!value) return true;

      const errMsg = 'Amount must be a valid positive integer';
      try {
        const str = String(value).trim();
        if (!/^\+?\d+$/.test(str)) {
          throw new Error(errMsg);
        }
        const amount = BigInt(str);
        if (amount <= 0n) {
          throw new Error(errMsg);
        }
        const maxI128 = (1n << 127n) - 1n;
        if (amount > maxI128) {
          throw new Error(errMsg);
        }
        return true;
      } catch {
        throw new Error(errMsg);
      }
    }),
  body('assetAddress')
    .optional()
    .isString()
    .notEmpty()
    .withMessage('Asset address must be a string'),
  validateRequest,
];

export const importRequestValidation = [
  body('merchantId').isString().notEmpty().withMessage('merchantId is required'),
  body('format')
    .isIn(VALID_IMPORT_FORMATS)
    .withMessage(`format must be one of: ${VALID_IMPORT_FORMATS.join(', ')}`),
  body('data')
    .custom((value) => typeof value === 'string' || Array.isArray(value))
    .withMessage('data must be a CSV string or JSON array'),
  body('columnMapping')
    .optional()
    .custom((value) => {
      if (typeof value !== 'object' || value === null || Array.isArray(value)) {
        throw new Error('columnMapping must be an object');
      }
      return true;
    }),
  body('options')
    .optional()
    .custom((value) => {
      if (typeof value !== 'object' || value === null || Array.isArray(value)) {
        throw new Error('options must be an object');
      }
      return true;
    }),
  validateRequest,
];

export const merchantParamValidation = [
  param('merchantId').isString().notEmpty().withMessage('merchantId is required'),
  validateRequest,
];

export const paginationValidation = [
  query('limit')
    .optional()
    .isInt({ min: 1, max: parseInt(process.env.PAGINATION_MAX_LIMIT || '100', 10) })
    .withMessage('limit must be a positive integer and at most the configured max'),
  query('cursor')
    .optional()
    .isString()
    .isLength({ max: 256 })
    .notEmpty()
    .withMessage('cursor must be a non-empty string and <= 256 chars'),
  validateRequest,
];

export const depositValidation = createLendingValidation();
export const borrowValidation = createLendingValidation();
export const repayValidation = createLendingValidation();
export const withdrawValidation = createLendingValidation();

export const createRecurringSubscriptionValidation = [
  body('userAddress')
    .isString()
    .notEmpty()
    .withMessage('userAddress is required')
    .custom((value) => {
      if (!StrKey.isValidEd25519PublicKey(value)) {
        throw new Error('Invalid Stellar address');
      }
      return true;
    }),
  body('action')
    .isIn(['deposit', 'borrow', 'repay'])
    .withMessage('action must be one of: deposit, borrow, repay'),
  ...amountValidation,
  body('interval')
    .isIn(['daily', 'weekly', 'monthly', 'quarterly', 'yearly'])
    .withMessage('interval must be one of: daily, weekly, monthly, quarterly, yearly'),
  body('frequency')
    .optional()
    .isInt({ min: 1, max: 365 })
    .withMessage('frequency must be an integer between 1 and 365'),
  body('startDate').optional().isISO8601().withMessage('startDate must be ISO 8601'),
  body('endDate').optional().isISO8601().withMessage('endDate must be ISO 8601'),
  body('assetAddress').optional().isString(),
  body('maxRetries').optional().isInt({ min: 0, max: 100 }),
  validateRequest,
];

export const validatePositionImport = [
  body('format')
    .isIn(VALID_IMPORT_FORMATS)
    .withMessage(`format must be one of: ${VALID_IMPORT_FORMATS.join(', ')}`),
  body('data')
    .custom((value) => typeof value === 'string' || Array.isArray(value))
    .withMessage('data must be a CSV string or JSON array'),
  body('columnMapping')
    .optional()
    .isObject()
    .withMessage('columnMapping must be an object'),
  body('options')
    .optional()
    .isObject()
    .withMessage('options must be an object'),
  body('options.validateOnly')
    .optional()
    .isBoolean()
    .withMessage('validateOnly must be a boolean'),
  body('options.allowUpdates')
    .optional()
    .isBoolean()
    .withMessage('allowUpdates must be a boolean'),
  body('options.previewLimit')
    .optional()
    .isInt({ min: 1, max: 100 })
    .withMessage('previewLimit must be between 1 and 100'),
  body('options.batchSize')
    .optional()
    .isInt({ min: 1, max: 100 })
    .withMessage('batchSize must be between 1 and 100'),
  validateRequest,
];

export const validatePositionExport = [
  body('format')
    .optional()
    .isIn(VALID_IMPORT_FORMATS)
    .withMessage(`format must be one of: ${VALID_IMPORT_FORMATS.join(', ')}`),
  body('includeZeroBalances')
    .optional()
    .isBoolean()
    .withMessage('includeZeroBalances must be a boolean'),
  body('userAddresses')
    .optional()
    .isArray()
    .withMessage('userAddresses must be an array'),
  body('userAddresses.*')
    .optional()
    .custom((value) => {
      if (!StrKey.isValidEd25519PublicKey(value) && !StrKey.isValidContract(value)) {
        throw new Error('Invalid Stellar address');
      }
      return true;
    }),
  body('assetAddress')
    .optional()
    .custom((value) => {
      if (value && !StrKey.isValidContract(value)) {
        throw new Error('Invalid contract address');
      }
      return true;
    }),
  validateRequest,
];
