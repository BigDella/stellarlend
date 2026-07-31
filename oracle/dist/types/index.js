/**
 * Oracle Service Type Definitions
 *
 * This module contains all TypeScript interfaces and types used across
 * the Oracle Integration Service for StellarLend protocol.
 */
/**
 * Validation error codes
 */
export var ValidationErrorCode;
(function (ValidationErrorCode) {
    ValidationErrorCode["PRICE_ZERO"] = "PRICE_ZERO";
    ValidationErrorCode["PRICE_NEGATIVE"] = "PRICE_NEGATIVE";
    ValidationErrorCode["PRICE_STALE"] = "PRICE_STALE";
    ValidationErrorCode["PRICE_DEVIATION_TOO_HIGH"] = "PRICE_DEVIATION_TOO_HIGH";
    ValidationErrorCode["INVALID_ASSET"] = "INVALID_ASSET";
    ValidationErrorCode["SOURCE_UNAVAILABLE"] = "SOURCE_UNAVAILABLE";
})(ValidationErrorCode || (ValidationErrorCode = {}));
//# sourceMappingURL=index.js.map