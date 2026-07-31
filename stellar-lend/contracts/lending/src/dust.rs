/// Returns true when `amount` is a positive value below the configured
/// minimum transaction size.
#[inline]
pub(crate) fn is_dust_amount(amount: i128, min_amount: i128) -> bool {
    min_amount > 0 && amount > 0 && amount < min_amount
}
