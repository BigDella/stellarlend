; ============================================================================
; liquidation_math.smt2 — SMT-LIB 2 specifications for StellarLend
; liquidation math (RiskManager::apply_liquidation, contracts/lending-risk)
;
; Tool: Z3 >= 4.12  (also compatible with CVC5)
; Run:  z3 liquidation_math.smt2
;
; Model:
;   bonus            = (repay * bonus_bps) div BPS
;   seized            = repay + bonus
;   new_collateral    = collateral - seized
;   new_debt          = debt - repay
;   health_factor(c,d) = (c * threshold) div d      (d > 0)
;
; Each (check-sat) should return "unsat", proving the negation of the safety
; property is unsatisfiable — i.e., the property holds for ALL inputs in the
; declared ranges.
; ============================================================================

(set-logic QF_NIA)

(define-const BPS Int 10000)
(define-const MAX_VAL Int 1000000000000)

(define-fun in_range_val ((x Int)) Bool
  (and (> x 0) (<= x MAX_VAL)))

(define-fun in_range_bps ((x Int)) Bool
  (and (>= x 0) (<= x BPS)))

; ============================================================================
; 1. Liquidation discount bounds: 0 <= bonus <= repay
; ============================================================================
(push)
(declare-const repay Int)
(declare-const bonus_bps Int)
(assert (in_range_val repay))
(assert (in_range_bps bonus_bps))

(declare-const bonus Int)
(assert (= bonus (div (* repay bonus_bps) BPS)))

(assert (not (and (>= bonus 0) (<= bonus repay))))
(check-sat) ; expect unsat
(pop)

; ============================================================================
; 2. Conservation: new_collateral + seized == collateral, new_debt + repay == debt
; ============================================================================
(push)
(declare-const collateral Int)
(declare-const debt Int)
(declare-const repay Int)
(declare-const bonus_bps Int)
(assert (in_range_val collateral))
(assert (in_range_val debt))
(assert (in_range_bps bonus_bps))
(assert (> repay 0))
(assert (< repay debt))

(declare-const bonus Int)
(assert (= bonus (div (* repay bonus_bps) BPS)))
(declare-const seized Int)
(assert (= seized (+ repay bonus)))
; Only reason about the case where the liquidation is solvent (Ok in Rust).
(assert (<= seized collateral))

(declare-const new_collateral Int)
(declare-const new_debt Int)
(assert (= new_collateral (- collateral seized)))
(assert (= new_debt (- debt repay)))

(assert (not (and (= (+ new_collateral seized) collateral)
                   (= (+ new_debt repay) debt))))
(check-sat) ; expect unsat
(pop)

; ============================================================================
; 3. Ratio condition, direction A:
;    collateral*repay >= debt*seized  =>  hf_after >= hf_before
; ============================================================================
(push)
(declare-const collateral Int)
(declare-const debt Int)
(declare-const repay Int)
(declare-const bonus_bps Int)
(declare-const threshold Int)
(assert (in_range_val collateral))
(assert (in_range_val debt))
(assert (in_range_bps bonus_bps))
(assert (> threshold 0))
(assert (<= threshold BPS))
(assert (> repay 0))
(assert (< repay debt))

(declare-const bonus Int)
(assert (= bonus (div (* repay bonus_bps) BPS)))
(declare-const seized Int)
(assert (= seized (+ repay bonus)))
(assert (<= seized collateral))

(declare-const new_collateral Int)
(declare-const new_debt Int)
(assert (= new_collateral (- collateral seized)))
(assert (= new_debt (- debt repay)))

(declare-const hf_before Int)
(declare-const hf_after Int)
(assert (= hf_before (div (* collateral threshold) debt)))
(assert (= hf_after (div (* new_collateral threshold) new_debt)))

; Premise: the ratio condition holds.
(assert (>= (* collateral repay) (* debt seized)))

; Negation of the conclusion: hf_after < hf_before.
(assert (< hf_after hf_before))
(check-sat) ; expect unsat
(pop)

; ============================================================================
; 4. Ratio condition, direction B:
;    collateral*repay <  debt*seized  =>  hf_after <= hf_before
;    (non-strict: bps-floor-division can land both sides on the same
;    integer at the boundary — see liquidation-math-proofs/src/lib.rs)
; ============================================================================
(push)
(declare-const collateral Int)
(declare-const debt Int)
(declare-const repay Int)
(declare-const bonus_bps Int)
(declare-const threshold Int)
(assert (in_range_val collateral))
(assert (in_range_val debt))
(assert (in_range_bps bonus_bps))
(assert (> threshold 0))
(assert (<= threshold BPS))
(assert (> repay 0))
(assert (< repay debt))

(declare-const bonus Int)
(assert (= bonus (div (* repay bonus_bps) BPS)))
(declare-const seized Int)
(assert (= seized (+ repay bonus)))
(assert (<= seized collateral))

(declare-const new_collateral Int)
(declare-const new_debt Int)
(assert (= new_collateral (- collateral seized)))
(assert (= new_debt (- debt repay)))

(declare-const hf_before Int)
(declare-const hf_after Int)
(assert (= hf_before (div (* collateral threshold) debt)))
(assert (= hf_after (div (* new_collateral threshold) new_debt)))

; Premise: the ratio condition does NOT hold.
(assert (< (* collateral repay) (* debt seized)))

; Negation of the conclusion: hf_after > hf_before.
(assert (> hf_after hf_before))
(check-sat) ; expect unsat
(pop)

; ============================================================================
; 5. Witness: the ratio condition CAN fail for realistic inputs (sat expected
;    — this is a satisfiability witness, not a safety property, confirming
;    the "liquidation can worsen health factor" finding is reachable and not
;    vacuous).
; ============================================================================
(push)
(declare-const collateral Int)
(declare-const debt Int)
(declare-const repay Int)
(declare-const bonus_bps Int)
(assert (= collateral 50000))
(assert (= debt 90000))
(assert (= repay 40000))
(assert (= bonus_bps 500))
(declare-const bonus Int)
(assert (= bonus (div (* repay bonus_bps) BPS)))
(declare-const seized Int)
(assert (= seized (+ repay bonus)))
(assert (< (* collateral repay) (* debt seized))) ; ratio condition fails here
(check-sat) ; expect sat
(pop)
