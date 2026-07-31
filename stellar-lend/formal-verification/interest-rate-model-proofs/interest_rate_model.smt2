; ============================================================================
; interest_rate_model.smt2 — SMT-LIB 2 specifications for the StellarLend
; two-slope ("kinked") interest rate model
;
; Tool: Z3 >= 4.12  (also compatible with CVC5)
; Run:  z3 interest_rate_model.smt2
;
; Each (check-sat) should return "unsat", proving the negation of the safety
; property is unsatisfiable — i.e., the property holds for ALL inputs in the
; declared ranges.
;
; Model (see contracts/lending-interest/src/lib.rs,
; InterestRateModel::calculate_borrow_rate):
;
;   u <= optimal:  rate(u) = base + (u * slope1) div BPS
;   u >  optimal:  rate(u) = base + (optimal * slope1) div BPS
;                              + ((u - optimal) * slope2) div BPS
;
; We use integer division truncating toward zero, matching Rust's `/` on
; non-negative i128 operands (all quantities here are non-negative bps
; values, so truncating and floor division coincide).
; ============================================================================

(set-logic QF_NIA)

(define-const BPS Int 10000)
(define-const MAX_PARAM Int 1000000000000) ; realistic bound, see lib.rs

(define-fun in_range_util ((u Int)) Bool
  (and (>= u 0) (<= u BPS)))

(define-fun in_range_param ((p Int)) Bool
  (and (>= p 0) (<= p MAX_PARAM)))

; rate(u) as an uninterpreted relation constrained by the two-branch
; definition below (declared once per proof block to keep each (push)/(pop)
; scope self-contained).

; ============================================================================
; 1. Boundary at 0% utilization: rate(0) == base_rate
; ============================================================================
(push)
(declare-const base Int)
(declare-const slope1 Int)
(declare-const slope2 Int)
(declare-const optimal Int)
(assert (in_range_param base))
(assert (in_range_param slope1))
(assert (in_range_param slope2))
(assert (in_range_util optimal))

; rate(0): since 0 <= optimal always, below-kink branch applies.
(declare-const rate0 Int)
(assert (= rate0 (+ base (div (* 0 slope1) BPS))))

; Negation of the property: rate(0) != base
(assert (not (= rate0 base)))
(check-sat) ; expect unsat
(pop)

; ============================================================================
; 2. Monotonicity: u1 <= u2  =>  rate(u1) <= rate(u2)
;
; Encoded directly over the two-branch definition (case split via ite).
; ============================================================================
(push)
(declare-const base Int)
(declare-const slope1 Int)
(declare-const slope2 Int)
(declare-const optimal Int)
(assert (in_range_param base))
(assert (in_range_param slope1))
(assert (in_range_param slope2))
(assert (in_range_util optimal))

(declare-const u1 Int)
(declare-const u2 Int)
(assert (in_range_util u1))
(assert (in_range_util u2))
(assert (<= u1 u2))

(define-fun rate ((u Int)) Int
  (ite (<= u optimal)
       (+ base (div (* u slope1) BPS))
       (+ base (div (* optimal slope1) BPS) (div (* (- u optimal) slope2) BPS))))

(declare-const r1 Int)
(declare-const r2 Int)
(assert (= r1 (rate u1)))
(assert (= r2 (rate u2)))

; Negation of the property: r1 > r2
(assert (> r1 r2))
(check-sat) ; expect unsat
(pop)

; ============================================================================
; 3. Kink continuity: value at u == optimal is single-valued (the two branch
;    definitions agree at the boundary point by construction — this checks
;    that no alternative "above-kink formula evaluated with excess=0" would
;    diverge from the actual below-kink result).
; ============================================================================
(push)
(declare-const base Int)
(declare-const slope1 Int)
(declare-const slope2 Int)
(declare-const optimal Int)
(assert (in_range_param base))
(assert (in_range_param slope1))
(assert (in_range_param slope2))
(assert (in_range_util optimal))

(declare-const below_formula_at_kink Int)
(assert (= below_formula_at_kink (+ base (div (* optimal slope1) BPS))))

(declare-const above_formula_zero_excess Int)
(assert (= above_formula_zero_excess (+ base (div (* optimal slope1) BPS) (div (* 0 slope2) BPS))))

; Negation: the two formulas disagree at the kink.
(assert (not (= below_formula_at_kink above_formula_zero_excess)))
(check-sat) ; expect unsat
(pop)

; ============================================================================
; 4. Bounded above by rate(100%): follows directly from monotonicity (2),
;    instantiated at u2 = BPS.
; ============================================================================
(push)
(declare-const base Int)
(declare-const slope1 Int)
(declare-const slope2 Int)
(declare-const optimal Int)
(assert (in_range_param base))
(assert (in_range_param slope1))
(assert (in_range_param slope2))
(assert (in_range_util optimal))

(declare-const u Int)
(assert (in_range_util u))

(define-fun rate2 ((x Int)) Int
  (ite (<= x optimal)
       (+ base (div (* x slope1) BPS))
       (+ base (div (* optimal slope1) BPS) (div (* (- x optimal) slope2) BPS))))

(declare-const r_u Int)
(declare-const r_max Int)
(assert (= r_u (rate2 u)))
(assert (= r_max (rate2 BPS)))

(assert (> r_u r_max))
(check-sat) ; expect unsat
(pop)
