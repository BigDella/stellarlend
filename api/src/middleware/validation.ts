import { body, check, param, query, validationResult } from 'express-validator';
import { Request, Response, NextFunction } from 'express';
import { StrKey } from '@stellar/stellar-sdk';
import { ValidationError } from '../utils/errors';

const VALID_OPERATIONS = ['deposit', 'borrow', 'repay', 'withdraw'];
const VALID_IMPORT_FORMATS = ['csv', 'json'];
const MAX_XDR_LENGTH = 20000;
const MAX_ASSET_ID_LENGTH = 128;

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
  check('assetAddress')
    .optional()
    .isString()
    .trim()
    .notEmpty()
    .isLength({ max: MAX_ASSET_ID_LENGTH })
    .withMessage('Asset address must be a non-empty string <= 128 chars'),
  validateRequest,
];

export const prepareValidation = createLendingValidation();

export const submitValidation = [
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
    .trim()
    .notEmpty()
    .isLength({ max: MAX_ASSET_ID_LENGTH })
    .withMessage('Asset address must be a non-empty string <= 128 chars'),
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
