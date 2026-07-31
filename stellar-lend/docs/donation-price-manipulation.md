# Donation-Based Price Manipulation

Direct token transfers to a lending contract can make the on-chain token balance
larger than the protocol's internal accounting. If pricing, share conversion, or
liquidation logic trusts the raw token balance, an attacker can donate assets to
temporarily inflate pool value, alter share price, and influence liquidation
decisions.

The lending contract protects this surface by separating accounted assets from
observed token balances:

- Deposits update per-asset accounted balances and share supply.
- `sync_donation_balance` compares the token balance against accounted assets
  plus previously quarantined balance.
- New unaccounted balance above the configured tolerance raises a donation alert
  and is quarantined instead of added to share price.
- `get_virtual_share_price_bps` calculates price from accounted assets plus
  virtual assets, excluding quarantined donations.
- Liquidation entrypoints reject collateral assets with an active donation alert
  until an admin acknowledges the event.

Small legitimate balance changes, including airdrops, are handled as quarantined
unaccounted balance. They do not benefit depositors, do not inflate virtual share
price, and can be acknowledged by governance or an operator after review.

Operators should keep `min_deposit_amount` high enough to make dust-based share
rounding attacks uneconomic and tune `max_unaccounted_bps` to match expected
asset transfer noise.
